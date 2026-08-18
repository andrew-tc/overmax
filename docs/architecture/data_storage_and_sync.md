# Data Storage, Cache and Sync Architecture

이 문서는 Overmax의 데이터 저장소(SQLite), 캐시 수명주기 관리(`StartupCacheManager`), V-Archive 증분 동기화, Steam 계정 감지 및 추천 프로바이더 연동 아키텍처를 설명한다.

---

## 1. 데이터 계층 개요 및 저장 구조

Overmax의 데이터 계층은 `rust/overmax_data/` 및 `rust/overmax_app/src/system/`에 분산되어 있으며, 설정·캐시·기록 데이터를 다음과 같이 분리 관리한다.

```
overmax/
├── settings.json              # 기본 정적 설정 (Default Config)
├── settings.user.json         # 사용자 변경분만 저장 (Delta Format)
└── cache/
    ├── songs.json             # V-Archive 곡 메타데이터 캐시 (정적/주기적 갱신)
    ├── image_index.db         # 자켓 HOG/해시 피처 인덱스 DB
    ├── record.db              # SQLite 로컬 플레이 기록 및 V-Archive 기록 DB
    └── provider_cache.json    # 외부 추천 Provider 응답 캐시
```

---

## 2. 무중단 기동을 위한 캐시 관리 (`StartupCacheManager`)

앱 실행 시 원격 API로부터 곡 메타데이터(`songs.json`)를 다운로드하는 동안 UI 스레드가 멈추는 현상을 방지하기 위해 2단계 기동 전략을 적용한다.

```
                           [ 앱 실행 (main) ]
                                   │
                    ┌──────────────┴──────────────┐
                    ▼                             ▼
        [ Cold Start (최초 실행) ]        [ Warm Start (캐시 존재) ]
        - 필수 캐시 파일 없음             - 기존 cache/songs.json 존재
        - UI 렌더 전 동기 다운로드        - 0.1초 즉시 UI 기동 (기존 캐시 로드)
        - 실패 시 기본 내장 데이터 폴백     - 백그라운드 스레드에서 TTL(24h) 검사
                                          - 갱신 완료 시 mpsc 전송 ➔ Arc 재바인딩
```

### 1) Cold Start (최초 실행)
* `cache/songs.json` 또는 필수 데이터가 없는 경우, 메인 윈도우 생성 직전 동기 HTTP 요청을 통해 최신 메타데이터를 내려받는다.
* 네트워크 연결 실패 시 바이너리에 내장된 기본 곡 정보를 로드하여 실행 실패를 방지한다.

### 2) Warm Start (캐시 존재 시)
* 기존 캐시 파일이 존재하는 경우 즉시 메모리에 적재하여 **0.1초 이내에 오버레이 윈도우를 띄운다**.
* 백그라운드 스레드(`StartupCacheManager`)가 캐시 파일의 수정 시간을 검사하여 24시간(TTL)이 경과했으면 비동기로 최신 데이터를 다운로드한다.
* 다운로드가 완료되면 `mpsc::channel`을 통해 새 `VArchiveDB`를 메인 UI로 전달하고, 메인 렌더 루프(`poll_updates`)에서 `*varchive_db = Arc::new(new_vdb)`로 스마트 포인터를 교체하므로 디텍션 및 렌더 루프의 락 대기 없이 즉시 반영된다.

---

## 3. SQLite 로컬 저장소 아키텍처 (`RecordDB`)

플레이어의 기록과 V-Archive 동기화 기록은 `cache/record.db` (SQLite)에서 단일 파일로 관리된다.

```
┌─────────────────────────────────┐       ┌─────────────────────────────────┐
│         Table: records          │       │     Table: varchive_records     │
│  - 로컬 오버레이 디텍션 확정 기록│       │  - V-Archive 원격 계정 다운로드  │
├─────────────────────────────────┤       ├─────────────────────────────────┤
│ PK: (song_id, mode, diff)       │       │ PK: (song_id, mode, diff)       │
│ Columns: score, rate, max_combo,│       │ STORED Columns: score, rating,  │
│          updated_at             │       │                 max_combo       │
└─────────────────────────────────┘       └─────────────────────────────────┘
                 │                                         │
                 └───────────────────┬─────────────────────┘
                                     │ (LEFT JOIN Single Query)
                                     ▼
                      [ Sync Candidate Extraction ]
                      - 갱신 대상 곡 즉시 산출 (O(N) 쿼리)
```

### 1) STORED 가상 컬럼 및 인덱스 최적화
* V-Archive API에서 받아온 JSON 페이로드에서 자주 조회되는 필드(`score`, `rating`, `max_combo`, `updated_at`)를 SQLite의 **생성형 물리 컬럼(STORED Generated Columns)**으로 추출한다.
* 복합 기본키 `(song_id, mode, diff)` 및 인덱스를 구성하여, 매 프레임 발생하는 선곡창 기록 조회(`get_rate_map`) 시 O(1) 수준의 빠른 읽기 성능을 보장한다.

### 2) Thread-Local 커넥션 캐싱
* UI 루프 및 디텍션 파이프라인에서 `RecordDB`를 조회할 때 매번 `Connection::open` 시스템 콜을 수행하는 오버헤드를 줄이기 위해, 스레드별 SQLite 커넥션을 `thread_local!`에 캐싱하여 재사용한다.
* 이를 통해 평균 읽기 지연시간을 1.09ms에서 **0.20ms(약 81.7% 감소)**로 단축했다.

---

## 4. V-Archive 증분 동기화 & O(1) 단일 곡 머지

```
[ 전체 동기화 스캔 ]
  - 로컬 캐시의 최신 updated_at 타임스탬프 획득
  - API 호출: GET /api/records?since={timestamp}
  - 변경된 N개 레코드만 다운로드 후 SQLite varchive_records 테이블에 upsert

[ 단일 곡 플레이 완료 업로드 ]
  - 1곡 업로드 API 호출: POST /api/record
  - 성공 시 해당 곡의 RecordKey(song_id, mode, diff)만 로컬 RecordManager에 O(1) Upsert
  - 전체 DB 재조회(refresh) 없이 메모리 캐시 및 dirty 키만 즉시 동기화
```

1. **`since` 파라미터 기반 증분 동기화**:
   동기화 창(`SyncWindow`) 열람 시 수천 개의 전체 기록을 매번 재다운로드하지 않고, 마지막 동기화 이후 갱신된 기록만 수신하여 네트워크 트래픽과 I/O를 최소화한다.
2. **단일 곡 업로드 O(1) 증분 갱신**:
   게임 결과창에서 점수가 갱신되어 V-Archive로 전송될 때, 전체 캐시를 무효화하고 다시 쿼리하는 대신 메모리 내 `RecordManager`의 해당 곡 인덱스만 O(1)로 직접 갱신하여 디스크 I/O 렉을 방지한다.
3. **가짜 동기화 후보(False Positive) 방지**:
   로컬 `records`와 `varchive_records`를 단일 SQL `LEFT JOIN` 쿼리로 대조하여, 실제 갱신이 필요한 곡만 정확히 추출한다.

---

## 5. Steam 계정 감지 파이프라인 (`SteamSession`)

로컬에 설치된 Steam 클라이언트 설정으로부터 현재 DJMAX를 플레이 중인 플레이어의 Steam ID와 V-Archive 연동 계정을 자동 추출한다.

### 4단계 계정 탐색 폴백
1. `loginusers.vdf` 내 `MostRecent = "1"` 플래그를 가진 활성 계정 탐색.
2. `AutoLoginUser` 설정에 일치하는 계정 탐색.
3. `Timestamp` 값이 가장 최신인 계정 탐색.
4. Linux 환경의 경우 Native Steam 경로(`~/.local/share/Steam`) 실패 시 Flatpak Steam 경로(`~/.var/app/com.valvesoftware.Steam`) 순차 탐색.

---

## 6. 추천 시스템 및 외부 Provider 연동

Overmax의 추천 시스템은 Trait 기반으로 추상화되어 있어 기본 내장 추천과 외부 커뮤니티 서비스를 병합할 수 있다.

```
                    [ NativeApp::recommend_for_state ]
                                   │
                                   ▼
                       [ CompositeRecommender ]
                                   │
         ┌─────────────────────────┴─────────────────────────┐
         ▼                                                   ▼
[ LocalFloorRecommender ]                         [ ProviderRecommender ]
 - 내장 Floor 근접 난이도 추천                     - 외부 HTTP Provider (행이봇 등)
 - 오프라인 100% 동작 보장                         - RecommendProviderFetchWorker가
 - Footer 평균 레이팅 통계 산출                      주기적으로 백그라운드 수신
```

* **`RecommendationSource` Trait**: 모든 추천 엔진(로컬, 외부)은 동일한 인터페이스를 구현하며, `entries` 목록만 반환한다.
* **`vary` 컨텍스트 협상**: Provider는 자신이 반응하는 컨텍스트(`song_id`, `mode`, `diff`, `v_id`)를 선언하며, `vary = []`인 경우 곡 변경 시 불필요한 HTTP 재요청이 발생하지 않는다.
* **Fail-Closed 안전장치**: 외부 네트워크 실패나 타임아웃 발생 시 에러를 노출하지 않고 조용히 로컬 `LocalFloorRecommender` 결과만 표시한다.
