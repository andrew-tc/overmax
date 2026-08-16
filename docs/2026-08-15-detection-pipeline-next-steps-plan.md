# 디텍션 파이프라인 잔여 작업 구현 계획

**작성일**: 2026-08-15  
**목표**: 게임 프레임 드랍을 우선적으로 막으면서, 현재 인식 품질과 반응 지연을 유지하고 향후 서로 다른 ROI의 씬을 안전하게 추가한다.  
**선행 완료**: Rate/Score checksum 캐시 복구, Freestyle/OpenMatch 후보 내 재킷 버퍼 재사용.  
**범위 밖**: 메모리 접근, 프로세스 인젝션, 캡처 엔진 교체, OCR 다중 패스, DB/사용자 설정 형식 변경.

---

## 1. 문제 정의

현재 비용 절감은 안전한 중복 계산 두 곳에 적용됐다. 남은 판단은 다음 세 질문에 대한 **실측**이 필요하다.

1. 실제 게임 플레이 중 CPU 시간을 가장 많이 쓰는 단계가 캡처인지, 씬 판별인지, 세부 상태 판별인지.
2. 새 씬 후보가 추가될 때 Unknown 구간의 비용이 현재보다 얼마나 증가하는지.
3. 정적 화면에서 추가 캐시가 필요한 ROI가 실제로 존재하는지.

측정 전에는 `parse_static_scene()`을 범용 레지스트리로 재작성하거나, 임계치·폴링 주기·확정 규칙을 바꾸지 않는다. 현재 후보 수에서는 그 변경의 유지보수 비용이 더 크다.

## 2. 원인 및 현재 제약

| 영역 | 현재 보호 장치 | 남은 위험 |
| --- | --- | --- |
| 캡처 | 재사용 프레임 버퍼, foreground/idle sleep, GDI/DXGI 선택 | 전체 창 캡처가 병목일 수 있음 |
| 씬 판별 | 0.3~2.0초 cooldown, centroid gate, category band, 재킷 매칭 | 서로 다른 ROI의 후보가 늘면 Unknown 구간 비용이 후보 수에 비례 |
| 세부 상태 | Rate/Score checksum, 200ms 1-pass, 5초 강제 검증 | Mode/Difficulty/Max Combo는 활성 씬에서 매 tick 실행 |
| 정확도 | WTA, scene hysteresis, `PlayContext` history commit | 캐시 무효화 실수는 stale state 또는 지연을 초래 가능 |

변경 중에도 다음 불변 조건을 유지한다.

- Rate/Score 템플릿 매칭은 한 번의 1-pass만 수행한다.
- `GameSessionState.is_stable` 전에는 기록하지 않는다.
- `song_id == 0`과 `Option::None`의 구분을 유지한다.
- window resize, capture interruption, scene exit에서는 관련 캐시를 즉시 무효화한다.

---

## 3. 구현 단계

### Phase A — 저오버헤드 단계별 시간 측정

**목적**: 다음 최적화의 대상과 우선순위를 사실로 결정한다.

**대상 파일**:

- `rust/overmax_engine/src/detector/detection_worker.rs`
- `rust/overmax_engine/src/detector/detection_pipeline.rs`
- 필요 시 `rust/overmax_app/src/ui/debug_ui.rs`

**구현**:

1. worker에서 `capture`, `pipeline.detect`, 결과 channel 전송의 elapsed time을 측정한다.
2. pipeline은 `scene`, `jacket`, `play_state` 구간의 elapsed time만 별도 수집한다. 각 구간은 이미 존재하는 함수 경계에 한해 측정하며, 로직 분해를 위한 리팩터링은 하지 않는다.
3. 매 프레임 결과를 채널 또는 UI로 보내지 않는다. worker 내부에 count/total/max만 누적하고, 디버그 창이 열려 있을 때만 5초에 한 번 snapshot을 전송한다.
4. snapshot에는 평균·최대 시간, 활성/Unknown 프레임 수, `match_jacket` 실행 횟수만 포함한다. 문자열 포맷, 벡터 할당, per-frame 로그는 금지한다.
5. debug UI가 닫힌 상태의 측정 경로는 `Instant::now()`와 정수 누적 외 추가 할당이 없도록 한다. release에서 telemetry UI 전송은 비활성화한다.

**검증**:

- 단위 테스트: 5초 집계 경계, 최대/평균 산출, reset 동작.
- 수동 측정: Freestyle, OpenMatch, 결과창, 실제 플레이(Unknown)에서 각 2분 이상 실행.
- 기록 형식: 캡처 엔진, 해상도, 평균/최대 capture·scene·jacket·play-state ms, 분당 `match_jacket` 횟수.

**완료 기준**: 측정 기능을 켠 상태에서도 감지 주기와 `DetectionOutput` 의미가 바뀌지 않고, debug UI가 닫혀 있을 때 UI repaint 빈도가 증가하지 않는다.

### Phase B — 측정 결과에 따른 단일 병목 개선

Phase A 결과가 나온 뒤 **하나의** 경로만 선택해 별도 커밋 묶음으로 진행한다.

#### B1. 캡처가 우세한 경우

1. 같은 게임·해상도에서 GDI와 DXGI를 비교한다.
2. 안정성과 frame freshness가 동등할 때만 더 낮은 CPU 경로를 기본 추천으로 문서화한다.
3. 기본값 변경은 실제 캡처 실패율과 사용자 환경 검증이 충분할 때만 검토한다. 캡처 엔진 교체는 이 계획의 자동 범위가 아니다.

#### B2. 씬 판별이 우세한 경우

1. Unknown 구간에서 후보별 gate 통과율과 `match_jacket` 호출 수를 비교한다.
2. 같은 ROI를 공유하는 새 후보가 생길 때만, ROI crop·thumbnail·hash를 해당 후보 그룹 내에서 공유한다.
3. 서로 다른 ROI 후보는 기존 후보의 판정 순서와 WTA 규칙을 유지한다. 실제 화면 앵커가 없으면 후보를 추가하지 않는다.
4. 임계치 변경은 false positive/false negative가 포함된 캡처 fixture로 회귀 테스트가 준비된 경우에만 별도 변경으로 진행한다.

#### B3. PlayState가 우세한 경우

1. Mode, Difficulty, Max Combo ROI 각각에 checksum을 시범 적용하되, Rate/Score와 달리 한 번에 모두 캐시하지 않는다.
2. 첫 대상은 값이 정적인 선곡창 Mode/Difficulty로 한정한다. Max Combo는 결과 연출 지연의 영향을 받으므로, 측정과 fixture가 준비되기 전에는 캐시하지 않는다.
3. checksum 변경, scene 변경, song 변경, resize, reset에서 즉시 재검사하며 5초 강제 검증을 둔다.
4. `PlayContext` history 테스트와 실제 결과창 Max Combo 기록 회귀를 통과해야 한다.

**선택 기준**: 평균 시간이 아니라 p95 또는 max spike가 게임 프레임 드랍과 함께 관측된 단계를 우선한다.

### Phase C — 새 씬 추가 절차

새 ROI 씬은 아래 순서를 지켜 하나씩 추가한다. 이는 현재 후보가 적은 상태에서 추상화를 먼저 만드는 대신, 씬별 차이를 명시적으로 검증하기 위한 절차다.

1. **입력 수집**: 목표 해상도마다 진입·정상·전환·실패 화면을 최소 10장 수집한다. 민감 정보는 포함하지 않는다.
2. **ROI 명세**: `rust/overmax_data/src/config/scene_config.rs`에 ROI를 추가하고 16:9, letterbox, windowed 변환 테스트를 작성한다.
3. **고유 gate**: 재킷 매칭 전에 동작하는 저비용 앵커 한 개 이상을 정의한다. 다른 씬과 재킷 ROI를 공유하면 그 앵커가 씬을 구분해야 한다.
4. **후보 함수**: `detection_pipeline.rs`에 최소 함수로 추가하고, 기존 후보의 순서 및 WTA 의미를 변경하지 않는다. 결과창이면 2프레임 `commit_result_scene` 규칙을 그대로 따른다.
5. **상태 판독**: Mode/Difficulty/Rate/Score/Max Combo 중 실제로 보이는 것만 추가한다. 보이지 않는 필드는 `None`을 유지해 verified=False가 되게 한다.
6. **회귀 검증**: 새 fixture뿐 아니라 Freestyle/OpenMatch/결과창 fixture 전부에서 scene, song ID, stable 여부를 비교한다.
7. **실게임 검증**: Phase A 지표로 신규 씬 추가 전후 CPU·latency를 비교하고, 기준을 넘으면 후보 gate를 보강한 뒤 재측정한다.

세 번째 이상으로 서로 다른 ROI의 씬 후보가 추가되어 중복 코드와 측정상 비용 증가가 확인될 때에만 `SceneCandidate` 데이터 구조를 검토한다. 그 전에는 현 함수 단위 구현이 더 예측 가능하고 검토가 쉽다.

### Phase D — 저수준 zero-copy 확장 (조건부)

**시작 조건**: Phase A에서 `ImageView::to_image_region()` 복사가 scene 또는 play-state p95의 유의미한 비중임이 확인된 경우.

1. `overmax_cv`에 stride-aware 입력 API를 하나씩 추가한다. 기존 연속 버퍼 API는 하위 호환을 위해 보존한다.
2. 첫 대상은 60x60 재킷의 histogram/centroid 경로 한 개로 제한한다.
3. 기존 입력과 stride 입력의 hash/histogram 결과 동치 테스트를 이미지 fixture로 작성한다.
4. 벤치마크에서 이득과 결과 동치를 확인한 뒤에만 `match_jacket` 전체 경로로 확장한다.

이 단계는 `overmax_engine`과 `overmax_cv`를 함께 바꾸므로, 시작 전 별도 승인을 받는다.

---

## 4. 커밋 및 검증 단위

| 커밋 | 변경 범위 | 필수 검증 |
| --- | --- | --- |
| 1 | Phase A 집계 자료구조 및 worker 측정 | engine unit test, `cargo clippy --all-targets` |
| 2 | Phase A debug UI 노출(필요할 때만) | app unit test, debug UI 수동 확인 |
| 3 | Phase B에서 선택한 단일 병목 개선 | 관련 fixture + `cargo test --workspace` |
| 4 | 새 씬 하나 | 해당 씬 fixture + 기존 씬 회귀 + 실제 측정 비교 |
| 5 | 문서/CONTEXT/TASKS 갱신 | `cargo fmt`, `cargo clippy --fix`, `cargo test --workspace` |

각 커밋은 behaviour-preserving 최적화와 인식 규칙 변경을 섞지 않는다. 인식 규칙 변경은 반드시 fixture와 함께 독립 커밋으로 남긴다.

## 5. 추천 순서

1. **Phase A**를 먼저 완료해 병목과 baseline을 기록한다.
2. 측정 결과로 **Phase B의 한 갈래만** 선택한다.
3. 신규 씬이 실제로 필요해지면 **Phase C** 절차와 fixture를 먼저 준비한다.
4. zero-copy CV API 확장은 **Phase D의 시작 조건**이 충족될 때만 검토한다.

이 순서는 성능을 추측으로 최적화하지 않으면서, 새 씬 추가 비용을 통제하고 현재 verified pipeline을 유지한다.
