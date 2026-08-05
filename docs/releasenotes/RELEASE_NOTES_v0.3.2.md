# Overmax v0.3.2 릴리즈 노트

> v0.3.1 이후 변경 사항 (v0.3.2)

---

## 🚀 주요 신규 기능 및 사용자 변경 사항

### 🎵 1. 추천곡 다양화 (Recommend Provider Protocol v1 & 외부 프로바이더 연동)
* **추천 시스템 다중 소스 추상화 (`CompositeRecommender`)**:
  * 기존 로컬 유사 구간 추천 시스템을 `RecommendationSource` trait으로 추상화하고, 로컬 추천과 외부 커뮤니티 추천 엔진을 한 화면에 병합할 수 있는 `CompositeRecommender` 아키텍처를 구축했습니다.
* **외부 추천 프로바이더 연동 지원 (Recommend Provider Protocol v1)**:
  * 설정창 `System` 탭에서 외부 추천 서버 URL(예: `http://localhost:8080`)을 입력하고 토글을 켜서 커뮤니티/AI 추천 곡을 오버레이로 받아볼 수 있습니다.
  * 설정창에서 프로바이더 연결 상태 및 응답을 실시간으로 확인할 수 있는 **연결 테스트** 기능이 추가되었습니다.
* **Host Enrichment & 개인 성적 자동 합성**:
  * 외부 프로바이더가 내보낸 원시 튜플 `(song_id, mode, diff)`을 호스트 오버레이가 수신하여, 로컬 DB의 곡 메타데이터(곡명, 작곡가, 난이도 레벨) 및 사용자 본인의 실제 달성 성적(Rate %, MaxCombo)을 자동 병합하여 깔끔하게 노출합니다.
* **추천 스코어(`score`) 기반 내림차순 정렬**:
  * 외부 프로바이더가 부여한 매칭 점수(`score`)를 기반으로 가장 추천도가 높은 곡이 오버레이 상단에 우선 표시됩니다.
* **V-Archive ID(`v_id`) 자동 전달**:
  * V-Archive 계정이 연동되어 있는 경우 사용자의 `v_id`가 추천 요청 시 함께 전달되어 **개인 맞춤형 추천** 결과를 수신할 수 있습니다.
* **커뮤니티 공개 스펙 & 파이썬 예제 서버 제공**:
  * 외부 추천 엔진 개발자를 위한 프로토콜 문서 `docs/overmax-recommend-protocol-v1.md` 및 바로 실행해 볼 수 있는 Python Mock 서버 예제 `examples/recommend_mock_server.py`를 공개했습니다.

### 🐧 2. Linux 환경 편의성 강화 (Contributed by @KHanwuL)
* **Linux 자동 업데이트 지원 (`AppUpdater`)**:
  * Linux 환경에서도 최신 버전을 자동으로 감지하고 업그레이드할 수 있는 자동 업데이트 모듈이 추가되었습니다.
* **Steam 자동 실행 `.desktop` 파일 생성**:
  * Linux 환경의 Steam에서 Overmax가 자동으로 함께 시작되도록 돕는 `.desktop` 파일 자동 생성 기능 및 설정 가이드가 이식되었습니다.

---

## ⚡ 성능 및 내부 아키텍처 최적화

### ⚡ 1. Zero-Copy `ImageView` 기반 감지 파이프라인
* **메모리 복사 소거 (Zero-Copy ROI Cropping)**:
  * 씬 감지 및 ROI 버퍼 슬라이싱 시 픽셀 데이터를 메모리에 매번 뷰/복사하지 않고, 메모리 주소를 직접 참조하는 `ImageView` 슬라이스 구조를 도입하여 CPU 및 메모리 대역폭 소모를 대폭 줄였습니다.

### 🏎️ 2. `JacketMatcher` 고속화 (SoA Layout + SIMD SAD)
* **SoA (Structure of Arrays) & SIMD SAD 연산 적용**:
  * 자켓 템플릿 매칭 연산을 SoA 메모리 레이아웃으로 변경하고 SIMD 기법을 적용하여 템플릿 탐색 속도를 극대화했습니다.
  * 히스토그램 조기 이탈(Early Exit) 필터 및 지연 페칭(Lazy Fetching)을 적용하여 불필요한 이미지 연산을 차단했습니다.

### 🛡️ 3. Pure Rust CV 템플릿 엔진 일원화 & OCR 완전 제거
* **Windows OCR 의존성 100% 제거**:
  * Legacy Windows OCR 디텍터 모듈을 완전히 삭제하고, 오직 Pure Rust CV 템플릿 엔진으로 씬 및 텍스트 감지 프로세스를 단일화했습니다.
* **`Bgr<T>` 구조체 도입 및 채널 연산 표준화**:
  * BGR 픽셀 타입 및 휘도(Luminance) 연산을 `Bgr` 구조체 및 `LumaMethod` 헬퍼로 통합하여 이미지 처리 정확도와 코드 가독성을 보장했습니다.

### 🔒 4. Type-Safe `Mode` & `Difficulty` Enum 전면화
* **문자열 벼락 표현 소거**:
  * 프로젝트 전반에 남아있던 `"4B"`, `"MX"` 형태의 임의 문자열 표현을 `overmax_core::Mode` 및 `Difficulty` type-safe Enum으로 전환했습니다.
  * `RecordKey` 구조체를 `(i32, Mode, Difficulty)` 튜플로 경량화하고 `Arc<str>`을 적극 활용하여 힙 메모리 할당을 대폭 감축했습니다.

### ⏳ 5. Manifest 메모리 캐싱 & HTTP 스팸 차단
* **Manifest 캐싱 (1시간 TTL) & 10초 스레드 쿨다운**:
  * 프로바이더의 `/manifest` 정보를 1시간 동안 메모리에 저장하고, 오버레이 렌더링 루프에서 캐시 파일 modification 시간을 대조하여 10초 이내 요청 시 백그라운드 스레드 생성 자체를 생략(Early Return)함으로써 네트워크 및 시스템 트래픽을 최적으로 제어했습니다.


