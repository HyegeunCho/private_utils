use thiserror::Error;

#[derive(Error, Debug)]
pub enum EbCaptureError {
    #[error("전자책 프로그램을 찾을 수 없습니다")]
    NoEbookPrograms,
    
    #[error("선택된 프로그램을 찾을 수 없습니다: {program}")]
    ProgramNotFound { program: String },
    
    #[error("화면 캡쳐 권한이 없습니다. 시스템 설정에서 권한을 허용해주세요.")]
    CapturePermissionDenied,
    
    #[error("화면 캡쳐에 실패했습니다: {reason}")]
    CaptureFailure { reason: String },
    
    #[error("PDF 생성에 실패했습니다: {reason}")]
    PdfGenerationFailure { reason: String },
    
    #[error("키보드 입력 시뮬레이션에 실패했습니다: {reason}")]
    KeyboardInputFailure { reason: String },
    
    #[error("디스크 공간이 부족합니다. 최소 {required_mb}MB 필요")]
    InsufficientDiskSpace { required_mb: u64 },
    
    #[error("잘못된 입력값입니다: {input}")]
    InvalidInput { input: String },
    
    #[error("IO 오류: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("이미지 처리 오류: {0}")]
    Image(#[from] image::ImageError),
}

pub type Result<T> = std::result::Result<T, EbCaptureError>; 