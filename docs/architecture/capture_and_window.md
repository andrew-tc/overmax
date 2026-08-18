# Capture and Window Subsystem Architecture

이 문서는 Overmax의 화면 캡처 엔진(DXGI, GDI, XComposite), 윈도우 위치 추적, Per-Monitor DPI 인식 및 다중 모니터 좌표 매핑 아키텍처를 설명한다.

---

## 1. 서브시스템 개요 및 제약 조건

화면 캡처 및 윈도우 추적 모듈은 `rust/overmax_engine/src/capture/`에 위치하며 다음 제약을 만족하도록 설계되었다.

1. **Zero Process Injection**: 대상 게임 프로세스의 메모리에 접근하지 않고 OS가 제공하는 캡처 API와 윈도우 관리 API만 사용한다.
2. **최소 CPU/GPU 오버헤드**: 게임 플레이 중 프레임 드랍을 유발하지 않도록 적응형 폴링과 효율적인 캡처 백엔드를 선택한다.
3. **DPI 가상화 왜곡 차단**: 모니터별 서로 다른 배율 설정(DPI) 환경에서도 픽셀 좌표가 1:1로 일치해야 한다.

---

## 2. 캡처 백엔드 구조 및 적응형 전환 (`AdaptiveCaptureEngine`)

```
                        [ WindowTracker ]
                               │
                (게임 창 크기 / 전체화면 여부 판단)
                               │
                               ▼
                   [ AdaptiveCaptureEngine ]
                               │
            ┌──────────────────┴──────────────────┐
            ▼                                     ▼
   [ DXGICaptureEngine ]                  [ GDICaptureEngine ]
  - Desktop Duplication API              - Win32 GDI BitBlt
  - GPU 텍스처 복사                      - CPU 메모리 복사
  - 전체화면 / Borderless 지원           - 창모드 / 다중모니터 안정성
  - 고성능, 초저지연 (0~2ms)             - 호환성 우수 (4~8ms)
```

### 1) DXGI Desktop Duplication (`rust/overmax_engine/src/capture/capture_engine/dxgi.rs`)
* DirectX 11 Desktop Duplication API(`IDXGIOutputDuplication`)를 활용하여 GPU 프레임버퍼에서 직접 화면을 캡처한다.
* 전체화면(Borderless/Exclusive) 환경에서 CPU 부하 없이 고속(1~2ms 이내)으로 프레임을 획득한다.
* 다중 모니터 환경에서는 게임 창의 중심 좌표를 계산하여 해당 창이 위치한 활성 DXGI 디스플레이(`IDXGIOutput1`)를 동적으로 탐색하고, 전체 데스크톱 좌표계를 로컬 모니터 기준 오프셋으로 변환하여 크롭한다.

### 2) Win32 GDI 캡처 (`rust/overmax_engine/src/capture/capture_engine/gdi.rs`)
* Win32 `BitBlt` 및 `GetDIBits`를 사용하여 게임 윈도우의 클라이언트 DC(Device Context)로부터 픽셀 데이터를 가져온다.
* 창모드 구동 시 다른 창에 가려지지 않은 영역을 안정적으로 캡처하며, 다양한 GPU/모니터 구성에서 높은 호환성을 제공한다.

### 3) 적응형 자동 선택 (`Auto` 모드) 및 실패 복구 정책
* 사용자가 캡처 방식으로 `Auto`를 선택한 경우:
  * 단일 모니터 및 전체화면 환경: **DXGI 백엔드** 우선 사용.
  * 창모드 환경 또는 다중 모니터 특정 구성: **GDI 백엔드** 자동 전환.
* DXGI 캡처 중 장치 분실(`DXGI_ERROR_DEVICE_REMOVED`, 모니터 해상도 변경 등)이 발생하면 즉시 GDI로 폴백하고 3초간 DXGI 재시도를 지연(쿨다운)하여 불필요한 시스템 콜 낭비를 차단한다.

### 4) Linux XComposite / MIT-SHM (`rust/overmax_engine/src/capture/capture_engine/linux.rs`)
* Linux X11/XWayland 환경에서는 XComposite 확장을 통해 대상 게임 창의 Pixmap을 획득하고, XShm(MIT-SHM) 공유 메모리를 활용해 오버헤드 없이 BGRA 프레임을 캡처한다.
* XWayland가 map/resize 이벤트 없이 backing pixmap을 내부적으로 교체할 때 이전 핸들이 멈춘 프레임(Frozen Frame)을 반환하는 문제를 차단하기 위해, 매 캡처마다 named pixmap 핸들을 안전하게 재획득하여 수명주기를 관리한다.

---

## 3. Per-Monitor DPI Aware V2 및 물리 좌표계 매핑

서로 다른 배율(예: 주 모니터 150%, 보조 모니터 100%)을 가진 환경에서 Win32 DPI 가상화가 개입하면 캡처 영역이 어긋나거나 흐려지는 문제가 발생한다.

### 1) DPI Aware V2 컨텍스트 선언
* 앱 초기화 시 `SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)`를 선언하여 OS 수준의 좌표 가상화 스케일링을 비활성화한다.
* 캡처 엔진과 윈도우 추적기는 항상 **물리 픽셀(Physical Pixel)** 좌표계를 기준으로 연산한다.

### 2) DWM 확장 프레임 경계 계산 (`WindowTracker`)
* 일반 `GetWindowRect`는 보이지 않는 투명 윈도우 그림자 테두리(약 7~8px)를 포함하므로, Win32 DWM API를 사용하여 실제 렌더링 영역을 계산한다.
  ```rust
  DwmGetWindowAttribute(
      hwnd,
      DWMWA_EXTENDED_FRAME_BOUNDS,
      &mut rect,
      size_of::<RECT>() as u32,
  );
  ```
* 창모드 캡처 시 `ClientToScreen`을 통해 타이틀바와 프레임을 제외한 순수 인게임 클라이언트 영역(1920x1080 등)의 물리 좌표만 정확히 크롭한다.

---

## 4. 윈도우 추적 및 동적 스케줄링 (`WindowTracker`)

게임 실행 상태와 창의 움직임에 따라 시스템 자원 소모를 최소화하기 위해 3단계 동적 폴링 주기를 적용한다.

```
[ 게임 창 미발견 ] ──(1000ms 주기)──► [ 대기 상태 (CPU 0%) ]
       │
[ 게임 창 발견 (정지 상태) ] ──(300ms 주기)──► [ 일반 디텍션 틱 ]
       │
[ 창 드래그 / 이동 감지 ] ──(16ms / 60FPS)──► [ 실시간 오버레이 스냅 추적 ]
```

1. **대기 주기 (1000ms)**:
   게임 프로세스(`DJMAX RESPECT V`)가 실행되지 않았거나 창 핸들을 찾지 못한 경우, 1초에 한 번만 `FindWindowW`를 호출하여 CPU 사용량을 0%로 유지한다.
2. **정지 주기 (300ms)**:
   창이 정지된 상태에서는 300ms 간격으로 위치 변화 여부만 검사하여 불필요한 Win32 API 호출을 줄인다.
3. **추적 주기 (16ms)**:
   사용자가 창모드 상태에서 게임 창을 드래그하여 이동하는 중에는 60FPS(16ms) 주기로 즉시 전환하여 오버레이 창이 이질감 없이 게임 창 모서리에 밀착되도록 추적한다.

---

## 5. 오버레이 Z-Order 및 창 스냅

1. **DWM 최상위 Z-Order 유지**:
   오버레이 창은 `SetWindowPos(HWND_TOPMOST, ..., SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE)`를 사용하여 게임 창 위에 항상 표시되도록 유지하되, 포커스를 빼앗지 않아 게임 입력(키보드/컨트롤러)을 방해하지 않는다.
2. **모서리 자동 스냅**:
   게임 창의 물리 좌표(`ExtendedFrameBounds`)를 기준으로 설정된 오프셋 및 앵커 방향(상단, 하단, 좌측, 우측)에 맞춰 오버레이 창의 위치를 자동 계산하여 배치한다.
