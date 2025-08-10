mod window_manager;
mod capture;
mod keyboard;
mod pdf_generator;
mod cli;
mod error;

use anyhow::Result;
use log::{info, error};

#[tokio::main]
async fn main() -> Result<()> {
    // 로깅 초기화
    env_logger::init();
    
    info!("EBook 캡쳐 프로그램 시작");
    
    // CLI 실행
    match cli::run().await {
        Ok(_) => {
            info!("프로그램이 성공적으로 완료되었습니다.");
        }
        Err(e) => {
            error!("오류가 발생했습니다: {}", e);
            std::process::exit(1);
        }
    }
    
    Ok(())
}
