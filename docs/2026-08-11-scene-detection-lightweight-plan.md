# 씬 감지 파이프라인 경량화 구현 계획

**대상 파일**: `rust/overmax_engine/src/detector/detection_pipeline.rs`  
**작성 목적**: 게임 플레이 중(Unknown 씬 유지 구간)에 발생하는 프레임 드랍 완화  
**적용 원칙**: `ENGINEERING_TASTE.md` — 작은 증분 변경, 씬 전이 반응성 유지, 임계치/우선순위 등 기존 동작(behavior)은 보존  

---

## 배경 및 문제 정의

`DetectionPipeline::detect_logo_if_due`는 게임 플레이 중(`last_logo_scene == Unknown`)에도 씬 재확인을 위해 주기적으로 `parse_static_scene()`을 호출한다. 이 함수는 매 호출마다 다음 세 개를 **순차적으로 전부** 실행한다:

1. `detect_result_scene_via_edge` (ResultFreestyle 재킷 ROI)
2. `detect_freestyle_scene_via_edge` (Freestyle 재킷 ROI)
3. `detect_openmatch_scene_via_edge` (OpenMatch 재킷 ROI)

각 함수 내부는 "저렴한 게이트(엣지 강도 / 카테고리 밴드 단색성) → 통과 시 `match_jacket()`(전체 이미지 DB 대상 pHash/dHash/aHash + 히스토그램 매칭)" 구조다. `match_jacket()`이 이 파이프라인에서 가장 비싼 연산이므로, 게이트가 오탐(false positive)으로 자주 열리면 게임 플레이 중에도 `match_jacket()`이 트리거되어 프레임 드랍의 원인이 될 수 있다.

**개선 방향**: 폴링 주기(반응성)는 건드리지 않는다. 대신
(a) 이미 계산되는 두 게이트(엣지/밴드)가 short-circuit 없이 둘 다 계산되는 낭비를 제거하고,
(b) `match_jacket()`이 Unknown 씬 구간에서 실제로 얼마나 자주 트리거되는지 먼저 측정한 뒤,
(c) 측정 결과에 따라 추가 프루닝 여부를 결정한다.

**금지 사항**:
- `JACKET_EDGE_THRESHOLD`, `similarity_threshold` 등 기존 임계치 상수를 변경하지 않는다.
- Freestyle / OpenMatch / Result 판별 우선순위 및 씬 확정 로직(`commit_result_scene`, `HysteresisBuffer`)을 변경하지 않는다.
- `match_jacket()` 내부 알고리즘(SoA 해시 순회, early-exit, 히스토그램 대조)은 건드리지 않는다.

---

## Step 0: 사전 확인 (죽은 코드 확인, 진행 전 필수)

씬 판별이 자켓 엣지/밴드/유사도 매칭으로 완전히 이전된 이후 `logo_roi` / `get_roi("logo")` 경로가 실제 판별 흐름에서 쓰이는지 재확인이 필요하다. (2026-07-16 CONTEXT.md 결정 로그: "Windows OCR 로고 스캔 최종 폴백 완전히 비활성화")

```bash
rg -n 'get_roi\("logo"|get_roi_for_scene\("logo"|logo_roi\(\)' rust/
```

- `detection_pipeline.rs`, `play_state.rs`, `templates/` 내에서 실사용 호출이 없고 `#[ignore]` 테스트(`test_scratch_images`)에서만 참조된다면 **로고 ROI는 현재 판별 경로에서 죽은 코드**로 간주한다.
- 이 Step 0는 정보 수집 목적이며, 이번 계획의 어떤 Step에서도 로고 ROI 코드를 삭제하지 않는다 (스코프 외 — 별도 정리 티켓으로 분리).
- 결과를 커밋 메시지나 PR 설명에 한 줄로 남긴다 (예: "logo ROI confirmed unused in current detection path; out of scope for this change").

---

## Step 1: 게이트 short-circuit 적용 (엣지/밴드 중복 계산 제거)

### 문제

`detect_result_scene_via_edge`, `detect_freestyle_scene_via_edge`, `detect_openmatch_scene_via_edge` 세 함수 모두 다음과 동일한 패턴을 갖는다:

```rust
let edge_ok = detect_jacket_edges(frame, jacket_roi, rois.scale())
    .map(|edge_strength| edge_strength >= JACKET_EDGE_THRESHOLD)
    .unwrap_or(false);
let band_ok = check_category_band_solid(frame, jacket_roi, rois.scale());

if edge_ok || band_ok {
    // ... match_jacket 호출
}
```

`edge_ok`와 `band_ok`가 각각 변수에 먼저 대입되기 때문에 `||`의 short-circuit이 작동하지 않고 항상 둘 다 계산된다.

### 변경

`check_category_band_solid`를 먼저 평가하고, 그 결과가 `false`일 때만 `detect_jacket_edges`를 평가하도록 순서를 바꾼다 (밴드가 더 가볍다는 것이 아니라, 두 게이트 중 하나가 먼저 확정되면 나머지를 생략하자는 것이 핵심이다):

```rust
let gate_ok = check_category_band_solid(frame, jacket_roi, rois.scale())
    || detect_jacket_edges(frame, jacket_roi, rois.scale())
        .map(|edge_strength| edge_strength >= JACKET_EDGE_THRESHOLD)
        .unwrap_or(false);

if gate_ok {
    // ... match_jacket 호출 (기존과 동일)
}
```

세 함수(`detect_result_scene_via_edge`, `detect_freestyle_scene_via_edge`, `detect_openmatch_scene_via_edge`) 모두 동일하게 적용한다. `detect_result_scene_via_edge`의 경우 `edge_ok`/`band_ok` 이후 로직(`debug_println!` 등)에서 두 변수를 각각 참조하는 곳이 없는지 확인하고, 있다면 `gate_ok`로 대체하거나 필요한 부분만 별도 변수로 유지한다.

### 완료 기준

- `cargo build --workspace`, `cargo test --workspace` 통과
- `cargo clippy --all-targets` 통과 (workspace lint: `cognitive_complexity` 상한 55 유지)
- 기존 씬 판별 관련 유닛 테스트(`detection_pipeline.rs` 내 `#[cfg(test)]` 모듈) 전부 그린 — 특히 `stays_detecting_until_hysteresis_activates`, `cached_frames_do_not_repeat_a_scene_miss` 등 씬 전이 타이밍에 의존하는 테스트
- 동작 변화 없음(behavior-preserving): 게이트가 열리는/닫히는 조건(OR 논리)은 동일, 단지 계산 순서/생략만 바뀜

### 리스크

- 낮음. 순수 계산 순서 변경이며 판정 결과(true/false)에는 영향 없음.

---

## Step 2: `match_jacket` 호출 빈도 실측 (Unknown 씬 구간 한정)

### 목적

Step 1은 게이트 자체의 오탐률을 줄이지 않는다. 실제로 게임 플레이 중 `match_jacket()`이 얼마나 자주 호출되는지 데이터 없이는 Step 3(추가 프루닝)의 설계 방향을 정할 수 없다. `OcrTelemetry` 패턴(기존 `RateTelemetry` 참고)을 따라 디버그 빌드 한정 카운터/로그를 추가한다.

### 변경

`DetectionPipeline` 구조체에 디버그 전용 카운터 필드를 추가하거나(권장하지 않음 — 상태 오염), 더 간단하게는 `parse_static_scene` 호출부에서 `match_jacket`이 실제로 실행된 경우에만 `debug_println!`으로 타임스탬프와 호출한 씬 후보(Result/Freestyle/OpenMatch)를 남긴다.

구체적으로 `detect_result_scene_via_edge` / `detect_freestyle_scene_via_edge` / `detect_openmatch_scene_via_edge` 각각에서 `gate_ok`가 `true`가 되어 `match_jacket`을 호출하기 직전에, 그리고 호출 당시의 `last_logo_scene`이 `Unknown`인 경우에 한해:

```rust
debug_println!(
    "    [telemetry] match_jacket triggered while scene=Unknown, candidate={}, now={:.2}",
    "freestyle" /* 또는 "result" / "openmatch" */,
    now
);
```

`now`를 전달받을 수 없는 함수 시그니처라면 파라미터로 추가하지 말고, 대신 이 로깅을 `detect_logo_if_due` 상위 레벨(이미 `now`를 갖고 있음)에서 `parse_static_scene` 반환값과 함께 남기는 것도 대안이다. **가장 침습이 적은 지점을 선택할 것** — 함수 시그니처 변경이 필요하면 범위가 커지므로, 가능하면 기존 `debug_println!` 매크로(release 빌드에서 완전히 컴파일 제외됨, `overmax_engine/src/lib.rs` 참고)만 추가하는 선에서 그친다.

### 측정 방법 (사용자 측 수동 작업, 에이전트는 로그 추가까지만)

1. 디버그 빌드로 실행
2. 곡 하나를 처음부터 끝까지 플레이 (게임 화면, Unknown 씬 유지)
3. 디버그 로그에서 `[telemetry] match_jacket triggered while scene=Unknown` 라인 개수를 집계
4. "분당 호출 횟수"로 환산

### 완료 기준

- `cargo build --workspace` (release 빌드에서 `debug_println!` 완전 제거되어 바이너리 크기/성능 영향 없음 확인)
- 로그 추가 외 다른 로직 변경 없음
- 측정 결과를 다음 형식으로 기록: `곡 재생시간 X분 동안 match_jacket 호출 Y회 (분당 Z회)`

### 다음 단계로의 게이트

**Step 3은 Step 2의 측정 결과를 사용자가 확인하고 명시적으로 승인한 뒤에만 진행한다.** 측정 없이 추측으로 Step 3을 설계하지 않는다 (`CONTEXT.md`의 "진단 우선" 원칙, 과거 multi-monitor 버그에서 3회 오진단 이력과 동일한 이유).

---

## Step 3: 추가 프루닝 (조건부, 보류)

Step 2 측정 결과에 따라 다음 중 하나를 검토한다. **이 계획서 시점에서는 어떤 것도 확정하지 않는다.**

- **호출 빈도가 낮다면(예: 분당 1~2회 이하)**: 이미 리소스 영향이 미미하므로 추가 작업 불필요. Step 1만으로 종료.
- **호출 빈도가 높다면**: 정적 프레임 판별(체크섬 기반 프루닝, `frame_utils::compute_pixel_checksum` 재사용 검토) 또는 게이트 임계치 조정을 검토하되, 반드시 별도 계획서로 재작성하고 재킷 ROI가 실제 게임플레이 화면에서 어떻게 보이는지(스크린샷 기반) 먼저 확인한 뒤 진행한다.

---

## 작업 순서 요약

| 순서 | 작업 | 승인 필요 여부 |
|---|---|---|
| Step 0 | logo ROI 죽은 코드 여부 확인 (정보 수집만) | 불필요 |
| Step 1 | 게이트 short-circuit 패치 | 이 계획서로 사전 승인됨 — 바로 진행 가능 |
| Step 2 | 디버그 텔레메트리 로그 추가 | 이 계획서로 사전 승인됨 — 바로 진행 가능 |
| (측정) | 사용자가 직접 플레이하며 로그 수집 | — |
| Step 3 | 추가 프루닝 설계 | **측정 결과 확인 후 별도 승인 필요** |
