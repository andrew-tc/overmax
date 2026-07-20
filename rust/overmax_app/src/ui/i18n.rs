//! Minimal hand-rolled i18n: Korean is the source-of-truth string and also the
//! lookup key, so call sites just wrap the existing literal in `t(...)`.

use serde_json::Value;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Locale {
    #[default]
    Ko,
    En,
}

static CURRENT_LOCALE: AtomicU8 = AtomicU8::new(0);

pub fn set_locale(locale: Locale) {
    let code = match locale {
        Locale::Ko => 0,
        Locale::En => 1,
    };
    CURRENT_LOCALE.store(code, Ordering::Relaxed);
}

pub fn current_locale() -> Locale {
    match CURRENT_LOCALE.load(Ordering::Relaxed) {
        1 => Locale::En,
        _ => Locale::Ko,
    }
}

/// Reads the top-level `"language"` key (`"ko"`/`"en"`) from merged settings JSON.
pub fn set_locale_from_settings(settings: &Value) {
    let locale = match settings.get("language").and_then(Value::as_str) {
        Some("en") => Locale::En,
        _ => Locale::Ko,
    };
    set_locale(locale);
}

/// Looks up `ko` (the source string, also the key) in the current locale.
/// Returns `ko` unchanged for `Locale::Ko`, or if no English mapping exists.
pub fn t(ko: &'static str) -> &'static str {
    if current_locale() == Locale::Ko {
        return ko;
    }
    translate_en(ko)
}

fn translate_en(ko: &'static str) -> &'static str {
    match ko {
        // settings_ui.rs
        "설정" => "Settings",
        "오버레이 설정" => "Overlay Settings",
        "크기" => "Size",
        "투명도" => "Opacity",
        "라이트모드" => "Lite Mode",
        "활성화" => "Enable",
        "추천 숨기기 및 레이아웃 축소" => "Hide recommendations and shrink layout",
        "오버레이 고정 위치" => "Overlay Snap Position",
        "좌상단" => "Top-Left",
        "우상단" => "Top-Right",
        "좌하단" => "Bottom-Left",
        "우하단" => "Bottom-Right",
        "수동" => "Manual",
        "V-Archive 계정" => "V-Archive Account",
        "연동 상태" => "Link Status",
        "자동화" => "Automation",
        "시작 시 자동 갱신" => "Auto-refresh on start",
        "데이터 동기화" => "Data Sync",
        "찾기" => "Browse",
        "🔍 동기화 후보 찾기" => "🔍 Find Sync Candidates",
        "업데이트 설정" => "Update Settings",
        "자동 업데이트" => "Auto Update",
        "사용" => "Enable",
        "버전 정보" => "Version",
        "발견된 Steam 계정이 없습니다." => "No Steam account found.",
        "일반" => "General",
        "언어" => "Language",
        "현재 Steam: " => "Current Steam: ",
        "현재 Steam: -" => "Current Steam: -",

        // sync_ui.rs
        "동기화" => "Sync",
        "Steam 계정 기준으로 업로드 후보를 확인합니다." => "Checks upload candidates for the current Steam account.",
        "스캔" => "Scan",
        "업로드 후보" => "Upload Candidates",
        "변경순" => "By Change",
        "제목순" => "By Title",
        "V-Archive 동기화" => "V-Archive Sync",
        "등록" => "Register",
        "삭제" => "Delete",

        // native_app_viewports.rs
        "Overmax 설정" => "Overmax Settings",
        "닫기" => "Close",
        "저장" => "Save",

        // tray_icon.rs
        "디버그 로그" => "Debug Log",
        "종료" => "Exit",

        // overlay_ui.rs
        "V-Archive 업로드 필요 (클릭하여 즉시 업로드)" => "V-Archive upload needed (click to upload now)",
        "V-Archive 계정 연동 필요 (설정에서 account.txt 경로를 지정해주세요)" => {
            "V-Archive account link needed (set account.txt path in Settings)"
        }
        "유사 구간 평균" => "Similar Section Average",
        "키파트 위주 패턴" => "Key-part Focused Pattern",
        "황배:" => "Gold:",
        "보조:" => "Assist:",
        "기록 없음" => "No Record",

        // overlay_recommend_ui.rs
        "패턴을 감지하는 중..." => "Detecting pattern...",
        "추천 결과 없음" => "No recommendations",
        "개 패턴" => " patterns",

        // debug_ui.rs
        "▶ 재개" => "▶ Resume",
        "⏸ 일시정지" => "⏸ Pause",
        "🗑 지우기" => "🗑 Clear",
        "필터:" => "Filter:",

        // native_app_recommend.rs
        "곡을 선택하세요" => "Please select a song",

        // native_app.rs
        "스캔 중…" => "Scanning…",
        "account.txt 경로 없음" => "account.txt path not set",
        "account.txt 파싱 실패" => "Failed to parse account.txt",
        "갱신 완료" => "Updated",
        "등록 완료" => "Registered",
        "V-Archive 실패: " => "V-Archive Failed: ",
        "API 조회 OK, 캐시 병합 실패: " => "API fetch OK, cache merge failed: ",
        "API 실패 (" => "API failed (",
        "), 폴백 캐시 갱신 실패: " => "), fallback cache update failed: ",
        "위 달성!)" => " achieved!)",
        "업로드 OK, 캐시 갱신 오류: " => "Upload OK, cache update error: ",

        // community/sync.rs (SyncCandidate::reason_label, translated via injected fn)
        "미등록" => "Not Registered",

        _ => ko,
    }
}
