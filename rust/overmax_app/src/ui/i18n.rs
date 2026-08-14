//! Zero-cost static i18n lookup system with compile-time embedded key-value resources.

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

/// Zero-cost, zero-reallocation macro-based i18n dispatcher.
/// Supports language-explicit zero-replace formatting and static lookup delegation.
#[macro_export]
macro_rules! t {
    // 1) 0-Cost Language Selector Helper
    (@select Ko => $ko:expr, En => $en:expr) => {
        match $crate::ui::i18n::current_locale() {
            $crate::ui::i18n::Locale::Ko => $ko,
            $crate::ui::i18n::Locale::En => $en,
        }
    };

    // 2) Gold / Assist Domain Meta Direct Matcher (No wrapper functions)
    (gold = $meta:expr) => {
        match $meta {
            overmax_data::community::sheet_meta::GoldMeta::HalfRandom => $crate::t!("gold-half-random"),
            overmax_data::community::sheet_meta::GoldMeta::MaxRandom => $crate::t!("gold-max-random"),
            overmax_data::community::sheet_meta::GoldMeta::Random => $crate::t!("gold-random"),
            overmax_data::community::sheet_meta::GoldMeta::None => "",
        }
    };
    (assist = $meta:expr) => {
        match $meta {
            overmax_data::community::sheet_meta::AssistMeta::Used => $crate::t!("assist-used"),
            overmax_data::community::sheet_meta::AssistMeta::Caution => $crate::t!("assist-caution"),
            overmax_data::community::sheet_meta::AssistMeta::NotUsed => $crate::t!("assist-not-used"),
            overmax_data::community::sheet_meta::AssistMeta::None => "",
        }
    };

    // 3) Parameterized Format Keys (Zero-Replace, Zero-Realloc Direct format! Bindings)
    ("candidate-count", n = $n:expr) => {
        $crate::t!(@select
            Ko => format!("후보 {}건", $n),
            En => format!("{} candidates", $n)
        )
    };
    ("sys-place-achieved", mode = $mode:expr, rank = $rank:expr) => {
        $crate::t!(@select
            Ko => format!("{} TOP {}위 달성!", $mode, $rank),
            En => format!("Achieved TOP {} in {}!", $rank, $mode)
        )
    };
    ("sys-upload-cache-error", error = $err:expr) => {
        $crate::t!(@select
            Ko => format!("업로드 OK, 캐시 갱신 오류: {}", $err),
            En => format!("Upload OK, cache error: {}", $err)
        )
    };
    ("sys-update-prompt-dialog", current = $current:expr, latest = $latest:expr) => {
        $crate::t!(@select
            Ko => format!("새 앱 업데이트가 있습니다.\n\n현재 버전: {}\n최신 버전: {}\n\n지금 업데이트를 진행할까요?", $current, $latest),
            En => format!("A new app update is available.\n\nCurrent version: {}\nLatest version: {}\n\nWould you like to update now?", $current, $latest)
        )
    };
    ("sys-update-error-dialog", error = $err:expr) => {
        $crate::t!(@select
            Ko => format!("자동 패치가 완료되지 않았습니다.\n\n사유: {}", $err),
            En => format!("The automatic update did not complete.\n\nReason: {}", $err)
        )
    };
    ("sys-api-ok-cache-failed", error = $err:expr) => {
        $crate::t!(@select
            Ko => format!("API 조회 OK, 캐시 병합 실패: {}", $err),
            En => format!("API fetch OK, cache merge failed: {}", $err)
        )
    };
    ("sys-api-and-fallback-failed", api_error = $api_err:expr, fallback_error = $fb_err:expr) => {
        $crate::t!(@select
            Ko => format!("API 실패 ({}), 폴백 캐시 갱신 실패: {}", $api_err, $fb_err),
            En => format!("API failed ({}), fallback cache update failed: {}", $api_err, $fb_err)
        )
    };
    ("sys-fallback-cache-failed", error = $err:expr) => {
        $crate::t!(@select
            Ko => format!("폴백 캐시 갱신 실패: {}", $err),
            En => format!("Fallback cache update failed: {}", $err)
        )
    };
    ("settings-find-sync-candidates-btn") => {
        $crate::t!(@select
            Ko => "🔍 동기화 후보 찾기".to_string(),
            En => "🔍 Find Sync Candidates".to_string()
        )
    };
    ("settings-capture-method-win") => {
        $crate::t!(@select
            Ko => "화면 캡처 설정 (Windows)".to_string(),
            En => "Screen Capture Method (Windows)".to_string()
        )
    };
    ("settings-capture-mode-dxgi") => {
        $crate::t!(@select
            Ko => "DXGI (실험적 / 고성능)".to_string(),
            En => "DXGI (Experimental / High Perf)".to_string()
        )
    };
    ("settings-capture-mode-gdi") => {
        $crate::t!(@select
            Ko => "GDI (기본값 / 안정적)".to_string(),
            En => "GDI (Default / Stable)".to_string()
        )
    };
    ("settings-eg-hint", domain = $domain:expr) => {
        $crate::t!(@select
            Ko => format!("예: {}", $domain),
            En => format!("e.g. {}", $domain)
        )
    };
    ("sync-filter-btn", icon = $icon:expr) => {
        $crate::t!(@select
            Ko => format!("{} 🔍 필터", $icon),
            En => format!("{} 🔍 Filter", $icon)
        )
    };
    ("sync-reset-btn") => {
        $crate::t!(@select
            Ko => "초기화 ↺".to_string(),
            En => "Reset ↺".to_string()
        )
    };
    ("sync-level-range", min = $min:expr, max = $max:expr) => {
        $crate::t!(@select
            Ko => format!("난이도 ({} ~ {})", $min, $max),
            En => format!("Level ({} ~ {})", $min, $max)
        )
    };
    ("rec-patterns-count", has = $has:expr, total = $total:expr) => {
        $crate::t!(@select
            Ko => format!("{}/{}곡", $has, $total),
            En => format!("{}/{} patterns", $has, $total)
        )
    };
    ("overlay-gold-rec-meta", val = $val:expr) => {
        $crate::t!(@select
            Ko => format!("황배:{}", $val),
            En => format!("Recommendation:{}", $val)
        )
    };
    ("overlay-assist-key-meta", val = $val:expr) => {
        $crate::t!(@select
            Ko => format!("보조:{}", $val),
            En => format!("Assist Key:{}", $val)
        )
    };
    ("status-varchive-failed-toast", error = $err:expr) => {
        $crate::t!(@select
            Ko => format!("V-Archive 동기화 실패: {}", $err),
            En => format!("V-Archive sync failed: {}", $err)
        )
    };
    ("sys-shortcut-create-success", path = $path:expr) => {
        $crate::t!(@select
            Ko => format!("앱 메뉴 바로가기를 생성했습니다.\n\n{}", $path),
            En => format!("App menu shortcut created successfully.\n\n{}", $path)
        )
    };
    ("sys-shortcut-create-failed", error = $err:expr) => {
        $crate::t!(@select
            Ko => format!("바로가기를 생성하지 못했습니다.\n\n{}", $err),
            En => format!("Failed to create shortcut.\n\n{}", $err)
        )
    };
    ("sys-upload-msg-with-rank", message = $msg:expr, rank_msg = $rank:expr) => {
        $crate::t!(@select
            Ko => format!("{} ({})", $msg, $rank),
            En => format!("{} ({})", $msg, $rank)
        )
    };

    // 4) Static Key Direct Lookup (Literal preferred, Expr fallback)
    ($key:literal) => {
        $crate::ui::i18n::t_str($key)
    };
    ($key:expr) => {
        $crate::ui::i18n::t_str($key)
    };
}

pub fn t_str(key: &str) -> &'static str {
    match current_locale() {
        Locale::Ko => lookup_ko(key),
        Locale::En => lookup_en(key),
    }
}

fn lookup_ko(key: &str) -> &'static str {
    match key {
        // Domain Meta Keys
        "gold-half-random" => "핲랜",
        "gold-max-random" => "맥랜",
        "gold-random" => "랜덤",
        "assist-used" => "사용",
        "assist-caution" => "주의",
        "assist-not-used" => "미사용",

        // Settings UI
        "settings-title" => "설정",
        "settings-overlay-section" => "오버레이 설정",
        "settings-size" => "크기",
        "settings-opacity" => "투명도",
        "settings-lite-mode" => "라이트모드",
        "settings-enable" => "활성화",
        "settings-lite-mode-desc" => "추천 숨기기 및 레이아웃 축소",
        "settings-snap-position" => "오버레이 고정 위치",
        "settings-top-left" => "좌상단",
        "settings-top-right" => "우상단",
        "settings-bottom-left" => "좌하단",
        "settings-bottom-right" => "우하단",
        "settings-manual" => "수동",
        "settings-varchive-account" => "V-Archive 계정",
        "settings-link-status" => "연동 상태",
        "settings-data-sync" => "데이터 동기화",
        "settings-browse" => "찾기",
        "settings-update-section" => "업데이트 설정",
        "settings-auto-update" => "자동 업데이트",
        "settings-use" => "사용",
        "settings-version-info" => "버전 정보",
        "settings-no-steam-account" => "발견된 Steam 계정이 없습니다.",
        "settings-general" => "일반",
        "settings-language" => "언어",
        "settings-recommend-provider" => "추천 Provider",
        "settings-use-external-provider" => "외부 Provider 사용",
        "settings-display-name" => "표시 이름",
        "settings-overlay-display" => "오버레이 표시",
        "settings-always-show" => "항상 표시",
        "settings-always-show-desc" => "게임 구동 중 씬 감지(Unknown) 결과와 상관없이 오버레이를 항상 표시합니다.",
        "settings-diagnostics" => "진단 및 디버그",
        "settings-debug-window" => "디버그 창",
        "settings-show-debug-window" => "디버그 모니터링 창 표시",
        "settings-debug-window-desc" => "실시간 탐지 수치 및 진단 로그를 표출하는 디버그 창을 엽니다.",
        "settings-screen-capture" => "화면 캡처 설정",
        "settings-protect-overlay" => "캡처 시 오버레이 보호",
        "settings-prevent-screen-capture" => "화면 캡처 방지",
        "settings-protect-overlay-desc" => "해제 시 화면 캡쳐에 잡히는 대신, 특정 영역에 오버레이가 위치하면 곡 인식이 제대로 동작하지 않게 됩니다.",

        // Sync UI
        "sync-title" => "동기화",
        "sync-desc" => "Steam 계정 기준으로 업로드 후보를 확인합니다.",
        "sync-scan" => "스캔",
        "sync-upload-candidates" => "업로드 후보",
        "sync-sort-by-change" => "변경순",
        "sync-sort-by-title" => "제목순",
        "sync-varchive-sync" => "V-Archive 동기화",
        "sync-register" => "등록",
        "sync-delete" => "삭제",
        "sync-mode" => "모드",
        "sync-difficulty" => "난이도",
        "sync-max-combo-only" => "맥스콤보 달성만",
        "sync-exclude-unuploaded" => "미업로드 제외",
        "sync-reason-not-registered" => "미등록",

        // App / Viewports
        "app-settings-window" => "Overmax 설정",
        "app-close" => "닫기",
        "app-save" => "저장",
        "Linux 앱 실행" => "Linux 앱 실행",
        "앱 메뉴" => "앱 메뉴",
        "바로가기 생성" => "바로가기 생성",

        // Tray Icon
        "tray-exit" => "종료",

        // Overlay UI
        "overlay-varchive-upload-needed" => "V-Archive 업로드 필요 (클릭하여 즉시 업로드)",
        "overlay-varchive-link-needed" => "V-Archive 계정 연동 필요 (설정에서 account.txt 경로를 지정해주세요)",
        "overlay-similar-avg" => "유사 구간 평균",
        "overlay-keypart-focused" => "키파트 위주 패턴",
        "overlay-gold-rec" => "황배",
        "overlay-assist-key" => "보조",
        "overlay-no-record" => "기록 없음",

        // Recommendations
        "rec-detecting-pattern" => "패턴을 감지하는 중...",
        "rec-no-recommendations" => "추천 결과 없음",
        "rec-patterns-suffix" => "개 패턴",
        "rec-select-song" => "곡을 선택하세요",

        // Status / Notifications
        "status-scanning" => "스캔 중…",
        "status-updated" => "갱신 완료",
        "status-registered" => "등록 완료",

        // System Dialogs
        "sys-already-running" => "이미 Overmax가 실행 중입니다. 기존 인스턴스를 종료한 뒤 다시 실행하세요.",
        "sys-reason" => "사유",

        _ => "",
    }
}

fn lookup_en(key: &str) -> &'static str {
    match key {
        // Domain Meta Keys
        "gold-half-random" => "Half Random",
        "gold-max-random" => "Max Random",
        "gold-random" => "Random",
        "assist-used" => "Used",
        "assist-caution" => "Caution",
        "assist-not-used" => "Not Used",

        // Settings UI
        "settings-title" => "Settings",
        "settings-overlay-section" => "Overlay Settings",
        "settings-size" => "Size",
        "settings-opacity" => "Opacity",
        "settings-lite-mode" => "Lite Mode",
        "settings-enable" => "Enable",
        "settings-lite-mode-desc" => "Hide recommendations and shrink layout",
        "settings-snap-position" => "Overlay Snap Position",
        "settings-top-left" => "Top-Left",
        "settings-top-right" => "Top-Right",
        "settings-bottom-left" => "Bottom-Left",
        "settings-bottom-right" => "Bottom-Right",
        "settings-manual" => "Manual",
        "settings-varchive-account" => "V-Archive Account",
        "settings-link-status" => "Link Status",
        "settings-data-sync" => "Data Sync",
        "settings-browse" => "Browse",
        "settings-update-section" => "Update Settings",
        "settings-auto-update" => "Auto Update",
        "settings-use" => "Enable",
        "settings-version-info" => "Version",
        "settings-no-steam-account" => "No Steam account found.",
        "settings-general" => "General",
        "settings-language" => "Language",
        "settings-recommend-provider" => "Recommend Provider",
        "settings-use-external-provider" => "Use External Provider",
        "settings-display-name" => "Display Name",
        "settings-overlay-display" => "Overlay Display",
        "settings-always-show" => "Always Show",
        "settings-always-show-desc" => "Keeps the overlay visible at all times, regardless of scene detection (Unknown) results.",
        "settings-diagnostics" => "Diagnostics & Debug",
        "settings-debug-window" => "Debug Window",
        "settings-show-debug-window" => "Show Debug Monitoring Window",
        "settings-debug-window-desc" => "Opens a debug window showing real-time detection metrics and diagnostic logs.",
        "settings-screen-capture" => "Screen Capture Settings",
        "settings-protect-overlay" => "Protect Overlay from Capture",
        "settings-prevent-screen-capture" => "Prevent Screen Capture",
        "settings-protect-overlay-desc" => "Disabling this exposes the overlay to screen capture, but recognition may fail if the overlay covers key areas.",

        // Sync UI
        "sync-title" => "Sync",
        "sync-desc" => "Checks upload candidates for the current Steam account.",
        "sync-scan" => "Scan",
        "sync-upload-candidates" => "Upload Candidates",
        "sync-sort-by-change" => "By Change",
        "sync-sort-by-title" => "By Title",
        "sync-varchive-sync" => "V-Archive Sync",
        "sync-register" => "Register",
        "sync-delete" => "Delete",
        "sync-mode" => "Mode",
        "sync-difficulty" => "Difficulty",
        "sync-max-combo-only" => "Max Combo achieved only",
        "sync-exclude-unuploaded" => "Exclude not uploaded",
        "sync-reason-not-registered" => "Not Registered",

        // App / Viewports
        "app-settings-window" => "Overmax Settings",
        "app-close" => "Close",
        "app-save" => "Save",

        // Tray Icon
        "tray-exit" => "Exit",

        // Overlay UI
        "overlay-varchive-upload-needed" => "V-Archive upload needed (click to upload now)",
        "overlay-varchive-link-needed" => "V-Archive account link needed (set account.txt path in Settings)",
        "overlay-similar-avg" => "Similar Section Average",
        "overlay-keypart-focused" => "Key-part Focused Pattern",
        "overlay-gold-rec" => "Recommendation",
        "overlay-assist-key" => "Assist Key",
        "overlay-no-record" => "No Record",

        // Recommendations
        "rec-detecting-pattern" => "Detecting pattern...",
        "rec-no-recommendations" => "No recommendations",
        "rec-patterns-suffix" => " patterns",
        "rec-select-song" => "Please select a song",

        // Status / Notifications
        "status-scanning" => "Scanning…",
        "status-updated" => "Updated",
        "status-registered" => "Registered",

        // System Dialogs
        "sys-already-running" => "Overmax is already running. Please close the existing instance and try again.",
        "sys-reason" => "Reason",

        // Fallback to Korean if key is unhandled in English
        _ => lookup_ko(key),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use overmax_data::community::sheet_meta::{AssistMeta, GoldMeta};

    #[test]
    fn test_zero_cost_i18n_translation_and_locale_switch() {
        set_locale(Locale::Ko);
        assert_eq!(t!("settings-title"), "설정");
        assert_eq!(t!("candidate-count", n = 3), "후보 3건");
        assert_eq!(
            t!("sys-place-achieved", mode = "4B", rank = 5),
            "4B TOP 5위 달성!"
        );
        assert_eq!(
            t!("sys-upload-cache-error", error = "timeout"),
            "업로드 OK, 캐시 갱신 오류: timeout"
        );
        assert_eq!(t!("rec-patterns-count", has = 3, total = 5), "3/5곡");
        assert_eq!(
            t!("sys-shortcut-create-success", path = "/usr/bin/app"),
            "앱 메뉴 바로가기를 생성했습니다.\n\n/usr/bin/app"
        );
        assert_eq!(
            t!(
                "sys-upload-msg-with-rank",
                message = "성공",
                rank_msg = "4B TOP 1위 달성!"
            ),
            "성공 (4B TOP 1위 달성!)"
        );
        assert_eq!(t!(gold = GoldMeta::HalfRandom), "핲랜");
        assert_eq!(t!(assist = AssistMeta::Caution), "주의");

        set_locale(Locale::En);
        assert_eq!(t!("settings-title"), "Settings");
        assert_eq!(t!("candidate-count", n = 3), "3 candidates");
        assert_eq!(
            t!("sys-place-achieved", mode = "4B", rank = 5),
            "Achieved TOP 5 in 4B!"
        );
        assert_eq!(
            t!("sys-upload-cache-error", error = "timeout"),
            "Upload OK, cache error: timeout"
        );
        assert_eq!(t!("rec-patterns-count", has = 3, total = 5), "3/5 patterns");
        assert_eq!(
            t!("sys-shortcut-create-success", path = "/usr/bin/app"),
            "App menu shortcut created successfully.\n\n/usr/bin/app"
        );
        assert_eq!(t!(gold = GoldMeta::HalfRandom), "Half Random");
        assert_eq!(t!(assist = AssistMeta::Caution), "Caution");

        set_locale(Locale::Ko);
    }
}
