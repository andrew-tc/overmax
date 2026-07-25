//! Minimal hand-rolled i18n: Korean is the source-of-truth string and also the
//! lookup key, so call sites just wrap the existing literal in `t(...)`.

use overmax_data::community::sheet_meta::{AssistMeta, GoldMeta};
use serde_json::Value;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Locale {
    #[default]
    Ko,
    En,
    Ja,
}

static CURRENT_LOCALE: AtomicU8 = AtomicU8::new(0);

pub fn set_locale(locale: Locale) {
    let code = match locale {
        Locale::Ko => 0,
        Locale::En => 1,
        Locale::Ja => 2,
    };
    CURRENT_LOCALE.store(code, Ordering::Relaxed);
}

pub fn current_locale() -> Locale {
    match CURRENT_LOCALE.load(Ordering::Relaxed) {
        1 => Locale::En,
        2 => Locale::Ja,
        _ => Locale::Ko,
    }
}

/// Reads the top-level `"language"` key (`"ko"`/`"en"`/`"ja"`) from merged settings JSON.
pub fn set_locale_from_settings(settings: &Value) {
    let locale = match settings.get("language").and_then(Value::as_str) {
        Some("en") => Locale::En,
        Some("ja") => Locale::Ja,
        _ => Locale::Ko,
    };
    set_locale(locale);
}

/// Looks up `ko` (the source string, also the key) in the current locale.
/// Returns `ko` unchanged for `Locale::Ko`, or if no mapping exists for the active locale.
pub fn t(ko: &'static str) -> &'static str {
    match current_locale() {
        Locale::Ko => ko,
        Locale::En => translate_en(ko),
        Locale::Ja => translate_ja(ko),
    }
}

/// Localizes a `GoldMeta` value for display. Kept separate from `t()`'s flat
/// Korean-string keyspace because `GoldMeta`/`AssistMeta` values are also used
/// as the community sheet's canonical wire format (matched by `as_str()`
/// elsewhere), and matching on the enum variant avoids key collisions with
/// unrelated `t()` entries that happen to share the same Korean text.
pub fn t_gold(meta: GoldMeta) -> &'static str {
    match (current_locale(), meta) {
        (_, GoldMeta::None) => "",
        (Locale::Ko, _) => meta.as_str(),
        (Locale::En, GoldMeta::HalfRandom) => "Half Random",
        (Locale::En, GoldMeta::MaxRandom) => "Max Random",
        (Locale::En, GoldMeta::Random) => "Random",
        (Locale::Ja, GoldMeta::HalfRandom) => "HALFランダム",
        (Locale::Ja, GoldMeta::MaxRandom) => "MAXランダム",
        (Locale::Ja, GoldMeta::Random) => "ランダム",
    }
}

/// Localizes an `AssistMeta` value for display. See `t_gold` for why this
/// isn't routed through the flat `t()` keyspace.
pub fn t_assist(meta: AssistMeta) -> &'static str {
    match (current_locale(), meta) {
        (_, AssistMeta::None) => "",
        (Locale::Ko, _) => meta.as_str(),
        (Locale::En, AssistMeta::Used) => "Used",
        (Locale::En, AssistMeta::Caution) => "Caution",
        (Locale::En, AssistMeta::NotUsed) => "Not Used",
        (Locale::Ja, AssistMeta::Used) => "使用",
        (Locale::Ja, AssistMeta::Caution) => "注意",
        (Locale::Ja, AssistMeta::NotUsed) => "未使用",
    }
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
        "동기화 후보 찾기" => "Find Sync Candidates",
        "업데이트 설정" => "Update Settings",
        "자동 업데이트" => "Auto Update",
        "사용" => "Enable",
        "버전 정보" => "Version",
        "발견된 Steam 계정이 없습니다." => "No Steam account found.",
        "일반" => "General",
        "언어" => "Language",
        "현재 Steam" => "Current Steam",
        "추천 Provider" => "Recommend Provider",
        "외부 Provider 사용" => "Use External Provider",
        "표시 이름" => "Display Name",
        "예" => "e.g.",
        "오버레이 표시" => "Overlay Display",
        "항상 표시" => "Always Show",
        "게임 구동 중 씬 감지(Unknown) 결과와 상관없이 오버레이를 항상 표시합니다." => {
            "Keeps the overlay visible at all times, regardless of scene detection (Unknown) results."
        }
        "진단 및 디버그" => "Diagnostics & Debug",
        "디버그 창" => "Debug Window",
        "디버그 모니터링 창 표시" => "Show Debug Monitoring Window",
        "실시간 탐지 수치 및 진단 로그를 표출하는 디버그 창을 엽니다." => {
            "Opens a debug window showing real-time detection metrics and diagnostic logs."
        }
        "화면 캡처 설정" => "Screen Capture Settings",
        "캡처 방식" => "Capture Method",
        "기본값 / 안정적" => "Default / Stable",
        "실험적 / 고성능" => "Experimental / High Performance",
        "캡처 시 오버레이 보호" => "Protect Overlay from Capture",
        "화면 캡처 방지" => "Prevent Screen Capture",
        "해제 시 화면 캡쳐에 잡히는 대신, 특정 영역에 오버레이가 위치하면 곡 인식이 제대로 동작하지 않게 됩니다." => {
            "Disabling this exposes the overlay to screen capture, but recognition may fail if the overlay covers key areas."
        }

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
        "황배" => "Recommendation",
        "보조" => "Assist Key",
        "기록 없음" => "No Record",

        // overlay_recommend_ui.rs
        "패턴을 감지하는 중..." => "Detecting pattern...",
        "추천 결과 없음" => "No recommendations",
        "개 패턴" => " patterns",

        // native_app_recommend.rs
        "곡을 선택하세요" => "Please select a song",

        // native_app.rs
        "스캔 중…" => "Scanning…",
        "account.txt 경로 없음" => "account.txt path not set",
        "account.txt 파싱 실패" => "Failed to parse account.txt",
        "갱신 완료" => "Updated",
        "등록 완료" => "Registered",
        "V-Archive 실패" => "V-Archive Failed",
        "API 조회 OK, 캐시 병합 실패" => "API fetch OK, cache merge failed",
        "API 실패" => "API failed",
        "폴백 캐시 갱신 실패" => "fallback cache update failed",
        "위 달성" => " achieved",
        "업로드 OK, 캐시 갱신 오류" => "Upload OK, cache update error",

        // community/sync.rs (SyncCandidate::reason_label, translated via injected fn)
        "미등록" => "Not Registered",

        // sync_ui.rs (filter panel)
        "필터" => "Filter",
        "초기화" => "Reset",
        "모드" => "Mode",
        "난이도" => "Difficulty",
        "레벨" => "Level",
        "맥스콤보 달성만" => "Max Combo achieved only",
        "미업로드 제외" => "Exclude not uploaded",

        // native_app.rs (sync scan status)
        "후보 {n}건" => "{n} candidates",

        // system/single_instance/windows.rs (MessageBoxW)
        "이미 Overmax가 실행 중입니다. 기존 인스턴스를 종료한 뒤 다시 실행하세요." => {
            "Overmax is already running. Please close the existing instance and try again."
        }

        // system/updater/windows.rs (MessageBoxW)
        "자동 패치가 완료되지 않았습니다." => "The automatic update did not complete.",
        "사유" => "Reason",
        "새 앱 업데이트가 있습니다." => "A new app update is available.",
        "현재 버전" => "Current version",
        "최신 버전" => "Latest version",
        "지금 업데이트를 진행할까요?" => "Update now?",

        _ => ko,
    }
}

fn translate_ja(ko: &'static str) -> &'static str {
    match ko {
        // settings_ui.rs
        "설정" => "設定",
        "오버레이 설정" => "オーバーレイ設定",
        "크기" => "サイズ",
        "투명도" => "透明度",
        "라이트모드" => "ライトモード",
        "활성화" => "有効化",
        "추천 숨기기 및 레이아웃 축소" => "おすすめを非表示にしてレイアウトを縮小",
        "오버레이 고정 위치" => "オーバーレイ固定位置",
        "좌상단" => "左上",
        "우상단" => "右上",
        "좌하단" => "左下",
        "우하단" => "右下",
        "수동" => "手動",
        "V-Archive 계정" => "V-Archiveアカウント",
        "연동 상태" => "連携状態",
        "자동화" => "自動化",
        "시작 시 자동 갱신" => "起動時に自動更新",
        "데이터 동기화" => "データ同期",
        "찾기" => "参照",
        "🔍 동기화 후보 찾기" => "🔍 同期候補を探す",
        "업데이트 설정" => "アップデート設定",
        "자동 업데이트" => "自動アップデート",
        "사용" => "使用する",
        "버전 정보" => "バージョン情報",
        "발견된 Steam 계정이 없습니다." => "Steamアカウントが見つかりません。",
        "일반" => "一般",
        "언어" => "言語",
        "현재 Steam: " => "現在のSteam: ",
        "현재 Steam: -" => "現在のSteam: -",

        // sync_ui.rs
        "동기화" => "同期",
        "Steam 계정 기준으로 업로드 후보를 확인합니다." => "現在のSteamアカウントを基準にアップロード候補を確認します。",
        "스캔" => "スキャン",
        "업로드 후보" => "アップロード候補",
        "변경순" => "変更順",
        "제목순" => "タイトル順",
        "V-Archive 동기화" => "V-Archive同期",
        "등록" => "登録",
        "삭제" => "削除",

        // native_app_viewports.rs
        "Overmax 설정" => "Overmax設定",
        "닫기" => "閉じる",
        "저장" => "保存",

        // tray_icon.rs
        "디버그 로그" => "デバッグログ",
        "종료" => "終了",

        // overlay_ui.rs
        "V-Archive 업로드 필요 (클릭하여 즉시 업로드)" => "V-Archiveアップロードが必要です(クリックして今すぐアップロード)",
        "V-Archive 계정 연동 필요 (설정에서 account.txt 경로를 지정해주세요)" => {
            "V-Archiveアカウント連携が必要です(設定でaccount.txtのパスを指定してください)"
        }
        "유사 구간 평균" => "類似区間平均",
        "키파트 위주 패턴" => "キーパート中心パターン",
        "황배:" => "おすすめ:",
        "보조:" => "アシストキー:",
        "기록 없음" => "記録なし",

        // overlay_recommend_ui.rs
        "패턴을 감지하는 중..." => "パターンを検出中...",
        "추천 결과 없음" => "おすすめなし",
        "개 패턴" => " パターン",

        // debug_ui.rs
        "▶ 재개" => "▶ 再開",
        "⏸ 일시정지" => "⏸ 一時停止",
        "🗑 지우기" => "🗑 クリア",
        "필터:" => "フィルター:",

        // native_app_recommend.rs
        "곡을 선택하세요" => "曲を選択してください",

        // native_app.rs
        "스캔 중…" => "スキャン中…",
        "account.txt 경로 없음" => "account.txtのパスが未設定です",
        "account.txt 파싱 실패" => "account.txtの解析に失敗しました",
        "갱신 완료" => "更新完了",
        "등록 완료" => "登録完了",
        "V-Archive 실패: " => "V-Archive失敗: ",
        "API 조회 OK, 캐시 병합 실패: " => "API取得OK、キャッシュ統合失敗: ",
        "API 실패 (" => "API失敗 (",
        "), 폴백 캐시 갱신 실패: " => "), フォールバックキャッシュ更新失敗: ",
        "위 달성!)" => "位達成!)",
        "업로드 OK, 캐시 갱신 오류: " => "アップロードOK、キャッシュ更新エラー: ",

        // community/sync.rs (SyncCandidate::reason_label, translated via injected fn)
        "미등록" => "未登録",

        // sync_ui.rs (filter panel)
        "필터" => "フィルター",
        "초기화 ↺" => "リセット ↺",
        "모드" => "モード",
        "난이도" => "難易度",
        "레벨" => "レベル",
        "맥스콤보 달성만" => "マックスコンボ達成のみ",
        "미업로드 제외" => "未アップロードを除外",

        // native_app.rs (sync scan status)
        "후보 {n}건" => "候補 {n}件",

        // system/single_instance/windows.rs (MessageBoxW)
        "이미 Overmax가 실행 중입니다. 기존 인스턴스를 종료한 뒤 다시 실행하세요." => {
            "Overmaxは既に起動しています。既存のインスタンスを終了してから再度実行してください。"
        }

        // system/updater/windows.rs (MessageBoxW)
        "자동 패치가 완료되지 않았습니다.\n\n사유: " => "自動パッチが完了しませんでした。\n\n理由: ",
        "새 앱 업데이트가 있습니다.\n\n현재 버전: " => "新しいアップデートがあります。\n\n現在のバージョン: ",
        "\n최신 버전: " => "\n最新バージョン: ",
        "\n\n지금 업데이트를 진행할까요?" => "\n\n今すぐアップデートしますか?",

        _ => ko,
    }
}
