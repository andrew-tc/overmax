# TASKS

Overmax v0.4.0 마일스톤 활성 작업 목록 및 백로그입니다.  
(이전 완료 작업 목록은 [`docs/archive/tasks/TASKS_v0.4.0_archive.md`](docs/archive/tasks/TASKS_v0.4.0_archive.md)를 참조)

---

## 1. 아키텍처 안정성 및 동시성 가드 (Architecture Robustness & Boundaries)

- [ ] **1.1 SQLite 다중 스레드 동시성 가드 (`SQLITE_BUSY` 방지)**
  - [ ] `record.db` 연결 시 `PRAGMA journal_mode=WAL;` 및 `busy_timeout` (5000ms) 설정 강제
  - [ ] 디텍션 워커의 플레이 기록 `upsert` 시 `SQLITE_BUSY` 재시도(Retry with backoff) 가드 추가로 플레이 기록 유실 원천 차단
- [ ] **1.2 설정(`SharedSettings`) 동기화 안전성 강화 및 I/O 큐 분리**
  - [ ] `serde_json::Value` 뮤텍스 락 경합 해소를 위한 정적 Typed Config 구조체 기반 스냅샷 읽기 적용
  - [ ] UI 슬라이더 조작 시 무차별 `std::thread::spawn` 호출을 방지하는 단일 백그라운드 설정 저장 큐(Debounce Worker) 구축
- [ ] **1.3 `StartupCacheManager` 캐시 갱신 전파 일원화 (Stale Reference 해소)**
  - [ ] 백그라운드 `songs.json` 갱신 시 `NativeApp`의 `varchive_db`뿐만 아니라 `Recommender` 내부 캐시 포인터도 함께 갱신하도록 Refresh 파이프라인 일원화
- [ ] **1.4 디텍션 워커 틱과 egui Repaint 스케줄링 최적화**
  - [ ] 정적 화면(화면 변화 없음)에서 매 틱마다 `ctx.request_repaint()`가 호출되어 발생하는 불필요한 GPU/CPU 렌더링 낭비 방지
  - [ ] `DetectionOutput`이 이전 프레임 대비 실질적으로 변경되었거나 창 위치가 이동했을 때만 Repaint를 요청하는 Throttle/Gate 적용

---

## 2. 감지 씬 다양화 및 인게임 확장

- [ ] **2.1 래더매치(Ladder Match) 씬 감지 대응**
  - [ ] 래더매치 밴픽/선곡 화면 및 대기실 감지 대응
  - [ ] 래더매치 결과창 인식 지원

---

## 3. 다국어 (i18n) 지원 확장

- [ ] **3.1 일본어(JA) 번역 및 폰트 지원 추가**
  - [ ] UI 및 오버레이 텍스트 일본어 리소스 작성
  - [ ] 일본어 CJK 폰트 렌더링 검증

---

## 4. 장기 백로그 (Long-term Backlog)

- [ ] **4.1 `overmax_engine`과 `overmax_data` 계층 결합도 완화 (Event-driven Architecture)**
  - [ ] `DetectionPipeline` 내부의 SQLite 직접 `upsert` 의존성을 제거하고, 엔진은 `VerifiedPlayEvent` 방출만 담당하도록 책임 분리
- [ ] **4.2 공식 V-Archive 클라이언트 보완/대체 자동 업로드 파이프라인 (장기)**
  - [ ] 게임 플레이 종료 시 감지된 플레이 기록을 V-Archive API로 안전하게 자동 백그라운드 업로드하는 파이프라인 설계
