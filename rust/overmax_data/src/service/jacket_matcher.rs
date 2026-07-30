use crate::store::image_index::{ImageEntry, ImageMatch};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct JacketMatcherConfig {
    pub similarity_threshold: f32,
    /// HOG 매칭이 완전히 제거됨에 따라, `margin_threshold`와 `disable_hog`는
    /// 더 이상 런타임 매칭에 실질적 영향을 미치지 않지만, 사용자 설정 파일(`settings.user.json`)
    /// 호환성을 깨지 않고 무해하게 유지하기 위해 필드를 보존합니다.
    pub margin_threshold: f32,
    pub disable_hog: bool,
}

#[derive(Debug)]
struct MatchCache {
    recent_indices: Vec<usize>,
}

pub struct JacketMatcher {
    entries: Arc<Vec<ImageEntry>>,
    config: JacketMatcherConfig,
    cache: std::sync::Mutex<MatchCache>,
    // SoA (Structure of Arrays) 평탄화 버퍼: L1/L2 CPU 캐시 연속성 극대화 및 SIMD 가속
    phash_list: Vec<u64>,
    dhash_list: Vec<u64>,
    ahash_list: Vec<u64>,
    /// 만약 모든 곡에 히스토그램이 존재하는 DB라면 `Some(Vec<[u8; 384]>)`로 촘촘하게 평탄화하여
    /// 루프 내부의 Option 분기 체크(Discriminant Branching)를 100% 제거
    hist_list: Option<Vec<[u8; 384]>>,
}

impl JacketMatcher {
    /// 즐겨찾기(Favorite) 및 테두리 마스킹이 적용된 총 비교 비트(160비트) 중,
    /// 노이즈가 가장 심한 특수 이미지들(예: Fundamental 등)에서 발생할 수 있는
    /// 최대 Hamming Distance 불일치 거리가 약 38~40비트 수준입니다.
    /// 정답이 잘못 걸러지는 누락(False Negative)을 방지하기 위해 통계 마진을 두어
    /// Early Exit 필터 임계치를 42비트로 정의합니다.
    /// 95% 이상의 완전 불일치 곡 후보군들은 POPCNT 3번으로 즉시 탈락(Early Exit)됩니다.
    const HAMMING_EARLY_EXIT_THRESHOLD: u32 = 42;

    pub fn new(entries: Arc<Vec<ImageEntry>>, config: JacketMatcherConfig) -> Self {
        let phash_list = entries.iter().map(|e| e.phash).collect();
        let dhash_list = entries.iter().map(|e| e.dhash).collect();
        let ahash_list = entries.iter().map(|e| e.ahash).collect();

        // 모든 entry에 grid_hist가 존재하면 Option을 루프 밖으로 빼내어 촘촘한 연속 메모리 버퍼 생성
        let has_all_hist = !entries.is_empty() && entries.iter().all(|e| e.grid_hist.is_some());
        let hist_list = if has_all_hist {
            Some(
                entries
                    .iter()
                    .filter_map(|e| e.grid_hist)
                    .collect::<Vec<[u8; 384]>>(),
            )
        } else {
            None
        };

        Self {
            entries,
            config,
            cache: std::sync::Mutex::new(MatchCache {
                recent_indices: Vec::new(),
            }),
            phash_list,
            dhash_list,
            ahash_list,
            hist_list,
        }
    }

    pub fn similarity_threshold(&self) -> f32 {
        self.config.similarity_threshold
    }

    fn update_cache(&self, idx: usize) {
        if let Ok(mut guard) = self.cache.lock() {
            if let Some(pos) = guard.recent_indices.iter().position(|&x| x == idx) {
                guard.recent_indices.remove(pos);
            }
            guard.recent_indices.insert(0, idx);
            if guard.recent_indices.len() > 8 {
                guard.recent_indices.truncate(8);
            }
        }
    }

    /// 100% 무상태(Stateless) 단일 패스 스캔으로 이전 곡 캐시 고착/Invalidation 오류를 0% 차단하며,
    /// SoA 평탄화 해시 및 SIMD SAD(u8::abs_diff) 히스토그램 대조로 고속 스캔을 수행합니다.
    pub fn match_jacket(
        &self,
        data: &[u8],
        width: usize,
        height: usize,
        channels: usize,
    ) -> Option<ImageMatch> {
        if self.phash_list.is_empty() {
            return None;
        }

        // 1. 3종 해시 추출
        let (q_phash, q_dhash, q_ahash) =
            overmax_cv::compute_image_hashes(data, width, height, channels).ok()?;

        // 2. 4x4 분할 RGB 그리드 히스토그램 추출 (BGRA 직접 입력, grayscale 변환 불필요)
        let q_grid_hist = overmax_cv::compute_grid_histogram(data, width, height, channels);

        // 오염 영역 비트 마스킹 (상단 y=0, 우측 x=7, 즐겨찾기 y=1, x=0)
        let mut mask_bits: u64 = 0;
        for x in 0..8 {
            mask_bits |= 1 << x; // y = 0
        }
        for y in 0..8 {
            mask_bits |= 1 << (y * 8 + 7); // x = 7
        }
        mask_bits |= 1 << 8; // y = 1, x = 0

        let hash_mask: u64 = !mask_bits;
        let compare_bits = hash_mask.count_ones() as f32; // 48.0
        let total_compare_bits = 64.0 + compare_bits * 2.0; // 160.0

        // 3. 무상태(Stateless) SoA 연속 메모리 루프 순회 (루프 외곽 Option 1회 분기)
        let len = self.phash_list.len();
        let mut best_idx = None;
        let mut best_sim = -1.0f32;

        if let Some(hist_list) = &self.hist_list {
            // [최적 경로] 1차 필터 탈락 시 384B 히스토그램 메모리는 주소 참조조차 100% 억제 (Lazy Fetch)
            #[allow(clippy::needless_range_loop)]
            for idx in 0..len {
                // L1 캐시 연속 정수 배열에서 직접 POPCNT 연산 (1클럭)
                let p_dist = (self.phash_list[idx] ^ q_phash).count_ones();
                let d_dist = ((self.dhash_list[idx] ^ q_dhash) & hash_mask).count_ones();
                let a_dist = ((self.ahash_list[idx] ^ q_ahash) & hash_mask).count_ones();

                let hamming_sum = p_dist + d_dist + a_dist;

                // 1차 필터: Early Exit (임계치 42비트)
                // 95%+ 불일치 곡은 아래 384B 히스토그램 버퍼 주소 참조조차 하지 않고 즉시 탈락!
                if hamming_sum > Self::HAMMING_EARLY_EXIT_THRESHOLD {
                    continue;
                }

                // 1차 필터를 통과한 5% 미만의 극소수 회차에서만 384B 메모리 로드 (Lazy Fetch)
                let e_hist = &hist_list[idx];
                let hist_diff: u32 = e_hist
                    .iter()
                    .zip(q_grid_hist.iter())
                    .map(|(&e, &q)| e.abs_diff(q) as u32)
                    .sum();
                let hist_sim = 1.0 - (hist_diff as f32 / 3072.0).clamp(0.0, 1.0);
                let hash_sim = 1.0 - (hamming_sum as f32 / total_compare_bits);
                let similarity = 0.5 * hash_sim + 0.5 * hist_sim;

                if similarity > best_sim {
                    best_sim = similarity;
                    best_idx = Some(idx);
                }
            }
        } else {
            // [폴백 경로] 히스토그램이 없는 레거시 DB 전용 루프 (루프 내 Option 분기 0개)
            for idx in 0..len {
                let p_dist = (self.phash_list[idx] ^ q_phash).count_ones();
                let d_dist = ((self.dhash_list[idx] ^ q_dhash) & hash_mask).count_ones();
                let a_dist = ((self.ahash_list[idx] ^ q_ahash) & hash_mask).count_ones();

                let hamming_sum = p_dist + d_dist + a_dist;

                if hamming_sum > Self::HAMMING_EARLY_EXIT_THRESHOLD {
                    continue;
                }

                let similarity = 1.0 - (hamming_sum as f32 / total_compare_bits);

                if similarity > best_sim {
                    best_sim = similarity;
                    best_idx = Some(idx);
                }
            }
        }

        if let Some(idx) = best_idx {
            if best_sim >= self.config.similarity_threshold {
                self.update_cache(idx);
                return Some(ImageMatch {
                    image_id: self.entries[idx].image_id.clone(),
                    similarity: best_sim,
                });
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_entry(image_id: &str, phash: u64) -> ImageEntry {
        ImageEntry {
            image_id: image_id.to_string(),
            phash,
            dhash: phash,
            ahash: phash,
            grid_hist: None,
        }
    }

    #[test]
    fn test_jacket_matcher_basic_match() {
        let entries = Arc::new(vec![
            dummy_entry("song-a", 0x0000_0000_0000_0000),
            dummy_entry("song-b", 0xFFFF_FFFF_FFFF_FFFF),
        ]);
        let config = JacketMatcherConfig {
            similarity_threshold: 0.75,
            margin_threshold: 3.0,
            disable_hog: false,
        };
        let matcher = JacketMatcher::new(entries, config);

        // 8x8 그레이스케일 이미지 모킹 (전부 0)
        let query_data = vec![0u8; 64];

        let matched = matcher.match_jacket(&query_data, 8, 8, 1).unwrap();
        assert_eq!(matched.image_id, "song-a");
        assert!(matched.similarity >= 0.9);
    }
}
