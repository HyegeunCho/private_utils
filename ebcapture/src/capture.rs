use crate::error::{EbCaptureError, Result};
use crate::window_manager::{WindowInfo, WindowRect};
use log::{info, debug, warn};
use scrap::{Capturer, Display};
use image::{ImageBuffer, RgbaImage, DynamicImage};
use std::path::Path;
use std::thread;
use std::time::Duration;

#[cfg(windows)]
use winapi::um::wingdi::*;
#[cfg(windows)]
use winapi::um::winuser::*;
#[cfg(windows)]
use winapi::shared::windef::*;

#[derive(Debug, Clone)]
enum PixelFormat {
    Bgra,  // 4 bytes per pixel: B, G, R, A
    Rgba,  // 4 bytes per pixel: R, G, B, A
    Bgr,   // 3 bytes per pixel: B, G, R
    Rgb,   // 3 bytes per pixel: R, G, B
}

impl PixelFormat {
    fn bytes_per_pixel(&self) -> usize {
        match self {
            PixelFormat::Bgra | PixelFormat::Rgba => 4,
            PixelFormat::Bgr | PixelFormat::Rgb => 3,
        }
    }
}

/// 지정된 윈도우의 화면을 직접 캡쳐합니다 (PrintWindow API 사용)
pub async fn capture_window(window: &WindowInfo, window_rect: &WindowRect, output_path: &Path) -> Result<()> {
    info!("🎯 윈도우 직접 캡쳐 시작: {} ({}x{})", 
        window.title, window_rect.width, window_rect.height);
    
    // Windows에서 직접 윈도우 캡쳐 시도
    #[cfg(windows)]
    {
        match capture_window_direct_windows(window, window_rect).await {
            Ok(image) => {
                image.save(output_path).map_err(|e| {
                    EbCaptureError::CaptureFailure { 
                        reason: format!("직접 캡쳐 이미지 저장 실패: {}", e) 
                    }
                })?;
                info!("✅ 윈도우 직접 캡쳐 완료: {}", output_path.display());
                return Ok(());
            }
            Err(e) => {
                warn!("직접 캡쳐 실패, 전체 화면 캡쳐로 대체: {}", e);
            }
        }
    }
    
    // 직접 캡쳐 실패시 기존 방식으로 대체
    info!("🔄 전체 화면 캡쳐 방식으로 대체 실행");
    let full_screen_image = capture_full_screen_to_image().await?;
    let cropped_image = crop_image_to_window(full_screen_image, window_rect)?;
    
    cropped_image.save(output_path).map_err(|e| {
        EbCaptureError::CaptureFailure { 
            reason: format!("대체 방식 이미지 저장 실패: {}", e) 
        }
    })?;
    
    info!("✅ 윈도우 캡쳐 완료 (대체 방식): {}", output_path.display());
    Ok(())
}

/// 전체 화면을 캡쳐합니다
pub async fn capture_full_screen(output_path: &Path) -> Result<()> {
    debug!("전체 화면 캡쳐 시작");
    
    let display = Display::primary().map_err(|e| {
        EbCaptureError::CaptureFailure { 
            reason: format!("주 디스플레이 가져오기 실패: {}", e) 
        }
    })?;
    
    let mut capturer = Capturer::new(display).map_err(|e| {
        EbCaptureError::CaptureFailure { 
            reason: format!("캡쳐러 생성 실패: {}", e) 
        }
    })?;
    
    let width = capturer.width();
    let height = capturer.height();
    debug!("캡쳐 해상도: {}x{}", width, height);
    
    // 첫 번째 프레임 건너뛰기 (보통 비어있음)
    let _ = capturer.frame();
    thread::sleep(Duration::from_millis(100));
    
    // 재시도 로직 추가
    let mut attempts = 0;
    let max_attempts = 3;
    
    while attempts < max_attempts {
        attempts += 1;
        debug!("캡쳐 시도 {}/{}", attempts, max_attempts);
        
        match capturer.frame() {
            Ok(frame) => {
                debug!("프레임 데이터 크기: {} bytes", frame.len());
                
                match save_frame_as_image_smart(&frame, width, height, output_path).await {
                    Ok(_) => {
                        info!("전체 화면 캡쳐 완료: {}", output_path.display());
                        return Ok(());
                    }
                    Err(e) => {
                        warn!("이미지 저장 실패 (시도 {}): {}", attempts, e);
                        if attempts == max_attempts {
                            return Err(e);
                        }
                        thread::sleep(Duration::from_millis(500));
                    }
                }
            }
            Err(e) => {
                warn!("프레임 캡쳐 실패 (시도 {}): {}", attempts, e);
                if attempts == max_attempts {
                    return Err(EbCaptureError::CaptureFailure { 
                        reason: format!("화면 캡쳐 실패: {}", e) 
                    });
                }
                thread::sleep(Duration::from_millis(500));
            }
        }
    }
    
    Err(EbCaptureError::CaptureFailure { 
        reason: "최대 재시도 횟수 초과".to_string() 
    })
}

/// 전체 화면을 캡쳐하여 DynamicImage로 반환합니다
async fn capture_full_screen_to_image() -> Result<DynamicImage> {
    info!("📺 전체 화면 캡쳐 시작");
    
    let display = Display::primary().map_err(|e| {
        EbCaptureError::CaptureFailure { 
            reason: format!("주 디스플레이 가져오기 실패: {}", e) 
        }
    })?;
    
    let mut capturer = Capturer::new(display).map_err(|e| {
        EbCaptureError::CaptureFailure { 
            reason: format!("캡쳐러 생성 실패: {}", e) 
        }
    })?;
    
    let width = capturer.width();
    let height = capturer.height();
    info!("화면 해상도: {}x{}", width, height);
    
    // 첫 번째 프레임 건너뛰기
    let _ = capturer.frame();
    thread::sleep(Duration::from_millis(100));
    
    // 간단한 재시도 로직
    for attempt in 1..=3 {
        match capturer.frame() {
            Ok(frame) => {
                info!("📥 프레임 획득: {} bytes", frame.len());
                
                if !frame.is_empty() {
                    return convert_frame_to_image(&frame, width, height);
                }
            }
            Err(e) => {
                warn!("캡쳐 시도 {}/3 실패: {}", attempt, e);
            }
        }
        thread::sleep(Duration::from_millis(500));
    }
    
    Err(EbCaptureError::CaptureFailure { 
        reason: "전체 화면 캡쳐 실패".to_string() 
    })
}

/// 프레임 데이터를 DynamicImage로 변환합니다 (단순화된 버전)
fn convert_frame_to_image(frame: &[u8], width: usize, height: usize) -> Result<DynamicImage> {
    let expected_bgra = width * height * 4;
    
    if frame.len() == expected_bgra {
        // BGRA → RGBA 변환
        let mut rgba_data = Vec::with_capacity(frame.len());
        
        for chunk in frame.chunks_exact(4) {
            rgba_data.push(chunk[2]); // R
            rgba_data.push(chunk[1]); // G
            rgba_data.push(chunk[0]); // B
            rgba_data.push(chunk[3]); // A
        }
        
        let img = ImageBuffer::from_raw(width as u32, height as u32, rgba_data)
            .ok_or_else(|| EbCaptureError::CaptureFailure { 
                reason: "RGBA ImageBuffer 생성 실패".to_string() 
            })?;
        
        Ok(DynamicImage::ImageRgba8(img))
    } else {
        Err(EbCaptureError::CaptureFailure { 
            reason: format!("지원되지 않는 프레임 크기: {} bytes ({}x{} BGRA = {})", 
                frame.len(), width, height, expected_bgra) 
        })
    }
}

/// 이미지를 윈도우 영역으로 크롭합니다
fn crop_image_to_window(image: DynamicImage, window_rect: &WindowRect) -> Result<DynamicImage> {
    info!("✂️ 이미지 크롭: ({}, {}) {}x{}", 
        window_rect.x, window_rect.y, window_rect.width, window_rect.height);
    
    // 좌표 검증
    let img_width = image.width() as i32;
    let img_height = image.height() as i32;
    
    if window_rect.x < 0 || window_rect.y < 0 || 
       window_rect.x + window_rect.width > img_width ||
       window_rect.y + window_rect.height > img_height {
        warn!("윈도우 좌표가 화면 영역을 벗어남. 조정 중...");
        
        // 안전한 좌표로 조정
        let safe_x = window_rect.x.max(0) as u32;
        let safe_y = window_rect.y.max(0) as u32;
        let safe_width = (window_rect.width.min(img_width - window_rect.x.max(0))).max(100) as u32;
        let safe_height = (window_rect.height.min(img_height - window_rect.y.max(0))).max(100) as u32;
        
        info!("조정된 좌표: ({}, {}) {}x{}", safe_x, safe_y, safe_width, safe_height);
        return Ok(image.crop_imm(safe_x, safe_y, safe_width, safe_height));
    }
    
    Ok(image.crop_imm(
        window_rect.x as u32, 
        window_rect.y as u32, 
        window_rect.width as u32, 
        window_rect.height as u32
    ))
}

/// Windows에서 PrintWindow API를 사용한 직접 윈도우 캡쳐
#[cfg(windows)]
async fn capture_window_direct_windows(window: &WindowInfo, window_rect: &WindowRect) -> Result<DynamicImage> {
    use crate::window_manager::WindowHandle;
    
    if let WindowHandle::Windows(hwnd) = &window.handle {
        info!("🖼️ PrintWindow API로 직접 캡쳐 시도");
        
        unsafe {
            // 윈도우 DC 가져오기
            let window_dc = GetWindowDC(*hwnd);
            if window_dc.is_null() {
                return Err(EbCaptureError::CaptureFailure { 
                    reason: "윈도우 DC 가져오기 실패".to_string() 
                });
            }
            
            // 메모리 DC 생성
            let mem_dc = CreateCompatibleDC(window_dc);
            if mem_dc.is_null() {
                ReleaseDC(*hwnd, window_dc);
                return Err(EbCaptureError::CaptureFailure { 
                    reason: "메모리 DC 생성 실패".to_string() 
                });
            }
            
            let width = window_rect.width;
            let height = window_rect.height;
            
            // 비트맵 생성
            let bitmap = CreateCompatibleBitmap(window_dc, width, height);
            if bitmap.is_null() {
                DeleteDC(mem_dc);
                ReleaseDC(*hwnd, window_dc);
                return Err(EbCaptureError::CaptureFailure { 
                    reason: "비트맵 생성 실패".to_string() 
                });
            }
            
            // 비트맵을 메모리 DC에 선택
            let old_bitmap = SelectObject(mem_dc, bitmap as *mut winapi::ctypes::c_void);
            
            // PrintWindow로 윈도우 내용을 메모리 DC에 복사
            let print_result = PrintWindow(*hwnd, mem_dc, 0);
            
            if print_result == 0 {
                // PrintWindow 실패시 BitBlt로 대체 시도
                warn!("PrintWindow 실패, BitBlt로 대체");
                let bitblt_result = BitBlt(
                    mem_dc, 0, 0, width, height,
                    window_dc, 0, 0, SRCCOPY
                );
                
                if bitblt_result == 0 {
                    SelectObject(mem_dc, old_bitmap);
                    DeleteObject(bitmap as *mut winapi::ctypes::c_void);
                    DeleteDC(mem_dc);
                    ReleaseDC(*hwnd, window_dc);
                    return Err(EbCaptureError::CaptureFailure { 
                        reason: "BitBlt도 실패".to_string() 
                    });
                }
            }
            
            // 비트맵 데이터 추출
            let mut bitmap_info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height, // 음수로 설정하여 top-down DIB
                    biPlanes: 1,
                    biBitCount: 32, // BGRA
                    biCompression: BI_RGB,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [RGBQUAD { rgbBlue: 0, rgbGreen: 0, rgbRed: 0, rgbReserved: 0 }],
            };
            
            let buffer_size = (width * height * 4) as usize;
            let mut buffer = vec![0u8; buffer_size];
            
            let lines = GetDIBits(
                mem_dc,
                bitmap,
                0,
                height as u32,
                buffer.as_mut_ptr() as *mut winapi::ctypes::c_void,
                &mut bitmap_info,
                DIB_RGB_COLORS
            );
            
            // 정리
            SelectObject(mem_dc, old_bitmap);
            DeleteObject(bitmap as *mut winapi::ctypes::c_void);
            DeleteDC(mem_dc);
            ReleaseDC(*hwnd, window_dc);
            
            if lines == 0 {
                return Err(EbCaptureError::CaptureFailure { 
                    reason: "비트맵 데이터 추출 실패".to_string() 
                });
            }
            
            info!("📥 직접 캡쳐 데이터: {} bytes ({}x{})", buffer.len(), width, height);
            
            // BGRA → RGBA 변환
            let mut rgba_data = Vec::with_capacity(buffer.len());
            for chunk in buffer.chunks_exact(4) {
                rgba_data.push(chunk[2]); // R
                rgba_data.push(chunk[1]); // G
                rgba_data.push(chunk[0]); // B
                rgba_data.push(chunk[3]); // A
            }
            
            let img = ImageBuffer::from_raw(width as u32, height as u32, rgba_data)
                .ok_or_else(|| EbCaptureError::CaptureFailure { 
                    reason: "직접 캡쳐 ImageBuffer 생성 실패".to_string() 
                })?;
            
            info!("✅ 직접 윈도우 캡쳐 성공");
            Ok(DynamicImage::ImageRgba8(img))
        }
    } else {
        Err(EbCaptureError::CaptureFailure { 
            reason: "Windows 핸들이 아님".to_string() 
        })
    }
}

async fn save_frame_as_image_smart(
    frame: &[u8], 
    width: usize, 
    height: usize, 
    output_path: &Path
) -> Result<()> {
    info!("이미지 변환 시작: {}x{}, 데이터 크기: {} bytes", width, height, frame.len());
    
    // Windows scrap은 일반적으로 BGRA 형식 반환
    // 단순하고 직접적인 접근법 사용
    let expected_bgra_size = width * height * 4;
    let expected_bgr_size = width * height * 3;
    
    // 1. BGRA 형식 시도 (가장 일반적)
    if frame.len() == expected_bgra_size {
        info!("BGRA 형식으로 변환 시도 (정확한 크기 매치)");
        match convert_bgra_to_rgba_fixed(frame, width, height, output_path).await {
            Ok(_) => {
                info!("✅ BGRA → RGBA 변환 성공");
                return Ok(());
            }
            Err(e) => warn!("BGRA 변환 실패: {}", e),
        }
    }
    
    // 2. BGR 형식 시도
    if frame.len() == expected_bgr_size {
        info!("BGR 형식으로 변환 시도");
        match convert_bgr_to_rgba_fixed(frame, width, height, output_path).await {
            Ok(_) => {
                info!("✅ BGR → RGBA 변환 성공");
                return Ok(());
            }
            Err(e) => warn!("BGR 변환 실패: {}", e),
        }
    }
    
    // 3. 크기가 맞지 않으면 실제 해상도 재계산 시도
    if frame.len() % 4 == 0 {
        let actual_pixels = frame.len() / 4;
        let calculated_height = actual_pixels / width;
        
        if calculated_height > 0 && calculated_height <= height * 2 {
            info!("해상도 재계산 시도: {}x{} → {}x{}", width, height, width, calculated_height);
            match convert_bgra_to_rgba_fixed(frame, width, calculated_height, output_path).await {
                Ok(_) => {
                    info!("✅ 해상도 조정 후 BGRA 변환 성공");
                    return Ok(());
                }
                Err(e) => warn!("해상도 조정 변환 실패: {}", e),
            }
        }
    }
    
    // 4. 모든 시도 실패 시 BMP로 저장
    warn!("표준 변환 실패, BMP 형식으로 저장 시도");
    save_as_bmp_fixed(frame, width, height, output_path).await
}

fn detect_pixel_format(frame: &[u8], width: usize, height: usize) -> Result<PixelFormat> {
    let frame_len = frame.len();
    let pixel_count = width * height;
    
    debug!("픽셀 형식 감지: 데이터 {} bytes, 픽셀 수 {}", frame_len, pixel_count);
    
    // 정확히 맞는 형식 찾기
    if frame_len == pixel_count * 4 {
        debug!("4바이트/픽셀 감지 - BGRA 또는 RGBA");
        return Ok(PixelFormat::Bgra); // Windows는 보통 BGRA
    }
    
    if frame_len == pixel_count * 3 {
        debug!("3바이트/픽셀 감지 - BGR 또는 RGB");
        return Ok(PixelFormat::Bgr); // Windows는 보통 BGR
    }
    
    // 해상도가 다를 가능성 체크
    let possible_heights = [
        frame_len / (width * 4),  // BGRA
        frame_len / (width * 3),  // BGR
    ];
    
    for &calc_height in &possible_heights {
        if calc_height > 0 && calc_height <= height * 2 { // 합리적인 범위
            debug!("계산된 높이: {}, 예상 높이: {}", calc_height, height);
            if frame_len == width * calc_height * 4 {
                warn!("실제 해상도가 다를 수 있음: {}x{}", width, calc_height);
                return Ok(PixelFormat::Bgra);
            }
            if frame_len == width * calc_height * 3 {
                warn!("실제 해상도가 다를 수 있음: {}x{}", width, calc_height);
                return Ok(PixelFormat::Bgr);
            }
        }
    }
    
    Err(EbCaptureError::CaptureFailure { 
        reason: format!(
            "알 수 없는 픽셀 형식: {} bytes ({}x{} = {} 픽셀)", 
            frame_len, width, height, pixel_count
        ) 
    })
}

async fn convert_with_format(
    frame: &[u8], 
    width: usize, 
    height: usize, 
    format: &PixelFormat,
    output_path: &Path
) -> Result<()> {
    let bytes_per_pixel = format.bytes_per_pixel();
    let expected_size = width * height * bytes_per_pixel;
    
    // 실제 높이 계산 (데이터 크기가 다를 경우)
    let actual_height = frame.len() / (width * bytes_per_pixel);
    let actual_size = width * actual_height * bytes_per_pixel;
    
    if frame.len() != expected_size && frame.len() == actual_size {
        debug!("해상도 조정: {}x{} → {}x{}", width, height, width, actual_height);
        return convert_with_adjusted_size(frame, width, actual_height, format, output_path).await;
    }
    
    if frame.len() != expected_size {
        return Err(EbCaptureError::CaptureFailure { 
            reason: format!(
                "크기 불일치 ({:?}): 예상 {} bytes, 실제 {} bytes", 
                format, expected_size, frame.len()
            ) 
        });
    }
    
    convert_with_adjusted_size(frame, width, height, format, output_path).await
}

async fn convert_with_adjusted_size(
    frame: &[u8], 
    width: usize, 
    height: usize, 
    format: &PixelFormat,
    output_path: &Path
) -> Result<()> {
    debug!("형식 {:?}로 변환: {}x{}", format, width, height);
    
    let mut rgba_data = Vec::with_capacity(width * height * 4);
    
    match format {
        PixelFormat::Bgra => {
            for chunk in frame.chunks_exact(4) {
                rgba_data.push(chunk[2]); // R
                rgba_data.push(chunk[1]); // G
                rgba_data.push(chunk[0]); // B
                rgba_data.push(chunk[3]); // A
            }
        }
        PixelFormat::Rgba => {
            rgba_data.extend_from_slice(frame);
        }
        PixelFormat::Bgr => {
            for chunk in frame.chunks_exact(3) {
                rgba_data.push(chunk[2]); // R
                rgba_data.push(chunk[1]); // G
                rgba_data.push(chunk[0]); // B
                rgba_data.push(255);      // A (불투명)
            }
        }
        PixelFormat::Rgb => {
            for chunk in frame.chunks_exact(3) {
                rgba_data.push(chunk[0]); // R
                rgba_data.push(chunk[1]); // G
                rgba_data.push(chunk[2]); // B
                rgba_data.push(255);      // A (불투명)
            }
        }
    }
    
    let img: RgbaImage = ImageBuffer::from_raw(width as u32, height as u32, rgba_data)
        .ok_or_else(|| EbCaptureError::CaptureFailure { 
            reason: format!("RGBA ImageBuffer 생성 실패 ({:?})", format) 
        })?;
    
    img.save(output_path).map_err(|e| {
        EbCaptureError::CaptureFailure { 
            reason: format!("이미지 저장 실패 ({:?}): {}", format, e) 
        }
    })?;
    
    debug!("변환 및 저장 성공: {:?}", format);
    Ok(())
}

async fn convert_bgra_to_rgba_fixed(
    frame: &[u8], 
    width: usize, 
    height: usize, 
    output_path: &Path
) -> Result<()> {
    info!("🔄 BGRA → RGBA 변환 중... ({}x{})", width, height);
    
    let expected_size = width * height * 4;
    if frame.len() != expected_size {
        return Err(EbCaptureError::CaptureFailure { 
            reason: format!(
                "BGRA 크기 불일치: 예상 {} bytes, 실제 {} bytes", 
                expected_size, frame.len()
            ) 
        });
    }
    
    let mut rgba_data = Vec::with_capacity(frame.len());
    
    // BGRA → RGBA 변환 (더 안전한 방식)
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 4;
            if idx + 3 < frame.len() {
                rgba_data.push(frame[idx + 2]); // R (B에서)
                rgba_data.push(frame[idx + 1]); // G  
                rgba_data.push(frame[idx + 0]); // B (R에서)
                rgba_data.push(frame[idx + 3]); // A
            }
        }
    }
    
    if rgba_data.len() != frame.len() {
        return Err(EbCaptureError::CaptureFailure { 
            reason: format!("RGBA 변환 후 크기 불일치: {} → {}", frame.len(), rgba_data.len()) 
        });
    }
    
    // 이미지 생성 및 저장
    let img: RgbaImage = ImageBuffer::from_raw(width as u32, height as u32, rgba_data)
        .ok_or_else(|| EbCaptureError::CaptureFailure { 
            reason: "RGBA ImageBuffer 생성 실패".to_string() 
        })?;
    
    img.save(output_path).map_err(|e| {
        EbCaptureError::CaptureFailure { 
            reason: format!("RGBA 이미지 저장 실패: {}", e) 
        }
    })?;
    
    info!("✅ BGRA → RGBA 변환 및 저장 완료");
    Ok(())
}

async fn convert_bgr_to_rgba_fixed(
    frame: &[u8], 
    width: usize, 
    height: usize, 
    output_path: &Path
) -> Result<()> {
    info!("🔄 BGR → RGBA 변환 중... ({}x{})", width, height);
    
    let expected_size = width * height * 3;
    if frame.len() != expected_size {
        return Err(EbCaptureError::CaptureFailure { 
            reason: format!(
                "BGR 크기 불일치: 예상 {} bytes, 실제 {} bytes", 
                expected_size, frame.len()
            ) 
        });
    }
    
    let mut rgba_data = Vec::with_capacity(width * height * 4);
    
    // BGR → RGBA 변환
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 3;
            if idx + 2 < frame.len() {
                rgba_data.push(frame[idx + 2]); // R (B에서)
                rgba_data.push(frame[idx + 1]); // G  
                rgba_data.push(frame[idx + 0]); // B (R에서)
                rgba_data.push(255);            // A (불투명)
            }
        }
    }
    
    let img: RgbaImage = ImageBuffer::from_raw(width as u32, height as u32, rgba_data)
        .ok_or_else(|| EbCaptureError::CaptureFailure { 
            reason: "BGR→RGBA ImageBuffer 생성 실패".to_string() 
        })?;
    
    img.save(output_path).map_err(|e| {
        EbCaptureError::CaptureFailure { 
            reason: format!("BGR→RGBA 이미지 저장 실패: {}", e) 
        }
    })?;
    
    info!("✅ BGR → RGBA 변환 및 저장 완료");
    Ok(())
}

async fn convert_bgra_to_rgb_and_save(
    frame: &[u8], 
    width: usize, 
    height: usize, 
    output_path: &Path
) -> Result<()> {
    debug!("BGRA → RGB 변환 시도 (투명도 무시)");
    
    let mut rgb_data = Vec::with_capacity(width * height * 3);
    
    for chunk in frame.chunks_exact(4) {
        if chunk.len() == 4 {
            // BGRA → RGB 변환 (A 채널 제거)
            rgb_data.push(chunk[2]); // R
            rgb_data.push(chunk[1]); // G  
            rgb_data.push(chunk[0]); // B
        }
    }
    
    let img = image::RgbImage::from_raw(width as u32, height as u32, rgb_data)
        .ok_or_else(|| EbCaptureError::CaptureFailure { 
            reason: "RGB ImageBuffer 생성 실패".to_string() 
        })?;
    
    img.save(output_path).map_err(|e| {
        EbCaptureError::CaptureFailure { 
            reason: format!("RGB 이미지 저장 실패: {}", e) 
        }
    })?;
    
    debug!("RGB 변환 및 저장 성공");
    Ok(())
}

async fn save_as_bmp_fixed(
    frame: &[u8], 
    width: usize, 
    height: usize, 
    output_path: &Path
) -> Result<()> {
    warn!("🔧 BMP 형식으로 대체 저장 시도");
    
    // 실제 높이 추정
    let bytes_per_pixel = if frame.len() % (width * 4) == 0 { 4 } else { 3 };
    let actual_height = frame.len() / (width * bytes_per_pixel);
    
    if actual_height == 0 {
        return Err(EbCaptureError::CaptureFailure { 
            reason: "유효하지 않은 이미지 차원".to_string() 
        });
    }
    
    info!("BMP 저장: {}x{}, {}바이트/픽셀", width, actual_height, bytes_per_pixel);
    
    // PNG 대신 BMP로 저장 (더 단순함)
    let bmp_path = output_path.with_extension("bmp");
    
    // 24비트 BMP 생성
    let mut rgb_data = Vec::with_capacity(width * actual_height * 3);
    
    for y in 0..actual_height {
        for x in 0..width {
            let idx = (y * width + x) * bytes_per_pixel;
            if idx + 2 < frame.len() {
                if bytes_per_pixel == 4 {
                    // BGRA → RGB
                    rgb_data.push(frame[idx + 2]); // R
                    rgb_data.push(frame[idx + 1]); // G
                    rgb_data.push(frame[idx + 0]); // B
                } else {
                    // BGR → RGB
                    rgb_data.push(frame[idx + 2]); // R  
                    rgb_data.push(frame[idx + 1]); // G
                    rgb_data.push(frame[idx + 0]); // B
                }
            } else {
                // 패딩
                rgb_data.extend_from_slice(&[0, 0, 0]);
            }
        }
    }
    
    // image 크레이트로 RGB 이미지 생성
    let img = image::RgbImage::from_raw(width as u32, actual_height as u32, rgb_data)
        .ok_or_else(|| EbCaptureError::CaptureFailure { 
            reason: "RGB ImageBuffer 생성 실패".to_string() 
        })?;
    
    img.save(&bmp_path).map_err(|e| {
        EbCaptureError::CaptureFailure { 
            reason: format!("BMP 파일 저장 실패: {}", e) 
        }
    })?;
    
    info!("✅ BMP 형식 저장 성공: {} ({}x{})", bmp_path.display(), width, actual_height);
    Ok(())
}

fn create_bmp_header(width: u32, height: u32, file_size: u32) -> [u8; 54] {
    let mut header = [0u8; 54];
    
    // BMP 파일 헤더 (14 bytes)
    header[0..2].copy_from_slice(b"BM");              // Signature
    header[2..6].copy_from_slice(&file_size.to_le_bytes());  // File size
    header[10..14].copy_from_slice(&54u32.to_le_bytes());    // Data offset
    
    // DIB 헤더 (40 bytes)
    header[14..18].copy_from_slice(&40u32.to_le_bytes());    // Header size
    header[18..22].copy_from_slice(&width.to_le_bytes());    // Width
    header[22..26].copy_from_slice(&height.to_le_bytes());   // Height
    header[26..28].copy_from_slice(&1u16.to_le_bytes());     // Planes
    header[28..30].copy_from_slice(&24u16.to_le_bytes());    // Bits per pixel
    
    header
} 