use crate::error::{EbCaptureError, Result};
use crate::{window_manager, capture, pdf_generator};
use log::{info, warn};
use std::io::{self, Write};
use std::path::PathBuf;
use tokio::time::{sleep, Duration};

pub async fn run() -> Result<()> {
    // 명령줄 인수 확인
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "test-pdf" {
        return run_pdf_test().await;
    }
    
    println!("=== EBook 캡쳐 프로그램 v1.0 ===\n");
    
    // 1. 시스템 환경 검사
    check_system_environment()?;
    
    // 2. 실행 중인 모든 프로그램 감지
    let all_programs = window_manager::detect_all_programs().await?;
    if all_programs.is_empty() {
        return Err(EbCaptureError::NoEbookPrograms);
    }
    
    // 3. 사용자에게 프로그램 선택 제시 (모든 프로그램)
    let selected_program = select_program_from_all(&all_programs)?;
    println!("선택된 프로그램: {}\n", selected_program.title);
    
    // 4. 페이지 수 입력
    let page_count = get_page_count()?;
    println!("캡쳐할 페이지 수: {}\n", page_count);
    
    // 5. 출력 디렉토리 확인/생성
    let output_dir = create_output_directory()?;
    println!("출력 디렉토리: {}\n", output_dir.display());
    
    // 6. 캡쳐 시작 전 대기
    countdown_before_capture().await;
    
    // 7. 선택된 프로그램 창을 최상단으로 이동 및 활성화
    window_manager::activate_and_bring_to_front(&selected_program).await?;
    
    // 8. 캡쳐 실행
    let captured_images = capture_pages(&selected_program, page_count, &output_dir).await?;
    
    // 9. PDF 생성
    let pdf_path = pdf_generator::create_pdf(&captured_images, &output_dir).await?;
    
    // 10. 임시 파일 정리 (옵션)
    cleanup_temp_files(&captured_images).await?;
    
    println!("\n🎉 캡쳐가 완료되었습니다!");
    println!("📁 PDF 파일: {}", pdf_path.display());
    
    Ok(())
}

fn check_system_environment() -> Result<()> {
    info!("시스템 환경 검사 중...");
    
    // 디스크 공간 검사 (간단한 구현)
    let available_space = get_available_disk_space()?;
    let required_space = 100; // 100MB 
    
    if available_space < required_space {
        return Err(EbCaptureError::InsufficientDiskSpace { 
            required_mb: required_space 
        });
    }
    
    println!("✅ 시스템 환경 검사 완료");
    Ok(())
}

fn select_program_from_all(programs: &[window_manager::WindowInfo]) -> Result<window_manager::WindowInfo> {
    // 전자책 프로그램 후보와 일반 프로그램을 구분
    let (ebook_candidates, other_programs) = separate_ebook_candidates(programs);
    
    println!("📚 전자책 프로그램 후보:");
    let mut option_index = 1;
    
    // 전자책 후보 먼저 표시
    for program in ebook_candidates.iter() {
        println!("{}. {} (PID: {})", option_index, program.title, program.pid);
        option_index += 1;
    }
    
    if ebook_candidates.is_empty() {
        println!("   (감지된 전자책 프로그램이 없습니다)");
    }
    
    println!("\n💻 기타 실행 중인 프로그램:");
    for program in other_programs.iter() {
        println!("{}. {} (PID: {})", option_index, program.title, program.pid);
        option_index += 1;
    }
    
    print!("\n번호를 입력하세요 (1-{}): ", programs.len());
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    let selection: usize = input.trim().parse()
        .map_err(|_| EbCaptureError::InvalidInput { 
            input: input.trim().to_string() 
        })?;
    
    if selection < 1 || selection > programs.len() {
        return Err(EbCaptureError::InvalidInput { 
            input: format!("범위를 벗어난 번호: {}", selection) 
        });
    }
    
    Ok(programs[selection - 1].clone())
}

fn separate_ebook_candidates(programs: &[window_manager::WindowInfo]) -> (Vec<window_manager::WindowInfo>, Vec<window_manager::WindowInfo>) {
    let ebook_keywords = [
        // 알라딘 관련
        "알라딘", "aladin", "alladinebook", "aladinebook", "알라딘이북", "알라딘북",
        // 기타 전자책 프로그램  
        "리디", "ridi", "ridibookx", "ridibooks",
        "yes24", "예스24", "yes24ebook",
        "크레마", "crema", "cremaker", 
        "교보", "kyobo", "kyobobook", "교보문고",
        "밀리의", "millie", "millielib",
        "adobe digital editions", "digital editions",
        "kindle", "amazon kindle",
        "calibre", "calibre-ebook",
        "epub", "epub reader", "epubreader",
        "pdf", "pdf reader", "pdfreader",
        "bookviewer", "book viewer", "ebookviewer", "ebook viewer",
        "viewer", "reader"
    ];
    
    let mut ebook_candidates = Vec::new();
    let mut other_programs = Vec::new();
    
    for program in programs {
        let title_lower = program.title.to_lowercase();
        
        // 시스템 윈도우나 빈 제목 제외
        if title_lower.trim().is_empty() || title_lower.len() < 3 {
            continue;
        }
        
        let system_windows = [
            "dwm", "explorer", "winlogon", "csrss", "smss", "wininit",
            "taskbar", "start menu", "cortana", "search", "notification",
            "program manager"
        ];
        
        if system_windows.iter().any(|sys| title_lower.contains(sys)) {
            continue;
        }
        
        // 전자책 관련 키워드 검사
        if ebook_keywords.iter().any(|keyword| title_lower.contains(keyword)) {
            ebook_candidates.push(program.clone());
        } else {
            other_programs.push(program.clone());
        }
    }
    
    (ebook_candidates, other_programs)
}

fn get_page_count() -> Result<u32> {
    print!("캡쳐할 페이지 수를 입력하세요 (1-999): ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    let page_count: u32 = input.trim().parse()
        .map_err(|_| EbCaptureError::InvalidInput { 
            input: input.trim().to_string() 
        })?;
    
    if page_count < 1 || page_count > 999 {
        return Err(EbCaptureError::InvalidInput { 
            input: format!("페이지 수 범위 초과: {}", page_count) 
        });
    }
    
    Ok(page_count)
}

fn create_output_directory() -> Result<std::path::PathBuf> {
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let dir_name = format!("captured_book_{}", timestamp);
    let output_dir = std::path::PathBuf::from(&dir_name);
    
    std::fs::create_dir_all(&output_dir)?;
    Ok(output_dir)
}

async fn countdown_before_capture() {
    println!("캡쳐를 시작합니다...");
    for i in (1..=3).rev() {
        print!("{}... ", i);
        io::stdout().flush().unwrap();
        sleep(Duration::from_secs(1)).await;
    }
    println!("시작!\n");
}

async fn capture_pages(
    window: &window_manager::WindowInfo, 
    page_count: u32,
    output_dir: &std::path::Path
) -> Result<Vec<std::path::PathBuf>> {
    let mut captured_images = Vec::new();
    
    // 윈도우 좌표 한 번만 가져오기 (캐싱)
    let window_rect = window_manager::get_window_rect(window).await?;
    println!("📐 윈도우 좌표: ({}, {}) 크기: {}x{}", 
        window_rect.x, window_rect.y, window_rect.width, window_rect.height);
    
    for page in 1..=page_count {
        println!("📸 페이지 {}/{} 캡쳐 중...", page, page_count);
        
        // 간단한 화면 캡쳐 (전체 화면 → 윈도우 영역 크롭)
        let image_path = output_dir.join(format!("page_{:03}.png", page));
        capture::capture_window(window, &window_rect, &image_path).await?;
        captured_images.push(image_path);
        
        // 마지막 페이지가 아니면 다음 페이지로
        if page < page_count {
            println!("⏭️ 다음 페이지로 이동... ({}/{})", page, page_count);
            
            // 첫 번째 캡쳐 후 윈도우 재활성화 (포커스 손실 방지)
            if page == 1 {
                println!("🔄 첫 번째 캡쳐 후 윈도우 재활성화...");
                window_manager::activate_and_bring_to_front(window).await?;
                sleep(Duration::from_millis(500)).await;
            }
            
            // 단일 페이지 이동 (수정된 로직)
            match crate::keyboard::navigate_to_next_page(window).await {
                Ok(_) => {
                    println!("✅ 페이지 이동 완료 ({}페이지 → {}페이지)", page, page + 1);
                }
                Err(e) => {
                    warn!("페이지 이동 중 오류: {}", e);
                    println!("⚠️ 페이지 이동에 실패했습니다. 수동으로 다음 페이지로 이동한 후 Enter를 눌러 계속하세요...");
                    
                    // 사용자 입력 대기
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input).unwrap();
                }
            }
            
            // 페이지 로딩 대기 (시간 최적화)
            sleep(Duration::from_millis(1500)).await;
        }
    }
    
    println!("\n✅ 모든 페이지 캡쳐 완료");
    Ok(captured_images)
}

async fn cleanup_temp_files(image_paths: &[std::path::PathBuf]) -> Result<()> {
    print!("임시 이미지 파일을 삭제하시겠습니까? (y/N): ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    if input.trim().to_lowercase() == "y" {
        for path in image_paths {
            if let Err(e) = std::fs::remove_file(path) {
                warn!("임시 파일 삭제 실패: {} - {}", path.display(), e);
            }
        }
        println!("🗑️ 임시 파일 정리 완료");
    }
    
    Ok(())
}

fn get_available_disk_space() -> Result<u64> {
    // 간단한 구현 - 실제로는 플랫폼별 API 사용 필요
    Ok(1000) // 1000MB로 가정
}

async fn run_pdf_test() -> Result<()> {
    println!("=== PDF 생성 테스트 모드 ===\n");
    
    // 테스트할 PNG 파일들의 경로
    let image_dir = PathBuf::from("captured_book_20250803_124920");
    let mut image_paths = Vec::new();
    
    println!("📁 테스트 디렉토리: {}", image_dir.display());
    
    // PNG 파일들을 찾아서 정렬
    for i in 1..=5 {
        let image_path = image_dir.join(format!("page_{:03}.png", i));
        if image_path.exists() {
            image_paths.push(image_path.clone());
            println!("✅ 이미지 발견: {}", image_path.display());
        } else {
            println!("❌ 이미지 없음: {}", image_path.display());
        }
    }
    
    if image_paths.is_empty() {
        println!("❌ PNG 파일을 찾을 수 없습니다!");
        return Err(EbCaptureError::PdfGenerationFailure { 
            reason: "테스트할 PNG 파일이 없습니다".to_string() 
        });
    }
    
    println!("\n📚 총 {} 개의 이미지로 PDF 생성을 시작합니다...", image_paths.len());
    
    // PDF 생성 테스트
    let output_dir = PathBuf::from("test_output");
    std::fs::create_dir_all(&output_dir)?;
    println!("📂 출력 디렉토리: {}", output_dir.display());
    
    println!("\n🔄 PDF 생성 중...");
    match pdf_generator::create_pdf(&image_paths, &output_dir).await {
        Ok(pdf_path) => {
            println!("\n✅ PDF 생성 성공!");
            println!("📄 생성된 PDF: {}", pdf_path.display());
            println!("\n🔍 이제 PDF 파일을 열어서 이미지가 올바르게 들어갔는지 확인해보세요!");
            
            // 파일 크기 확인
            if let Ok(metadata) = std::fs::metadata(&pdf_path) {
                println!("📊 파일 크기: {} bytes", metadata.len());
            }
        }
        Err(e) => {
            println!("\n❌ PDF 생성 실패: {}", e);
            return Err(e);
        }
    }
    
    Ok(())
} 