use crate::error::{EbCaptureError, Result};
use log::{info, debug, warn};

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub title: String,
    pub pid: u32,
    pub handle: WindowHandle,
}

#[derive(Debug, Clone)]
pub struct WindowRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone)]
pub enum WindowHandle {
    #[cfg(windows)]
    Windows(winapi::shared::windef::HWND),
    #[cfg(target_os = "macos")]
    MacOS(u32), // CGWindowID
    #[cfg(not(any(windows, target_os = "macos")))]
    Unsupported,
}

/// 실행 중인 모든 프로그램을 감지합니다
pub async fn detect_all_programs() -> Result<Vec<WindowInfo>> {
    info!("실행 중인 모든 프로그램 감지 중...");
    
    let all_windows = get_all_windows().await?;
    
    // 기본 필터링: 시스템 윈도우나 빈 제목 제외
    let filtered_windows = filter_valid_windows(all_windows);
    
    debug!("감지된 프로그램 수: {}", filtered_windows.len());
    
    Ok(filtered_windows)
}

/// 선택된 윈도우를 최상단으로 이동하고 활성화합니다
pub async fn activate_and_bring_to_front(window: &WindowInfo) -> Result<()> {
    info!("윈도우 최상단 이동 및 활성화: {}", window.title);
    
    #[cfg(windows)]
    {
        match bring_window_to_front_windows(&window.handle) {
            Ok(_) => {
                info!("✅ 윈도우가 최상단으로 이동되었습니다.");
            }
            Err(e) => {
                warn!("윈도우 최상단 이동 실패: {}", e);
                // 실패해도 계속 진행
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        activate_window_macos(&window.handle)?;
    }
    
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        return Err(EbCaptureError::CaptureFailure { 
            reason: "지원되지 않는 플랫폼".to_string() 
        });
    }
    
    // 윈도우가 이동될 때까지 대기
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
    Ok(())
}

/// 윈도우의 화면상 좌표와 크기를 가져옵니다
pub async fn get_window_rect(window: &WindowInfo) -> Result<WindowRect> {
    info!("윈도우 좌표 가져오기: {}", window.title);
    
    #[cfg(windows)]
    {
        get_window_rect_windows(&window.handle)
    }
    
    #[cfg(target_os = "macos")]
    {
        // TODO: macOS 구현 추가
        Err(EbCaptureError::CaptureFailure { 
            reason: "macOS는 아직 지원되지 않습니다".to_string() 
        })
    }
    
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Err(EbCaptureError::CaptureFailure { 
            reason: "지원되지 않는 플랫폼".to_string() 
        })
    }
}

async fn get_all_windows() -> Result<Vec<WindowInfo>> {
    #[cfg(windows)]
    {
        get_windows_list()
    }
    
    #[cfg(target_os = "macos")]
    {
        get_macos_windows()
    }
    
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Err(EbCaptureError::CaptureFailure { 
            reason: "지원되지 않는 플랫폼".to_string() 
        })
    }
}

fn filter_valid_windows(windows: Vec<WindowInfo>) -> Vec<WindowInfo> {
    windows.into_iter()
        .filter(|window| {
            let title_lower = window.title.to_lowercase();
            
            // 빈 제목이나 너무 짧은 제목 제외
            if title_lower.trim().is_empty() || title_lower.len() < 3 {
                return false;
            }
            
            // 기본 시스템 윈도우 제외
            let system_windows = [
                "dwm", "winlogon", "csrss", "smss", "wininit",
                "program manager", "desktop window manager"
            ];
            
            if system_windows.iter().any(|sys| title_lower.contains(sys)) {
                return false;
            }
            
            true
        })
        .collect()
}

// Windows 구현
#[cfg(windows)]
mod windows_impl {
    use super::*;
    use winapi::um::winuser::*;
    use winapi::shared::windef::*;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    
    pub fn get_windows_list() -> Result<Vec<WindowInfo>> {
        let mut windows = Vec::new();
        
        unsafe {
            EnumWindows(Some(enum_windows_proc), &mut windows as *mut Vec<WindowInfo> as isize);
        }
        
        Ok(windows)
    }
    
    unsafe extern "system" fn enum_windows_proc(
        hwnd: HWND, 
        lparam: isize
    ) -> i32 {
        let windows = &mut *(lparam as *mut Vec<WindowInfo>);
        
        // 윈도우가 보이는지 확인
        if IsWindowVisible(hwnd) == 0 {
            return 1; // 계속 열거
        }
        
        // 윈도우 제목 가져오기
        let mut title_buffer = [0u16; 256];
        let title_len = GetWindowTextW(hwnd, title_buffer.as_mut_ptr(), 256);
        
        if title_len > 0 {
            let title = OsString::from_wide(&title_buffer[..title_len as usize])
                .to_string_lossy()
                .to_string();
            
            // 프로세스 ID 가져오기
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            
            if !title.trim().is_empty() {
                windows.push(WindowInfo {
                    title,
                    pid,
                    handle: WindowHandle::Windows(hwnd),
                });
            }
        }
        
        1 // 계속 열거
    }
    
    pub fn bring_window_to_front_windows(handle: &WindowHandle) -> Result<()> {
        let WindowHandle::Windows(hwnd) = handle;
        unsafe {
            // 1. 윈도우 상태 확인 후 적절한 처리 (크기 유지)
            if winapi::um::winuser::IsIconic(*hwnd) != 0 {
                    // 최소화된 경우에만 복원
                    ShowWindow(*hwnd, SW_RESTORE);
                    info!("최소화된 윈도우를 복원했습니다");
                } else {
                    // 최소화되지 않은 경우 크기 유지하면서 보이기
                    ShowWindow(*hwnd, SW_SHOW);
                    info!("윈도우 크기를 유지하면서 활성화했습니다");
                }
                
                // 2. 최상단으로 이동 (HWND_TOPMOST)
                winapi::um::winuser::SetWindowPos(
                    *hwnd,
                    winapi::um::winuser::HWND_TOPMOST,
                    0, 0, 0, 0,
                    winapi::um::winuser::SWP_NOMOVE | winapi::um::winuser::SWP_NOSIZE
                );
                
                // 3. 포커스 설정
                if SetForegroundWindow(*hwnd) == 0 {
                    // 대안적 방법 시도
                    let current_thread = winapi::um::processthreadsapi::GetCurrentThreadId();
                    let mut target_thread = 0u32;
                    GetWindowThreadProcessId(*hwnd, &mut target_thread);
                    
                    if target_thread != 0 && target_thread != current_thread {
                        winapi::um::winuser::AttachThreadInput(current_thread, target_thread, 1);
                        SetForegroundWindow(*hwnd);
                        winapi::um::winuser::AttachThreadInput(current_thread, target_thread, 0);
                    }
                }
                
                // 4. 다시 일반 윈도우로 변경 (항상 최상단은 아니게)
                winapi::um::winuser::SetWindowPos(
                    *hwnd,
                    winapi::um::winuser::HWND_NOTOPMOST,
                    0, 0, 0, 0,
                    winapi::um::winuser::SWP_NOMOVE | winapi::um::winuser::SWP_NOSIZE
                );
                
                info!("윈도우 최상단 이동 완료");
                Ok(())
        }
    }
    
    pub fn get_window_rect_windows(handle: &WindowHandle) -> Result<WindowRect> {
        let WindowHandle::Windows(hwnd) = handle;
        unsafe {
            let mut rect = winapi::shared::windef::RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            
            if winapi::um::winuser::GetWindowRect(*hwnd, &mut rect) != 0 {
                let window_rect = WindowRect {
                    x: rect.left,
                    y: rect.top,
                    width: rect.right - rect.left,
                    height: rect.bottom - rect.top,
                };
                
                info!("윈도우 좌표: ({}, {}), 크기: {}x{}", 
                    window_rect.x, window_rect.y, window_rect.width, window_rect.height);
                
                Ok(window_rect)
            } else {
                Err(EbCaptureError::CaptureFailure { 
                    reason: "윈도우 좌표 가져오기 실패".to_string() 
                })
            }
        }
    }
}

#[cfg(windows)]
use windows_impl::*;

// macOS 구현 (기본 스켈레톤)
#[cfg(target_os = "macos")]
mod macos_impl {
    use super::*;
    
    pub fn get_macos_windows() -> Result<Vec<WindowInfo>> {
        // TODO: Core Graphics API를 사용하여 구현
        Ok(Vec::new())
    }
    
    pub fn activate_window_macos(handle: &WindowHandle) -> Result<()> {
        // TODO: Cocoa API를 사용하여 구현
        Ok(())
    }
}

#[cfg(target_os = "macos")]
use macos_impl::*; 