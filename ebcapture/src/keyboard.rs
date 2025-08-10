use crate::error::{EbCaptureError, Result};
use crate::window_manager::{WindowInfo};
use enigo::{Enigo, KeyboardControllable, MouseControllable, Key, MouseButton};
use log::{debug, info, warn};
use tokio::time::{sleep, Duration};

/// 오른쪽 화살표 키를 전송합니다 (다음 페이지) - 향상된 버전
pub async fn send_right_arrow() -> Result<()> {
    debug!("📤 오른쪽 화살표 키 전송 시작");
    
    let mut enigo = Enigo::new();
    
    // 더 안정적인 키 입력을 위한 짧은 대기
    sleep(Duration::from_millis(100)).await;
    
    // 키 클릭 (누르고 떼기)
    enigo.key_click(Key::RightArrow);
    
    // 키 입력 후 대기
    sleep(Duration::from_millis(200)).await;
    
    debug!("✅ 오른쪽 화살표 키 입력 완료");
    Ok(())
}

/// 왼쪽 화살표 키를 전송합니다 (이전 페이지)
#[allow(dead_code)]
pub async fn send_left_arrow() -> Result<()> {
    debug!("왼쪽 화살표 키 전송");
    
    let mut enigo = Enigo::new();
    enigo.key_click(Key::LeftArrow);
    
    Ok(())
}

/// Page Down 키를 전송합니다
pub async fn send_page_down() -> Result<()> {
    debug!("Page Down 키 전송");
    
    let mut enigo = Enigo::new();
    enigo.key_click(Key::PageDown);
    
    Ok(())
}

/// Page Up 키를 전송합니다
#[allow(dead_code)]
pub async fn send_page_up() -> Result<()> {
    debug!("Page Up 키 전송");
    
    let mut enigo = Enigo::new();
    enigo.key_click(Key::PageUp);
    
    Ok(())
}

/// 스페이스바를 전송합니다 (일부 리더에서 페이지 넘김)
pub async fn send_space() -> Result<()> {
    debug!("스페이스바 전송");
    
    let mut enigo = Enigo::new();
    enigo.key_click(Key::Space);
    
    Ok(())
}

/// Enter 키를 전송합니다
#[allow(dead_code)]
pub async fn send_enter() -> Result<()> {
    debug!("Enter 키 전송");
    
    let mut enigo = Enigo::new();
    enigo.key_click(Key::Return);
    
    Ok(())
}

/// 마우스 클릭을 시뮬레이션합니다 (특정 위치)
pub async fn click_at_position(x: i32, y: i32) -> Result<()> {
    debug!("마우스 클릭: ({}, {})", x, y);
    
    let mut enigo = Enigo::new();
    
    // 마우스 위치 이동
    enigo.mouse_move_to(x, y);
    sleep(Duration::from_millis(100)).await;
    
    // 클릭
    enigo.mouse_click(MouseButton::Left);
    
    Ok(())
}

/// 페이지 넘김 방법을 자동으로 선택하여 실행합니다
#[allow(dead_code)]
pub async fn navigate_next_page(method: NavigationMethod) -> Result<()> {
    match method {
        NavigationMethod::RightArrow => send_right_arrow().await,
        NavigationMethod::PageDown => send_page_down().await,
        NavigationMethod::Space => send_space().await,
        NavigationMethod::Click(x, y) => click_at_position(x, y).await,
    }
}

/// 페이지 넘김 방법을 정의하는 열거형
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum NavigationMethod {
    RightArrow,
    PageDown,
    Space,
    Click(i32, i32), // x, y 좌표
}

impl NavigationMethod {
    /// 전자책 프로그램에 따른 기본 네비게이션 방법을 반환합니다
    pub fn for_program(program_name: &str) -> Self {
        let name_lower = program_name.to_lowercase();
        
        if name_lower.contains("ridi") || name_lower.contains("리디") {
            NavigationMethod::RightArrow
        } else if name_lower.contains("aladin") || name_lower.contains("알라딘") {
            //NavigationMethod::PageDown
            NavigationMethod::RightArrow
        } else if name_lower.contains("yes24") || name_lower.contains("예스24") {
            NavigationMethod::Space
        } else if name_lower.contains("kindle") {
            NavigationMethod::RightArrow
        } else if name_lower.contains("calibre") {
            NavigationMethod::PageDown
        } else {
            // 기본값: 오른쪽 화살표
            NavigationMethod::RightArrow
        }
    }
}

/// 윈도우에 포커스를 확보하고 안정적으로 다음 페이지로 이동합니다
pub async fn navigate_to_next_page(window: &WindowInfo) -> Result<()> {
    info!("🔄 다음 페이지 이동 시작 (단일 페이지)");
    
    // 1. 윈도우 포커스 재확보 (강화된 버전)
    ensure_window_focus(window).await?;
    
    // 2. 포커스 확보 대기 (첫 번째 페이지 이동을 위해 추가 대기)
    sleep(Duration::from_millis(500)).await;
    
    // 3. 프로그램별 적절한 키 입력 1회 실행
    let navigation_method = NavigationMethod::for_program(&window.title);
    info!("📋 네비게이션 방법: {:?}", navigation_method);
    
    match navigate_with_retry(&navigation_method).await {
        Ok(_) => {
            info!("✅ 페이지 이동 성공 (1페이지)");
            Ok(())
        }
        Err(e) => {
            warn!("❌ 기본 방법 실패: {}", e);
            info!("🔄 대안 방법으로 재시도");
            try_alternative_navigation().await
        }
    }
}

/// 네비게이션 방법을 한 번만 실행하고, 실패시에만 재시도합니다
async fn navigate_with_retry(method: &NavigationMethod) -> Result<()> {
    debug!("네비게이션 실행: {:?}", method);
    
    // 1번만 키 입력 실행
    match method {
        NavigationMethod::RightArrow => {
            send_right_arrow().await?;
        }
        NavigationMethod::PageDown => {
            send_page_down().await?;
        }
        NavigationMethod::Space => {
            send_space().await?;
        }
        NavigationMethod::Click(x, y) => {
            click_at_position(*x, *y).await?;
        }
    }
    
    // 키 입력 완료 후 짧은 대기
    sleep(Duration::from_millis(300)).await;
    debug!("✅ 네비게이션 키 입력 완료");
    
    Ok(())
}

/// 대안적인 네비게이션 방법들을 순차적으로 시도합니다
async fn try_alternative_navigation() -> Result<()> {
    warn!("🔄 대안적 네비게이션 방법 시도");
    
    let methods = [
        NavigationMethod::RightArrow,
        NavigationMethod::PageDown,
        NavigationMethod::Space,
    ];
    
    for method in &methods {
        info!("🎯 대안 방법 시도: {:?}", method);
        
        match navigate_with_retry(method).await {
            Ok(_) => {
                info!("✅ 대안 방법 성공: {:?}", method);
                return Ok(());
            }
            Err(e) => {
                warn!("❌ 방법 실패: {:?} - {}", method, e);
                // 다음 방법 시도 전 짧은 대기
                sleep(Duration::from_millis(500)).await;
            }
        }
    }
    
    Err(EbCaptureError::KeyboardInputFailure { 
        reason: "모든 네비게이션 방법 실패".to_string() 
    })
}

/// 윈도우 포커스를 확보합니다
async fn ensure_window_focus(window: &WindowInfo) -> Result<()> {
    #[cfg(windows)]
    {
        use crate::window_manager::WindowHandle;
        use winapi::um::winuser::*;
        
        let WindowHandle::Windows(hwnd) = &window.handle;
        unsafe {
            // 윈도우를 전면으로 가져오기
            if SetForegroundWindow(*hwnd) == 0 {
                warn!("SetForegroundWindow 실패, 대안 방법 시도");
                
                // 대안적 방법
                let current_thread = winapi::um::processthreadsapi::GetCurrentThreadId();
                let mut target_thread = 0u32;
                GetWindowThreadProcessId(*hwnd, &mut target_thread);
                
                if target_thread != 0 && target_thread != current_thread {
                    winapi::um::winuser::AttachThreadInput(current_thread, target_thread, 1);
                    SetForegroundWindow(*hwnd);
                    winapi::um::winuser::AttachThreadInput(current_thread, target_thread, 0);
                }
            }
            
            // 윈도우 활성화 (크기 유지)
            if winapi::um::winuser::IsIconic(*hwnd) != 0 {
                // 최소화된 경우에만 복원
                ShowWindow(*hwnd, SW_RESTORE);
                debug!("최소화된 윈도우를 복원하여 포커스 재확보");
            } else {
                // 최소화되지 않은 경우 크기 유지
                ShowWindow(*hwnd, SW_SHOW);
                debug!("윈도우 크기 유지하면서 포커스 재확보");
            }
        }
    }
    
    // 포커스 확보 후 대기
    sleep(Duration::from_millis(300)).await;
    Ok(())
} 