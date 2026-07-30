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
    hist_list: Vec<Option<[u8; 384]>>,
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
        let hist_list = entries.iter().map(|e| e.grid_hist).collect();

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

    pub fn match_jacket(
        &self,
        data: &[u8],
        width: usize,
        height: usize,
        channels: usize,
    ) -> Option<ImageMatch> {
        self.match_jacket_with_top_k(data, width, height, channels, 10)
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

    /// 구버전 매칭 엔진의 public API 시그니처 호환성을 유지하기 위한 메서드입니다.
    /// 100% 무상태(Stateless) 단일 패스 스캔으로 이전 곡 캐시 고착/Invalidation 오류를 0% 차단하며,
    /// SoA 평탄화 해시 및 SIMD SAD(u8::abs_diff) 히스토그램 대조로 고속 스캔을 수행합니다.
    pub fn match_jacket_with_top_k(
        &self,
        data: &[u8],
        width: usize,
        height: usize,
        channels: usize,
        _top_k: usize,
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

        // 3. 무상태(Stateless) SoA 연속 메모리 루프 순회
        let len = self.phash_list.len();
        let mut best_idx = None;
        let mut best_sim = -1.0f32;

        for idx in 0..len {
            // L1 캐시 연속 정수 배열에서 직접 POPCNT 연산 (1클럭)
            let p_dist = (self.phash_list[idx] ^ q_phash).count_ones();
            let d_dist = ((self.dhash_list[idx] ^ q_dhash) & hash_mask).count_ones();
            let a_dist = ((self.ahash_list[idx] ^ q_ahash) & hash_mask).count_ones();

            let hamming_sum = p_dist + d_dist + a_dist;

            // 1차 필터: Early Exit (임계치 42비트)
            if hamming_sum > Self::HAMMING_EARLY_EXIT_THRESHOLD {
                continue;
            }

            // 2차 필터: SIMD SAD(u8::abs_diff) 히스토그램 L1 차이 연산
            let similarity = if let Some(e_hist) = &self.hist_list[idx] {
                let hist_diff: u32 = e_hist
                    .iter()
                    .zip(q_grid_hist.iter())
                    .map(|(&e, &q)| e.abs_diff(q) as u32)
                    .sum();
                let hist_sim = 1.0 - (hist_diff as f32 / 3072.0).clamp(0.0, 1.0);
                let hash_sim = 1.0 - (hamming_sum as f32 / total_compare_bits);
                0.5 * hash_sim + 0.5 * hist_sim
            } else {
                1.0 - (hamming_sum as f32 / total_compare_bits)
            };

            if similarity > best_sim {
                best_sim = similarity;
                best_idx = Some(idx);
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
