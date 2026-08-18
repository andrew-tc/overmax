# overmax_cv

Pure Rust 기반의 경량 컴퓨터 비전 및 이미지 해시/지문(Fingerprint) 알고리즘 라이브러리입니다.
외부 무거운 CV 라이브러리(OpenCV 등)나 OS 의존성 없이 표준 Rust만으로 고성능 이미지 프로세싱을 수행합니다.

---

## 🚀 주요 기능 및 알고리즘

- **Perceptual Image Hashing**:
  - `aHash` (Average Hash, 8×8)
  - `dHash` (Difference Hash, 9×8)
  - `pHash` (Perceptual Hash, 32×32 2D-DCT 기반)
- **HOG (Histogram of Oriented Gradients)**:
  - 자켓 이미지 매칭용 1,764차원 / 3,780차원 HOG Feature 추출
  - Gaussian Block Weighting & L2-Hys Normalization
- **4×4 Grid RGB Histogram**:
  - 16개 격자 영역별 8-bin R/G/B 색상 분포 추출 (총 384바이트 L1 정규화 벡터)
- **Zero-Allocation Template Matching Engine**:
  - `[u32; 32]` 비트마스크 패킹 및 CPU `count_ones()`(Popcount) 기반 초고속 템플릿 매칭
  - 문자 분할(`segment_characters`) 및 리사이즈(`resize_binary_nearest_into`) 스택 버퍼 최적화
- **Binarization & Contrast**:
  - Global Contrast / Luminance 기반 고속 이진화
  - Bradley-Roth 적응형 임계처리 (Adaptive Thresholding)

---

## 🛠️ 빌드 및 테스트

순수 Rust 라이브러리로 별도의 외부 환경 설정 없이 Cargo로 즉시 검증할 수 있습니다.

```powershell
# 단위 테스트 실행
cargo test -p overmax_cv
```
