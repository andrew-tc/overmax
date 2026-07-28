use crate::capture::frame_utils::ImageRegion;
use overmax_core::SceneType;
use std::fmt;

#[derive(Clone, Default, PartialEq)]
pub struct OcrTelemetry {
    pub rate_text: String,
    pub threshold: u8,
    pub bg_mean: f32,
    pub use_invert: bool,
    pub image_pixels: Vec<u8>,
    pub image_width: usize,
    pub image_height: usize,
}

impl fmt::Debug for OcrTelemetry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OcrTelemetry")
            .field("rate_text", &self.rate_text)
            .field("threshold", &self.threshold)
            .field("bg_mean", &self.bg_mean)
            .field("use_invert", &self.use_invert)
            .field("image_pixels_len", &self.image_pixels.len())
            .field("image_width", &self.image_width)
            .field("image_height", &self.image_height)
            .finish()
    }
}

pub struct OcrDetector;

impl Default for OcrDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl OcrDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn is_available(&self) -> bool {
        true
    }

    /// 상단 로고 영역 감지 (Windows OCR 제거로 인해 더 이상 로고 OCR을 수행하지 않습니다)
    pub fn detect_logo(&self, _logo: &ImageRegion) -> (SceneType, String, String) {
        (SceneType::Unknown, String::new(), "UNKNOWN".to_string())
    }

    /// Rate 영역을 Pure Rust CV 템플릿 매칭으로 감지합니다.
    pub fn detect_rate(&self, rate: &ImageRegion) -> (Option<f32>, String, Option<OcrTelemetry>) {
        let cv_templates = get_digit_templates();
        let matched = match match_digits_template(rate, &cv_templates) {
            Ok(m) => m,
            Err(_) => return (None, String::new(), None),
        };
        let (matched_str, binary, threshold, max_y) = matched;

        // 템플릿 매칭 결과에서 ?를 제거하고 파싱 시도
        let rate_val = (!matched_str.is_empty())
            .then(|| matched_str.replace('?', ""))
            .and_then(|clean_str| parse_rate_text(&clean_str));

        if let Some(val) = rate_val {
            let telemetry = OcrTelemetry {
                rate_text: matched_str.clone(),
                threshold,
                bg_mean: max_y as f32,
                use_invert: false,
                image_pixels: binary,
                image_width: rate.width as usize,
                image_height: rate.height as usize,
            };
            (Some(val), matched_str, Some(telemetry))
        } else {
            (None, String::new(), None)
        }
    }

    /// Score 영역을 템플릿 매칭을 통해 정수로 파싱합니다.
    pub fn detect_score(&self, score: &ImageRegion) -> Option<u32> {
        let cv_templates = get_digit_templates();
        match match_digits_template(score, &cv_templates) {
            Ok((matched_str, _, _, _)) => {
                let parsed = parse_score_text(&matched_str);
                if parsed.is_none() || matched_str.contains('?') {
                    println!(
                        "      [Debug Score] Template matching failed/invalid. Matched String: '{}', Parsed: {:?}",
                        matched_str, parsed
                    );
                    None
                } else {
                    parsed
                }
            }
            Err(e) => {
                println!(
                    "      [Debug Score] match_digits_template failed with error: {}",
                    e
                );
                None
            }
        }
    }

    /// 결과창 뱃지 이미지로부터 모드(4B~8B)와 난이도(NM~SC)를 감지합니다.
    /// Freestyle 결과창 모드 영역을 템플릿 매칭으로 판독합니다.
    pub fn detect_freestyle_mode(&self, mode_img: &ImageRegion) -> Option<String> {
        let w = mode_img.width as usize;
        let h = mode_img.height as usize;
        if w * h == 0 {
            return None;
        }

        let (binary, _, _) = match overmax_cv::binarize_by_global_contrast(
            &mode_img.bgra,
            w,
            h,
            overmax_cv::LumaMethod::Average,
            1,
        ) {
            Ok(b) => b,
            Err(_) => return None,
        };
        // [가드레일] 50x68 해상도(3400px)에서 전경색 픽셀이 20px(약 0.6%) 미만인 경우,
        // 유효한 텍스트 뼈대가 실존하지 않는 빈 화면(Blank/Black)으로 규정하여
        // 템플릿 오인식(False Positive)을 차단하고 즉시 None을 반환합니다.
        let fg_count = binary.iter().filter(|&&x| x == 1).count();
        if fg_count < 20 {
            return None;
        }
        let (target_w, target_h) = (50usize, 68usize);
        let resized_binary = resize_binary(&binary, w, h, target_w, target_h);

        let t_infos: Vec<MatchTemplateInfo> =
            crate::detector::templates::result_mode::RESULT_MODE_TEMPLATES
                .iter()
                .map(|t| MatchTemplateInfo {
                    width: t.width,
                    height: t.height,
                    mask: t.mask,
                    label: t.mode_label,
                })
                .collect();

        match_best_template(&resized_binary, target_w, target_h, &t_infos, 0.75, |_| 0)
    }

    /// 결과 화면 전용 난이도 패널 영역을 템플릿 매칭으로 감지합니다.
    pub fn detect_result_difficulty(&self, diff_img: &ImageRegion) -> Option<String> {
        let w = diff_img.width as usize;
        let h = diff_img.height as usize;
        if w * h == 0 {
            return None;
        }

        let (binary, _, _) = match overmax_cv::binarize_by_global_contrast(
            &diff_img.bgra,
            w,
            h,
            overmax_cv::LumaMethod::Average,
            1,
        ) {
            Ok(b) => b,
            Err(_) => return None,
        };
        // [가드레일] 90x18 해상도(1620px)에서 전경색 픽셀이 10px(약 0.6%) 미만인 경우,
        // 유효한 텍스트 뼈대가 실존하지 않는 빈 화면(Blank/Black)으로 규정하여
        // 템플릿 오인식(False Positive)을 차단하고 즉시 None을 반환합니다.
        let fg_count = binary.iter().filter(|&&x| x == 1).count();
        if fg_count < 10 {
            return None;
        }
        let (target_w, target_h) = (90usize, 18usize);
        let resized_binary = resize_binary(&binary, w, h, target_w, target_h);

        let t_infos: Vec<MatchTemplateInfo> =
            crate::detector::templates::result_diff::RESULT_DIFF_TEMPLATES
                .iter()
                .map(|t| MatchTemplateInfo {
                    width: t.width,
                    height: t.height,
                    mask: t.mask,
                    label: t.name,
                })
                .collect();

        match_best_template(&resized_binary, target_w, target_h, &t_infos, 0.80, |_| 0)
    }

    /// 오픈매치 결과 화면 전용 난이도 영역을 템플릿 매칭으로 감지합니다. (106x18 해상도 적용)
    pub fn detect_openmatch_result_difficulty(&self, diff_img: &ImageRegion) -> Option<String> {
        let w = diff_img.width as usize;
        let h = diff_img.height as usize;
        if w * h == 0 {
            return None;
        }

        let binary = overmax_cv::adaptive_threshold_bradley_roth(
            &diff_img.bgra,
            w,
            h,
            overmax_cv::LumaMethod::Average,
            80,
            0.03,
            1,
        );
        let (target_w, target_h) = (106usize, 18usize);
        let resized_binary = resize_binary(&binary, w, h, target_w, target_h);

        let t_infos: Vec<MatchTemplateInfo> =
            crate::detector::templates::result_diff::RESULT_DIFF_OPEN_TEMPLATES
                .iter()
                .map(|t| MatchTemplateInfo {
                    width: t.width,
                    height: t.height,
                    mask: t.mask,
                    label: t.name,
                })
                .collect();

        match_best_template(
            &resized_binary,
            target_w,
            target_h,
            &t_infos,
            0.80,
            |label| match label {
                "NM" => 15,
                "HD" => 35,
                "MX" => 0,
                "SC" => 55,
                _ => 0,
            },
        )
    }

    pub fn recognize_text_color(&self, _region: &ImageRegion) -> Option<String> {
        None
    }

    pub fn recognize_text_binarized(
        &self,
        _region: &ImageRegion,
        _force_invert: bool,
    ) -> Option<String> {
        None
    }

    /// 텍스트 내에 유효한 버튼 모드 키워드가 포함되어 있는지 판단합니다.
    pub fn contains_mode_keyword(&self, text: &str) -> bool {
        let norm = text.to_lowercase();
        norm.contains("4b") || norm.contains("5b") || norm.contains("6b") || norm.contains("8b")
    }

    /// 텍스트에서 매칭되는 버튼 모드를 문자열로 파싱합니다.
    pub fn parse_mode_from_text(&self, text: &str) -> Option<String> {
        let norm = text.to_lowercase();
        if norm.contains("4b") || norm.contains('4') {
            Some("4B".to_string())
        } else if norm.contains("5b") || norm.contains('5') {
            Some("5B".to_string())
        } else if norm.contains("6b") || norm.contains('6') {
            Some("6B".to_string())
        } else if norm.contains("8b") || norm.contains('8') {
            Some("8B".to_string())
        } else {
            None
        }
    }

    pub fn recognize_text_all_passes(&self, _region: &ImageRegion) -> Option<String> {
        None
    }
}

#[allow(dead_code)]
fn match_logo_scene(text: &str) -> Option<(SceneType, String)> {
    let normalized = normalize_alnum(text).to_lowercase();
    if normalized.contains("buttontunes") || normalized.contains("button") {
        Some((SceneType::ResultFreestyle, normalized))
    } else if normalized.contains("freestyle") {
        Some((SceneType::Freestyle, normalized))
    } else if normalized.contains("online") {
        if normalized.contains("open") || normalized.contains("openmatch") {
            Some((SceneType::OpenMatch, normalized))
        } else if normalized.contains("ladder") || normalized.contains("laddermatch") {
            Some((SceneType::LadderMatch, normalized))
        } else {
            Some((SceneType::Online, normalized))
        }
    } else if normalized.contains("tunes") || normalized.contains("tune") {
        let has_number = normalized.chars().any(|c| c.is_ascii_digit());
        if has_number {
            Some((SceneType::ResultOpen2, normalized))
        } else {
            None
        }
    } else {
        None
    }
}

#[allow(dead_code)]
fn scene_label(scene: SceneType) -> String {
    match scene {
        SceneType::Freestyle => "FREESTYLE".to_string(),
        SceneType::Online => "ONLINE".to_string(),
        SceneType::OpenMatch => "OPEN_MATCH".to_string(),
        SceneType::LadderMatch => "LADDER_MATCH".to_string(),
        SceneType::ResultFreestyle => "RESULT_FREESTYLE".to_string(),
        SceneType::ResultOpen3 => "RESULT_OPEN3".to_string(),
        SceneType::ResultOpen2 => "RESULT_OPEN2".to_string(),
        _ => "UNKNOWN".to_string(),
    }
}

fn match_digits_template(
    img: &ImageRegion,
    cv_templates: &[overmax_cv::CvTemplate],
) -> Result<(String, Vec<u8>, u8, u8), String> {
    let w = img.width as usize;
    let h = img.height as usize;

    // 1. 고휘도 이진화 전처리
    let (binary, threshold, max_y) = overmax_cv::binarize_by_global_contrast(
        &img.bgra,
        w,
        h,
        overmax_cv::LumaMethod::Average,
        255,
    )
    .map_err(|e| e.to_string())?;

    // 2. 수직 투영 분할
    let segments = overmax_cv::segment_characters(&binary, w, h).map_err(|e| e.to_string())?;

    // 3. 템플릿 매칭 판독
    let mut matched_str = String::new();
    for &(x1, x2) in &segments {
        let char_w = x2 - x1;
        let char_h = h;
        let mut char_bin = vec![0u8; char_w * char_h];
        for y in 0..char_h {
            for x in 0..char_w {
                char_bin[y * char_w + x] = binary[y * w + (x1 + x)];
            }
        }

        if let Ok(Some((ch, _score))) =
            overmax_cv::match_character(&char_bin, char_w, char_h, cv_templates)
        {
            if ch.is_ascii_digit() || ch == '.' || ch == '%' {
                matched_str.push(ch);
            }
        } else {
            matched_str.push('?');
        }
    }

    Ok((matched_str, binary, threshold, max_y))
}

fn resize_binary(binary: &[u8], w: usize, h: usize, target_w: usize, target_h: usize) -> Vec<u8> {
    if w == target_w && h == target_h {
        return binary.to_vec();
    }
    let mut dst = vec![0u8; target_w * target_h];
    for dy in 0..target_h {
        let sy = (dy * h) / target_h;
        let sy_clamped = sy.min(h - 1);
        for dx in 0..target_w {
            let sx = (dx * w) / target_w;
            let sx_clamped = sx.min(w - 1);
            dst[dy * target_w + dx] = binary[sy_clamped * w + sx_clamped];
        }
    }
    dst
}

fn get_digit_templates() -> Vec<overmax_cv::CvTemplate<'static>> {
    crate::detector::templates::digit::DIGIT_TEMPLATES
        .iter()
        .map(|t| overmax_cv::CvTemplate {
            char_val: t.char_val,
            width: t.width,
            height: t.height,
            mask: t.mask,
        })
        .collect()
}

struct MatchTemplateInfo<'a> {
    width: usize,
    height: usize,
    mask: &'a [u8],
    label: &'a str,
}

fn match_best_template(
    resized_binary: &[u8],
    target_w: usize,
    target_h: usize,
    templates: &[MatchTemplateInfo],
    min_score: f32,
    safe_x_calc: impl Fn(&str) -> usize,
) -> Option<String> {
    let mut best_score = 0.0f32;
    let mut best_label: Option<String> = None;
    let compare_total = target_w * target_h;

    for t in templates {
        if t.width != target_w || t.height != target_h {
            continue;
        }
        let safe_x = safe_x_calc(t.label);
        let mut matches = 0usize;
        for dy in 0..target_h {
            for dx in 0..target_w {
                let i = dy * target_w + dx;
                if dx < safe_x || resized_binary[i] == t.mask[i] {
                    matches += 1;
                }
            }
        }
        let score = matches as f32 / compare_total as f32;
        if score > min_score && score > best_score {
            best_score = score;
            best_label = Some(t.label.to_string());
        }
    }
    if best_label.is_none() {
        let mut max_candidate_score = 0.0f32;
        let mut max_candidate_label = "None";
        for t in templates {
            if t.width != target_w || t.height != target_h {
                continue;
            }
            let safe_x = safe_x_calc(t.label);
            let mut matches = 0usize;
            for dy in 0..target_h {
                for dx in 0..target_w {
                    let i = dy * target_w + dx;
                    if dx < safe_x || resized_binary[i] == t.mask[i] {
                        matches += 1;
                    }
                }
            }
            let score = matches as f32 / compare_total as f32;
            if score > max_candidate_score {
                max_candidate_score = score;
                max_candidate_label = t.label;
            }
        }
        println!("      [Debug Template] Matching failed. Best candidate: '{}' with score {:.4} (min_score: {:.4})", max_candidate_label, max_candidate_score, min_score);
    }
    best_label
}

fn parse_score_text(text: &str) -> Option<u32> {
    let clean = text
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>();
    if clean.len() != 6 && clean.len() != 7 {
        return None;
    }
    clean.parse::<u32>().ok()
}

fn parse_rate_text(text: &str) -> Option<f32> {
    let mut cleaned = String::new();
    let mut dot_seen = false;
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            cleaned.push(ch);
        } else if ch == '.' && !dot_seen {
            cleaned.push(ch);
            dot_seen = true;
        }
    }
    let mut value = cleaned.parse::<f32>().ok()?;

    // Windows OCR 오인식 대응:
    // "94.12%"를 "9412%"와 같이 소수점(.)을 누락하여 인식하는 경우가 존재합니다.
    // DJMAX RESPECT V의 Rate는 항상 소수점 둘째 자리까지 표기되므로,
    // 문자열에 소수점이 감지되지 않았고 파싱 결과가 MIN_VALID_RATE(80.0%) 이상인 경우
    // 소수점 이하 2자리가 정수로 취급되었다고 가정하고 100.0으로 나누어 보정합니다.
    // 이를 통해 0.00%가 900% 노이즈로 튈 때 9.00% 등으로 오보정되어 통과하는 부작용을 예방합니다.
    if !dot_seen && value >= (crate::detector::play_state::MIN_VALID_RATE * 100.0) {
        value /= 100.0;
    }

    // 소수점 셋째 자리 이하 무조건 버림(Truncate) 보정 적용하여 반올림 차단
    value = (value * 100.0).floor() / 100.0;

    // 유효한 실시간 기록으로 처리할 수 있는 최소 범위(MIN_VALID_RATE = 80.0%) 이상인 경우만 유효값으로 반환하고,
    // 0.00%가 4.00% 또는 9.00% 노이즈로 완벽하게 잘못 오인식되는 수치 등은 스캔 시점에 원천 배제합니다.
    (crate::detector::play_state::MIN_VALID_RATE..=100.0)
        .contains(&value)
        .then_some(value)
}

#[allow(dead_code)]
fn normalize_alnum(text: &str) -> String {
    text.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_uppercase)
        .collect()
}

#[allow(dead_code)]
fn is_logo_keyword_match(keyword: &str, normalized_ocr: &str) -> bool {
    if keyword.is_empty() || normalized_ocr.is_empty() {
        return false;
    }
    if normalized_ocr.contains(keyword) {
        return true;
    }

    let min_partial_len = keyword.len().min(6);
    for idx in 0..=keyword.len().saturating_sub(min_partial_len) {
        if normalized_ocr.contains(&keyword[idx..idx + min_partial_len]) {
            return true;
        }
    }
    sequence_ratio(keyword, normalized_ocr) >= 0.72
}

fn sequence_ratio(left: &str, right: &str) -> f32 {
    let lcs = lcs_len(left.as_bytes(), right.as_bytes()) as f32;
    2.0 * lcs / (left.len() + right.len()) as f32
}

fn lcs_len(left: &[u8], right: &[u8]) -> usize {
    let mut prev = vec![0; right.len() + 1];
    let mut curr = vec![0; right.len() + 1];
    for &a in left {
        for (idx, &b) in right.iter().enumerate() {
            curr[idx + 1] = if a == b {
                prev[idx] + 1
            } else {
                curr[idx].max(prev[idx + 1])
            };
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[right.len()]
}

#[cfg(test)]
mod tests {
    use super::{is_logo_keyword_match, normalize_alnum, parse_rate_text, parse_score_text};

    #[test]
    fn parses_score_text_correctly() {
        assert_eq!(parse_score_text("999,800"), Some(999800));
        assert_eq!(parse_score_text("1,000,000"), Some(1000000));
        assert_eq!(parse_score_text("abc"), None);
    }

    #[test]
    fn parses_rate_text_like_python_path() {
        assert_eq!(parse_rate_text("99.43%"), Some(99.43));
        assert_eq!(parse_rate_text("100.00"), Some(100.0));
        assert_eq!(parse_rate_text("101.0"), None);
        // 소수점 누락 보정 테스트
        assert_eq!(parse_rate_text("9412%"), Some(94.12));
        assert_eq!(parse_rate_text("10000"), Some(100.0));
        // 소수점 셋째 자리 버림(Truncate) 보정 테스트
        assert_eq!(parse_rate_text("99.289%"), Some(99.28));
        assert_eq!(parse_rate_text("99.281"), Some(99.28));
        assert_eq!(parse_rate_text("99.280"), Some(99.28));
    }

    #[test]
    fn normalizes_logo_text_to_alnum_uppercase() {
        assert_eq!(normalize_alnum("free style!"), "FREESTYLE");
    }

    #[test]
    fn matches_logo_keyword_by_substring_partial_or_ratio() {
        assert!(is_logo_keyword_match("FREESTYLE", "DJMAXFREESTYLE"));
        assert!(is_logo_keyword_match("FREESTYLE", "FREEST"));
        assert!(is_logo_keyword_match("FREESTYLE", "FREESTY1E"));
        assert!(is_logo_keyword_match("ONLINE", "DJMAXONLINE"));
        assert!(is_logo_keyword_match("ONLINE", "ONL1NE"));
        assert!(!is_logo_keyword_match("FREESTYLE", "MISSION"));
    }
}
