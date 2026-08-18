# Architecture Robustness and Concurrency Guard Implementation Plan

**작성일**: 2026-08-18  
**목표**: SQLite 동시 쓰기 락 충돌(`SQLITE_BUSY`) 방지, 설정 동기화 큐 안정화, 캐시 전파 일원화, 디텍션 워커 Repaint 스케줄링 최적화.  
**대상 크레이트**: `overmax_data`, `overmax_app`, `overmax_engine`  
**불변 제약 조건**: 기존 `record.db` 스키마 호환 유지, `settings.user.json` delta 형식 유지, 100% OCR-Free 및 인게임 성능 우선.

---

## 1. 문제 정의

1. **SQLite 다중 스레드 동시 쓰기 시 `SQLITE_BUSY` 발생 및 플레이 기록 유실 위험**:
   - `RecordDB`에서 커넥션을 열 때 PRAGMA WAL 모드와 `busy_timeout`이 설정되지 않아, V-Archive 동기화 중 게임 결과창 저장 시 락 충돌로 기록이 실패할 수 있음.
2. **`SharedSettings` 비동기 저장 시 스레드 폭주 및 I/O 레이스 컨디션**:
   - UI 슬라이더 조작 시 매 이벤트마다 `std::thread::spawn`을 호출하여 `settings.user.json` 파일 쓰기 경합 및 디스크 I/O 낭비 발생.
3. **`StartupCacheManager` 캐시 갱신 시 `Recommender` 내부 포인터 미전파 (Stale Reference)**:
   - 백그라운드에서 `songs.json`이 갱신되어 메인 UI의 `varchive_db`가 교체되어도 `Recommender` 내부의 `varchive_db` 핸들은 구버전으로 유지됨.
4. **정적 화면에서의 불필요한 `request_repaint()` 렌더링 낭비**:
   - 화면에 아무런 변화가 없는 정지 상태에서도 `DetectionWorker`가 매 틱마다 무조건 `ctx.request_repaint()`를 호출하여 GPU/CPU 리소스 낭비.

---

## 2. 세부 구현 계획

### Phase 1: SQLite 다중 스레드 동시성 가드 (`overmax_data::store::record_db`)

1. **공통 커넥션 팩토리 `open_connection()` 구현**:
   - `Connection::open(&self.db_path)` 하드코딩 호출을 단일 내부 헬퍼 `open_conn(&self) -> Result<Connection>`으로 일원화.
   - 커넥션 생성 직후 필수 PRAGMA 적용:
     ```sql
     PRAGMA journal_mode = WAL;
     PRAGMA busy_timeout = 5000;
     PRAGMA synchronous = NORMAL;
     ```
2. **`save()` 및 `upsert_varchive_record()`에 재시도 가드 적용**:
   - 일시적인 `SQLITE_BUSY` 발생 시 최대 3회 지수 백오프(10ms, 25ms, 50ms) 재시도 루프 적용.

---

### Phase 2: 설정 저장 I/O 큐 및 디바운스 워커 (`overmax_app::ui::settings_ui`)

1. **설정 디스크 저장 전용 채널 및 워커 구축**:
   - 슬라이더 조작 시 매번 `std::thread::spawn`을 띄우지 않고, `mpsc::Sender<SaveSettingsRequest>`로 채널 전송.
   - 백그라운드 단일 워커가 100ms 디바운스(Debounce / Coalesce) 큐를 유지하여 마지막 최종 상태만 디스크에 안전하게 원자적 저장(`tempfile` ➔ `atomic rename`).
2. **`SharedSettings` 락 점유 시간 최소화**:
   - UI 렌더링 루프에서는 스냅샷 복사만 수행하고 무거운 직렬화 연산은 저장 워커 스레드 내부에서 수행.

---

### Phase 3: `StartupCacheManager` 캐시 전파 일원화 (`overmax_app::ui::native_app`)

1. **`NativeApp::poll_startup_cache_updates()` 정비**:
   - `startup_cache_manager.poll_updates(&mut self.varchive_db, &mut self.sheet_meta)`가 `true`를 반환할 때:
     - `self.recommender = Arc::new(Recommender::new(self.varchive_db.clone(), self.record_manager.clone()));`
     - `self.sheet_meta` 및 관련 뷰포트 상태를 원자적으로 동시 갱신.

---

### Phase 4: `DetectionWorker` Repaint 스케줄링 게이트 (`overmax_engine::detector::detection_worker`)

1. **`DetectionOutput` 변화 감지 게이트 (Throttling)**:
   - `DetectionWorker` 내부에 이전 프레임의 핵심 상태(스냅 좌표 `game_rect`, 세션 상태 `state`, `current_song_id`, `confidence`, `jacket_status`)를 추적.
   - 이전 프레임과 비교하여 실질적인 변화가 발생했거나 창이 이동 중일 때만 `(self.repaint_callback)()` 호출.
   - 정지 상태에서 변화가 없을 때는 불필요한 Repaint를 건너뜀 (최대 1초 주기 하트비트만 유지).

---

## 3. 검증 전략

1. **단위 및 통합 테스트**:
   - `cargo test --workspace`
   - SQLite 동시 읽기/쓰기 멀티스레드 스트레스 테스트 작성 (`test_concurrent_record_db_writes`)
2. **Clippy 및 린트 검증**:
   - `cargo clippy --all-targets`
3. **실행 및 회귀 검증**:
   - 설정창 슬라이더 드래그 시 CPU 사용량 및 파일 저장 무결성 확인.
