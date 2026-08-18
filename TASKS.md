# TASKS

Overmax v0.4.0 마일스톤 활성 작업 목록 및 백로그입니다.  
(이전 완료 작업 목록은 [`docs/archive/tasks/TASKS_v0.4.0_archive.md`](docs/archive/tasks/TASKS_v0.4.0_archive.md)를 참조)

---

## 1. 아키텍처 안정성 및 동시성 가드 (Architecture Robustness & Boundaries)

- [x] **1.1 SQLite 다중 스레드 동시성 가드 (`SQLITE_BUSY` 방지)**
  - [x] `record.db` 연결 시 `PRAGMA journal_mode=WAL;` 및 `busy_timeout` (5000ms) 설정 강제 (`open_conn` 팩토리 일원화)
  - [x] 디텍션 워커의 플레이 기록 `upsert` 및 쓰기 메서드에 `SQLITE_BUSY` 지수 백오프 재시도(`with_retry`) 가드 추가 및 멀티스레드 동시성 스트레스 테스트 완료
- [x] **1.2 설정(`SharedSettings`) 동기화 안전성 강화 및 I/O 큐 분리**
  - [x] UI 슬라이더 조작 시 무차별 `std::thread::spawn` 호출을 방지하는 단일 백그라운드 설정 저장 큐(`SettingsDebounceWriter`, 100ms Debounce) 구축
  - [x] `settings.user.json` 임시 파일 쓰기 후 atomic rename 교체로 파일 손상 방지
- [x] **1.3 `StartupCacheManager` 캐시 갱신 전파 일원화 (Stale Reference 해소)**
  - [x] 백그라운드 `songs.json` 갱신 시 `NativeApp`의 `varchive_db`뿐만 아니라 `Recommender` 내부 캐시 포인터도 함께 갱신하도록 `drain_fetch_results` 파이프라인 일원화
- [x] **1.4 디텍션 워커 틱과 egui Repaint 스케줄링 최적화**
  - [x] `RepaintFingerprint` 값 객체 기반 단일 Change Detector 도입으로 정적 화면에서 불필요한 `request_repaint()` 0회 차단 (GPU/CPU 낭비 제거)
  - [x] 창 위치 이동(`game_rect`), 씬 감지, 자켓 매칭, 캡처 에러 등 실질적인 렌더 변화 시에만 100% Repaint 트리거 보장

---

## 2. 크레이트별 책임 명확화 및 모듈 경계 정리 (Crate Boundary & Responsibility Refactoring)

- [ ] **2.1 [정리 1순위] ROI(관심영역) 좌표 설정(`scene_config.rs`)을 `overmax_engine`으로 이동**
  - [ ] `overmax_data::config::scene_config`의 `GlobalRoiConfig`, `SceneRoiConfig`, `RoiRect`를 `overmax_engine::detector::roi` 모듈로 이관
  - [ ] 디텍션 파이프라인 전용 ROI 좌표 정의와 런타임 연산을 `overmax_engine` 단일 책임으로 일원화
- [ ] **2.2 [정리 2순위] V-Archive 네트워크 I/O(`reqwest`) 및 외부 API 통신을 `overmax_data`로 일원화**
  - [ ] `overmax_app::system`의 `varchive_upload`, `cache_update`, `recommend_provider_fetch`를 `overmax_data::community`로 통합
  - [ ] `overmax_app`이 비즈니스 HTTP I/O를 직접 다루지 않고 순수 UI 렌더링 및 OS 이벤트 핸들링에 집중하도록 격리
- [ ] **2.3 [정리 3순위] `overmax_cv` 미사용 함수 정리 (`hog`는 DB Builder 레거시 DB 지원용으로 보존)**
  - [ ] `overmax_cv`에서 완전히 미사용되는 `compute_image_features_v2`, `compute_hashes_gray` 등 데드코드 정리
  - [ ] `hog.rs` 및 `compute_image_hog`는 `db_builder` 오프라인 DB 생성 및 레거시 호환을 위해 안전하게 유지

---

## 3. 감지 씬 다양화 및 인게임 확장

- [ ] **3.1 래더매치(Ladder Match) 씬 감지 대응**
  - [ ] 래더매치 밴픽/선곡 화면 및 대기실 감지 대응
  - [ ] 래더매치 결과창 인식 지원

---

## 4. 다국어 (i18n) 지원 확장

- [ ] **4.1 일본어(JA) 번역 및 폰트 지원 추가**
  - [ ] UI 및 오버레이 텍스트 일본어 리소스 작성
  - [ ] 일본어 CJK 폰트 렌더링 검증

---

## 5. 장기 백로그 (Long-term Backlog)

- [ ] **5.1 `overmax_engine`과 `overmax_data` 계층 결합도 완화 (Event-driven Architecture)**
  - [ ] `DetectionPipeline` 내부의 SQLite 직접 `upsert` 의존성을 제거하고, 엔진은 `VerifiedPlayEvent` 방출만 담당하도록 책임 분리
- [ ] **5.2 공식 V-Archive 클라이언트 보완/대체 자동 업로드 파이프라인 (장기)**
  - [ ] 게임 플레이 종료 시 감지된 플레이 기록을 V-Archive API로 안전하게 자동 백그라운드 업로드하는 파이프라인 설계
