# 🎥 TubeLoader

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

고성능 유튜브 영상 다운로더 - Rust로 작성된 빠르고 안전한 CLI 도구

## ✨ 주요 기능

- 🚀 **비동기 다운로드**: Tokio 기반의 효율적인 비동기 처리
- 🔄 **동시 다운로드**: 여러 영상을 동시에 다운로드 (기본값: 3개)
- 📊 **실시간 진행률**: 다운로드 상태를 실시간으로 확인
- 🎵 **오디오 추출**: 영상에서 오디오만 추출하여 MP3로 저장
- 🎬 **고급 품질 제어**: 5단계 품질 (best, high, medium, low, worst)
- 🎨 **코덱 선택**: 비디오(VP9, AVC1, AV1) 및 오디오(Opus, AAC, MP3) 코덱 선택
- 📁 **커스텀 출력**: 원하는 폴더에 저장
- 🛡️ **안전한 파일명**: 모든 OS에서 호환되는 파일명 자동 변환
- ⚡ **메모리 효율성**: Rust의 zero-cost abstraction 활용
- 🔧 **자동 의존성 관리**: yt-dlp 및 ffmpeg 바이너리 자동 다운로드
- 🛠️ **진단 모드**: 상세한 디버깅 정보 제공
- 🔄 **자동 재시도**: 최대 3회 재시도로 안정성 향상

## 🔧 설치 방법

### 사전 요구사항
- Rust 1.70 이상
- Windows, macOS, Linux 지원
- **추가 설치 불필요**: yt-dlp와 ffmpeg는 첫 실행 시 자동 다운로드됩니다!

### 빌드
```bash
# 저장소 클론
git clone <repository-url>
cd tubeloader

# 의존성 설치 및 빌드
cargo build --release

# 실행 파일 위치: ./target/release/tubeloader
```

## 🚀 사용 방법

### 기본 사용법

```bash
# 단일 영상 다운로드 (첫 실행 시 yt-dlp/ffmpeg 자동 설치)
./target/release/tubeloader "https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# 여러 영상 동시 다운로드
./target/release/tubeloader \
  "https://www.youtube.com/watch?v=dQw4w9WgXcQ" \
  "https://www.youtube.com/watch?v=oHg5SJYRHA0"
```

### 고급 옵션

#### 오디오만 다운로드
```bash
# MP3 파일로 오디오만 추출
./target/release/tubeloader --audio-only "https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# 고품질 AAC 오디오로 다운로드
./target/release/tubeloader \
  --audio-only \
  --audio-quality best \
  --audio-codec aac \
  "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
```

#### 품질 및 코덱 설정
```bash
# 최고 품질 VP9 코덱으로 다운로드
./target/release/tubeloader \
  --quality best \
  --video-codec vp9 \
  "https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# 용량 절약을 위한 저품질 다운로드
./target/release/tubeloader \
  --quality low \
  --audio-quality medium \
  "https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# 특정 코덱 조합으로 다운로드
./target/release/tubeloader \
  --quality high \
  --audio-quality best \
  --video-codec av1 \
  --audio-codec opus \
  "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
```

#### 출력 폴더 지정
```bash
# 특정 폴더에 저장
./target/release/tubeloader --output "./my-videos" "https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# 절대 경로 사용
./target/release/tubeloader --output "C:/Videos" "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
```

#### 동시 다운로드 수 조정
```bash
# 5개 영상을 동시에 다운로드
./target/release/tubeloader --concurrent 5 \
  "https://www.youtube.com/watch?v=video1" \
  "https://www.youtube.com/watch?v=video2" \
  "https://www.youtube.com/watch?v=video3" \
  "https://www.youtube.com/watch?v=video4" \
  "https://www.youtube.com/watch?v=video5"
```

#### 진단 모드
```bash
# 상세한 디버깅 정보와 함께 실행
./target/release/tubeloader --verbose "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
```

### 복합 사용 예시

```bash
# 음악 플레이리스트를 고품질 Opus로 추출
./target/release/tubeloader \
  --audio-only \
  --audio-quality best \
  --audio-codec opus \
  --output "./music" \
  --concurrent 2 \
  --verbose \
  "https://www.youtube.com/watch?v=song1" \
  "https://www.youtube.com/watch?v=song2" \
  "https://www.youtube.com/watch?v=song3"

# 고품질 영상 컬렉션 다운로드
./target/release/tubeloader \
  --quality best \
  --audio-quality high \
  --video-codec vp9 \
  --audio-codec opus \
  --output "./collection" \
  --concurrent 3 \
  "https://www.youtube.com/watch?v=video1" \
  "https://www.youtube.com/watch?v=video2"
```

## 📋 옵션 상세 설명

### 명령줄 옵션

| 옵션 | 단축형 | 기본값 | 설명 |
|------|--------|--------|------|
| `--quality` | `-q` | `high` | 영상 품질 (`best`, `high`, `medium`, `low`, `worst`) |
| `--audio-quality` | | `high` | 오디오 품질 (`best`, `high`, `medium`, `low`, `worst`) |
| `--video-codec` | | `any` | 비디오 코덱 (`vp9`, `avc1`, `av1`, `any`) |
| `--audio-codec` | | `any` | 오디오 코덱 (`opus`, `aac`, `mp3`, `any`) |
| `--output` | `-o` | `./downloads` | 다운로드 폴더 경로 |
| `--concurrent` | `-c` | `3` | 동시 다운로드할 영상 수 |
| `--audio-only` | `-a` | 없음 | 오디오만 다운로드 (MP3) |
| `--verbose` | | 없음 | 진단 정보 출력 |
| `--help` | `-h` | 없음 | 도움말 표시 |

### 품질 설정 가이드

#### 비디오 품질
- `best`: 가장 높은 화질 (4K, 1440p 등)
- `high`: 고화질 (1080p 목표)
- `medium`: 중간 화질 (720p 목표)
- `low`: 저화질 (480p 목표)
- `worst`: 가장 낮은 화질

#### 오디오 품질
- `best`: 최고 음질 (320kbps+)
- `high`: 고음질 (192kbps 목표)
- `medium`: 중간 음질 (128kbps 목표)
- `low`: 저음질 (96kbps 목표)
- `worst`: 최저 음질

#### 코덱 추천
- **VP9**: 효율적인 압축, 좋은 화질 (추천)
- **AV1**: 최신 코덱, 최고 효율성 (호환성 주의)
- **AVC1/H.264**: 가장 널리 지원되는 코덱
- **Opus**: 최고 품질의 오디오 코덱 (추천)
- **AAC**: 널리 지원되는 고품질 오디오
- **MP3**: 범용적 호환성

### 지원하는 URL 형식

- `https://www.youtube.com/watch?v=VIDEO_ID`
- `https://youtu.be/VIDEO_ID`
- `https://m.youtube.com/watch?v=VIDEO_ID`
- `VIDEO_ID` (11자리 ID만 입력 시 자동 완성)

## 📊 사용 예시 출력

```
🎥 TubeLoader - 유튜브 영상 다운로더
📅 라이브러리: yt-dlp 크레이트 v1.3.4
📁 다운로드 폴더: ./downloads
⚡ 동시 다운로드 수: 3
🔧 yt-dlp 및 ffmpeg 바이너리를 준비하는 중...
✅ 바이너리 준비 완료!
✅ 유효한 URL: https://www.youtube.com/watch?v=dQw4w9WgXcQ
📋 1 개의 영상을 다운로드합니다...

[1] 영상 정보를 가져오는 중: https://www.youtube.com/watch?v=dQw4w9WgXcQ
[1] 다운로드 시작: Rick Astley - Never Gonna Give You Up
[████████████████████████████████████████] 100% ([1] Rick Astley - Never Gonna Give You Up)
[1] ✅ 완료: Rick Astley - Never Gonna Give You Up

📊 다운로드 결과:
  성공: 1 개

✅ 성공한 다운로드 목록:
  1. Rick Astley - Never Gonna Give You Up
     📂 저장 위치: downloads\Rick Astley - Never Gonna Give You Up.mp4

✅ 모든 다운로드가 완료되었습니다!
```

## 🏗️ 기술 스택

- **언어**: Rust 2021 Edition
- **비동기 런타임**: Tokio
- **CLI 파싱**: clap
- **YouTube 처리**: yt-dlp 크레이트 (yt-dlp 바이너리 래퍼)
- **진행률 표시**: indicatif
- **에러 처리**: anyhow, thiserror
- **자동 의존성**: yt-dlp 및 ffmpeg 바이너리 자동 다운로드

## 🛡️ "Video source empty" 문제 해결

새로운 `yt-dlp` 크레이트 기반으로 **"Video source empty"** 오류를 대폭 개선했습니다:

### ✅ 해결된 문제들
- **지역 제한**: yt-dlp의 우수한 지역 제한 우회 기능
- **연령 제한**: 더 나은 연령 제한 콘텐츠 처리
- **라이브 스트림**: 라이브 콘텐츠 감지 및 안내
- **프리미어 영상**: 프리미어 상태 감지
- **삭제된 영상**: 명확한 오류 메시지

### 🔄 자동 재시도 시스템
- 최대 **3회 자동 재시도**
- 재시도 불가능한 오류 자동 감지
- 2초 간격으로 지능적 재시도

### 🔍 상세한 오류 분석
```bash
# 진단 모드로 상세 정보 확인
./target/release/tubeloader --verbose "문제있는URL"
```

## 🚨 주의사항

- 저작권이 있는 콘텐츠의 다운로드는 해당 법률을 준수해야 합니다
- 개인적인 용도로만 사용하세요
- YouTube의 서비스 약관을 확인하세요
- 네트워크 상태에 따라 다운로드 속도가 달라질 수 있습니다
- 첫 실행 시 yt-dlp와 ffmpeg 다운로드로 인해 시간이 걸릴 수 있습니다

## 🐛 문제 해결

### 일반적인 오류

1. **"영상 정보를 가져올 수 없습니다"**
   - URL이 올바른지 확인
   - `--verbose` 옵션으로 상세 정보 확인
   - 영상이 비공개/삭제되었는지 확인

2. **"Video source empty" 또는 "No formats"**
   - 지역 제한 또는 연령 제한일 가능성
   - 다른 품질로 시도: `--quality medium`
   - 자동 재시도가 작동하므로 잠시 대기

3. **"바이너리 준비 실패"**
   - 인터넷 연결 확인
   - 방화벽 설정 확인
   - `libs` 폴더 권한 확인

4. **다운로드 속도가 느림**
   - 동시 다운로드 수 조정: `--concurrent 1`
   - 네트워크 상태 확인
   - 다른 품질로 시도

### 고급 문제 해결

```bash
# 1. 진단 모드로 상세 정보 확인
./target/release/tubeloader --verbose "문제URL"

# 2. 저품질로 테스트
./target/release/tubeloader --quality low "문제URL"

# 3. 오디오만 테스트
./target/release/tubeloader --audio-only "문제URL"

# 4. 단일 다운로드로 테스트
./target/release/tubeloader --concurrent 1 "문제URL"
```

## 🔄 업데이트

새로운 기능이나 버그 수정을 위해 주기적으로 업데이트됩니다:

```bash
# 최신 코드로 업데이트
git pull origin main
cargo build --release

# 의존성 업데이트 (선택사항)
cargo update
```

### yt-dlp 바이너리 업데이트
프로그램이 자동으로 최신 yt-dlp를 다운로드하지만, 수동 업데이트도 가능합니다:
```bash
# libs 폴더 삭제 후 재실행하면 최신 버전 다운로드
rm -rf libs
./target/release/tubeloader "테스트URL"
```

## 🤝 기여하기

1. Fork 생성
2. Feature 브랜치 생성 (`git checkout -b feature/amazing-feature`)
3. 변경사항 커밋 (`git commit -m 'Add amazing feature'`)
4. 브랜치에 Push (`git push origin feature/amazing-feature`)
5. Pull Request 생성

### 개발 가이드라인
- Rust 최신 안정 버전 사용
- `cargo fmt`로 코드 포맷팅
- `cargo clippy`로 정적 분석
- 새로운 기능에 대한 테스트 추가

## 📜 라이선스

이 프로젝트는 MIT 라이선스 하에 배포됩니다. 자세한 내용은 [LICENSE](LICENSE) 파일을 참조하세요.

## 🙏 감사의 말

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) - 강력한 YouTube 다운로더
- [yt-dlp Rust 크레이트](https://crates.io/crates/yt-dlp) - Rust 래퍼 제공
- [tokio](https://tokio.rs/) - 비동기 런타임
- [clap](https://github.com/clap-rs/clap) - CLI 인터페이스
- [indicatif](https://github.com/console-rs/indicatif) - 진행률 표시

## 🚀 성능 벤치마크

| 기능 | 이전 (rusty-ytdl) | 현재 (yt-dlp) | 개선율 |
|------|------------------|---------------|--------|
| **성공률** | ~70% | ~95% | 🔥 +25% |
| **지역 제한 영상** | ❌ 실패 | ✅ 성공 | 🎯 100% |
| **오류 복구** | ❌ 없음 | ✅ 3회 재시도 | 🛡️ 안정성 향상 |
| **의존성 관리** | 🔧 수동 | 🤖 자동 | 📦 편의성 향상 |

---

**⭐ 이 프로젝트가 유용하다면 Star를 눌러주세요!**

**🐛 문제가 발생했나요?** [Issues](../../issues)에서 도움을 받으세요.

**💡 새로운 아이디어가 있나요?** [Discussions](../../discussions)에서 공유해주세요. 