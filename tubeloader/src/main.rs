use anyhow::{Context, Result};
use clap::Parser;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use url::Url;
use yt_dlp::{Youtube, model::{VideoQuality, AudioQuality, VideoCodecPreference, AudioCodecPreference}};

#[derive(Parser)]
#[command(name = "tubeloader")]
#[command(about = "유튜브 영상 다운로더", long_about = None)]
struct Cli {
    /// 다운로드할 유튜브 URL들
    #[arg(help = "유튜브 영상 URL (여러 개 가능)")]
    urls: Vec<String>,
    
    /// 다운로드 품질 설정
    #[arg(short, long, default_value = "high", help = "영상 품질 (best, high, medium, low, worst)")]
    quality: String,
    
    /// 출력 디렉토리
    #[arg(short, long, default_value = "./downloads", help = "다운로드 폴더 경로")]
    output: String,
    
    /// 동시 다운로드 수
    #[arg(short, long, default_value = "3", help = "동시 다운로드할 영상 수")]
    concurrent: usize,
    
    /// 오디오만 다운로드
    #[arg(short, long, help = "오디오만 다운로드")]
    audio_only: bool,
    
    /// 진단 모드
    #[arg(long, help = "진단 정보 출력")]
    verbose: bool,
    
    /// 오디오 품질
    #[arg(long, default_value = "high", help = "오디오 품질 (best, high, medium, low, worst)")]
    audio_quality: String,
    
    /// 비디오 코덱
    #[arg(long, default_value = "any", help = "비디오 코덱 (vp9, avc1, av1, any)")]
    video_codec: String,
    
    /// 오디오 코덱
    #[arg(long, default_value = "any", help = "오디오 코덱 (opus, aac, mp3, any)")]
    audio_codec: String,
    
    /// 자막 건너뛰기
    #[arg(long, help = "자막 다운로드 건너뛰기 (JSON 파싱 오류 방지)")]
    skip_subtitles: bool,
}

/// 다운로드 결과를 저장하는 구조체
#[derive(Debug)]
struct DownloadResult {
    url: String,
    title: Option<String>,
    success: bool,
    error: Option<String>,
    file_path: Option<PathBuf>,
}

impl DownloadResult {
    fn success(url: String, title: String, file_path: PathBuf) -> Self {
        Self {
            url,
            title: Some(title),
            success: true,
            error: None,
            file_path: Some(file_path),
        }
    }
    
    fn failure(url: String, error: String) -> Self {
        Self {
            url,
            title: None,
            success: false,
            error: Some(error),
            file_path: None,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    println!("🎥 TubeLoader - 유튜브 영상 다운로더");
    println!("📅 라이브러리: yt-dlp 크레이트 v1.3.4");
    println!("📁 다운로드 폴더: {}", cli.output);
    println!("⚡ 동시 다운로드 수: {}", cli.concurrent);
    
    if cli.verbose {
        println!("🔍 진단 모드가 활성화되었습니다");
        println!("🖥️  OS: {}", std::env::consts::OS);
        println!("🏗️  Architecture: {}", std::env::consts::ARCH);
    }
    
    // 출력 디렉토리 생성
    tokio::fs::create_dir_all(&cli.output)
        .await
        .context("출력 디렉토리를 생성할 수 없습니다")?;
    
    // URL 유효성 검사
    let valid_urls = validate_urls(&cli.urls, cli.verbose)?;
    
    if valid_urls.is_empty() {
        println!("❌ 유효한 유튜브 URL이 없습니다.");
        println!("\n📖 지원하는 URL 형식:");
        println!("  • https://www.youtube.com/watch?v=VIDEO_ID");
        println!("  • https://youtu.be/VIDEO_ID");
        println!("  • https://m.youtube.com/watch?v=VIDEO_ID");
        return Ok(());
    }
    
    println!("📋 {} 개의 영상을 다운로드합니다...\n", valid_urls.len());
    
    // yt-dlp 및 ffmpeg 바이너리 준비
    println!("🔧 yt-dlp 및 ffmpeg 바이너리를 준비하는 중...");
    let libraries_dir = PathBuf::from("libs");
    let output_dir = PathBuf::from(&cli.output);
    
    let fetcher = match Youtube::with_new_binaries(libraries_dir, output_dir).await {
        Ok(fetcher) => {
            println!("✅ 바이너리 준비 완료!");
            fetcher
        },
        Err(e) => {
            println!("❌ 바이너리 준비 실패: {}", e);
            return Err(e.into());
        }
    };
    
    // 영상 다운로드 시작
    download_videos(valid_urls, &cli, &fetcher).await?;
    
    println!("\n✅ 모든 다운로드가 완료되었습니다!");
    Ok(())
}

/// URL 유효성 검사 함수
fn validate_urls(urls: &[String], verbose: bool) -> Result<Vec<String>> {
    let mut valid_urls = Vec::new();
    
    for url in urls {
        match extract_video_id(url) {
            Some(video_id) => {
                let normalized_url = format!("https://www.youtube.com/watch?v={}", video_id);
                valid_urls.push(normalized_url.clone());
                if verbose {
                    println!("✅ 유효한 URL: {} (Video ID: {})", normalized_url, video_id);
                } else {
                    println!("✅ 유효한 URL: {}", normalized_url);
                }
            }
            None => {
                println!("❌ 잘못된 URL: {}", url);
                if verbose {
                    println!("   🔍 유튜브 Video ID를 추출할 수 없습니다");
                }
            }
        }
    }
    
    Ok(valid_urls)
}

/// 유튜브 URL에서 Video ID 추출
fn extract_video_id(url: &str) -> Option<String> {
    // 다양한 유튜브 URL 형식 지원
    if let Ok(parsed_url) = Url::parse(url) {
        if let Some(host) = parsed_url.host_str() {
            if host.contains("youtube.com") || host.contains("m.youtube.com") {
                // youtube.com/watch?v=VIDEO_ID 형식
                if let Some(query) = parsed_url.query() {
                    for pair in query.split('&') {
                        let parts: Vec<&str> = pair.split('=').collect();
                        if parts.len() == 2 && parts[0] == "v" {
                            return Some(parts[1].to_string());
                        }
                    }
                }
            } else if host.contains("youtu.be") {
                // youtu.be/VIDEO_ID 형식
                let path = parsed_url.path();
                if path.len() > 1 {
                    return Some(path[1..].to_string());
                }
            }
        }
    }
    
    // URL이 아닌 경우 Video ID로 간주하고 유효성 검사
    if url.len() == 11 && url.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Some(url.to_string());
    }
    
    None
}

/// 영상 다운로드 메인 함수
async fn download_videos(urls: Vec<String>, cli: &Cli, fetcher: &Youtube) -> Result<()> {
    use futures_util::stream;
    
    // 동시 다운로드 제한을 위한 세마포어
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(cli.concurrent));
    
    // 모든 다운로드 작업을 스트림으로 변환
    let download_tasks = stream::iter(urls.into_iter().enumerate())
        .map(|(index, url)| {
            let semaphore = semaphore.clone();
            let cli_clone = cli.clone();
            
            async move {
                let _permit = semaphore.acquire().await.unwrap();
                download_single_video(url, index + 1, &cli_clone, fetcher).await
            }
        })
        .buffer_unordered(cli.concurrent);
    
    // 모든 다운로드 작업 실행
    let results: Vec<DownloadResult> = download_tasks.collect().await;
    
    // 결과 분석
    let successful_results: Vec<&DownloadResult> = results.iter().filter(|r| r.success).collect();
    let failed_results: Vec<&DownloadResult> = results.iter().filter(|r| !r.success).collect();
    
    // 결과 요약 출력
    println!("\n📊 다운로드 결과:");
    println!("  성공: {} 개", successful_results.len());
    
    // 성공한 다운로드 목록 출력
    if !successful_results.is_empty() {
        println!("\n✅ 성공한 다운로드 목록:");
        for (i, result) in successful_results.iter().enumerate() {
            if let Some(title) = &result.title {
                println!("  {}. {}", i + 1, title);
                if let Some(path) = &result.file_path {
                    println!("     📂 저장 위치: {}", path.display());
                }
            }
        }
    }
    
    // 실패한 다운로드 목록 출력
    if !failed_results.is_empty() {
        println!("  실패: {} 개", failed_results.len());
        println!("\n❌ 실패한 다운로드 목록:");
        for (i, result) in failed_results.iter().enumerate() {
            println!("  {}. URL: {}", i + 1, result.url);
            if let Some(error) = &result.error {
                println!("     원인: {}", error);
            }
            println!();
        }
    }
    
    Ok(())
}

/// 단일 영상 다운로드
async fn download_single_video(url: String, index: usize, cli: &Cli, fetcher: &Youtube) -> DownloadResult {
    println!("[{}] 영상 정보를 가져오는 중: {}", index, url);
    
    // 재시도 로직을 위한 상수
    const MAX_RETRIES: usize = 3;
    const RETRY_DELAY_MS: u64 = 2000;
    
    for attempt in 1..=MAX_RETRIES {
        match download_attempt(&url, index, cli, fetcher, attempt).await {
            Ok(result) => return result,
            Err(e) => {
                let error_str = e.to_string().to_lowercase();
                
                // 재시도할 수 없는 오류들
                if error_str.contains("private") || 
                   error_str.contains("deleted") || 
                   error_str.contains("unavailable") ||
                   error_str.contains("copyright") {
                    println!("[{}] ❌ 재시도 불가능한 오류: {}", index, e);
                    return DownloadResult::failure(url, format!("재시도 불가능한 오류: {}", e));
                }
                
                if attempt < MAX_RETRIES {
                    println!("[{}] ⚠️  시도 {}/{}에서 실패, {}초 후 재시도: {}", 
                             index, attempt, MAX_RETRIES, RETRY_DELAY_MS / 1000, e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(RETRY_DELAY_MS)).await;
                } else {
                    println!("[{}] ❌ 모든 재시도 실패: {}", index, e);
                    return DownloadResult::failure(url, format!("최종 실패 ({}회 시도): {}", MAX_RETRIES, e));
                }
            }
        }
    }
    
    DownloadResult::failure(url, "알 수 없는 오류".to_string())
}

/// 단일 다운로드 시도
async fn download_attempt(url: &str, index: usize, cli: &Cli, fetcher: &Youtube, attempt: usize) -> Result<DownloadResult> {
    if attempt > 1 {
        println!("[{}] 시도 {}: {}", index, attempt, url);
    }
    
    // 영상 정보 가져오기
    let video_info = match fetcher.fetch_video_infos(url.to_string()).await {
        Ok(info) => info,
        Err(e) => {
            let error_str = e.to_string().to_lowercase();
            
            // 자막 관련 JSON 파싱 오류 감지
            if error_str.contains("unknown variant") && error_str.contains("srt") && 
               (error_str.contains("json3") || error_str.contains("vtt") || error_str.contains("ttml")) {
                let error_msg = format!(
                    "자막 형식 호환 문제 감지: 이 영상에는 지원되지 않는 자막 형식(SRT)이 포함되어 있습니다. \
                     현재 yt-dlp 크레이트에서 SRT 자막이 완전히 지원되지 않아 발생하는 문제입니다. \
                     해결책: 다른 영상을 시도하거나 --skip-subtitles 옵션을 사용하세요. 원본 오류: {}", e);
                println!("[{}] ⚠️  {}", index, error_msg);
                return Err(anyhow::anyhow!(error_msg));
            }
            
            let error_msg = format!("영상 정보 로드 실패: {}", e);
            println!("[{}] ❌ {}", index, error_msg);
            return Err(anyhow::anyhow!(error_msg));
        }
    };
    
    let title = sanitize_filename(&video_info.title);
    let extension = if cli.audio_only { "mp3" } else { "mp4" };
    let filename = format!("{}.{}", title, extension);
    let file_path = Path::new(&cli.output).join(&filename);
    
    // 파일이 이미 존재하는지 확인
    if file_path.exists() {
        let error_msg = format!("파일이 이미 존재합니다: {}", filename);
        println!("[{}] ⚠️  {}", index, error_msg);
        return Ok(DownloadResult::failure(url.to_string(), error_msg));
    }
    
    println!("[{}] 다운로드 시작: {}", index, title);
    
    // 진행률 바 설정
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{bar:40.cyan/blue}] {pos}% ({msg})")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message(format!("[{}] {}", index, title));
    
    // 품질 및 코덱 설정
    let video_quality = parse_video_quality(&cli.quality);
    let audio_quality = parse_audio_quality(&cli.audio_quality);
    let video_codec = parse_video_codec(&cli.video_codec);
    let audio_codec = parse_audio_codec(&cli.audio_codec);
    
    // 다운로드 실행
    let download_result = if cli.audio_only {
        // 오디오만 다운로드
        fetcher.download_audio_stream_with_quality(
            url.to_string(),
            &filename,
            audio_quality,
            audio_codec
        ).await
    } else {
        // 비디오 + 오디오 다운로드
        fetcher.download_video_with_quality(
            url.to_string(),
            &filename,
            video_quality,
            video_codec,
            audio_quality,
            audio_codec
        ).await
    };
    
    match download_result {
        Ok(downloaded_path) => {
            pb.finish_with_message(format!("[{}] ✅ 완료: {}", index, title));
            Ok(DownloadResult::success(url.to_string(), title, downloaded_path))
        }
        Err(e) => {
            pb.finish_with_message(format!("[{}] ❌ 실패: {}", index, title));
            
            // 구체적인 에러 원인 분석
            let error_str = e.to_string().to_lowercase();
            let categorized_error = if error_str.contains("source empty") || error_str.contains("no formats") {
                format!("영상 소스를 찾을 수 없음: 지역 제한, 연령 제한, 또는 특수 영상 형식일 수 있습니다. 원본 오류: {}", e)
            } else if error_str.contains("network") || error_str.contains("connection") {
                format!("네트워크 연결 오류: {}", e)
            } else if error_str.contains("permission") || error_str.contains("access") {
                format!("파일 쓰기 권한 오류: {}", e)
            } else if error_str.contains("space") || error_str.contains("disk") {
                format!("디스크 공간 부족: {}", e)
            } else if error_str.contains("unavailable") || error_str.contains("private") {
                format!("영상을 사용할 수 없음 (비공개/삭제됨): {}", e)
            } else if error_str.contains("age") || error_str.contains("restricted") {
                format!("연령 제한 또는 지역 제한: {}", e)
            } else if error_str.contains("live") {
                format!("라이브 스트림은 지원되지 않습니다: {}", e)
            } else if error_str.contains("premiere") {
                format!("프리미어 영상은 아직 지원되지 않습니다: {}", e)
            } else {
                format!("다운로드 실패: {}", e)
            };
            
            println!("[{}] 🔍 상세 원인: {}", index, categorized_error);
            Err(anyhow::anyhow!(categorized_error))
        }
    }
}

/// 비디오 품질 파싱
fn parse_video_quality(quality: &str) -> VideoQuality {
    match quality.to_lowercase().as_str() {
        "best" => VideoQuality::Best,
        "high" => VideoQuality::High,
        "medium" => VideoQuality::Medium,
        "low" => VideoQuality::Low,
        "worst" => VideoQuality::Worst,
        _ => VideoQuality::High,
    }
}

/// 오디오 품질 파싱
fn parse_audio_quality(quality: &str) -> AudioQuality {
    match quality.to_lowercase().as_str() {
        "best" => AudioQuality::Best,
        "high" => AudioQuality::High,
        "medium" => AudioQuality::Medium,
        "low" => AudioQuality::Low,
        "worst" => AudioQuality::Worst,
        _ => AudioQuality::High,
    }
}

/// 비디오 코덱 파싱
fn parse_video_codec(codec: &str) -> VideoCodecPreference {
    match codec.to_lowercase().as_str() {
        "vp9" => VideoCodecPreference::VP9,
        "avc1" | "h264" => VideoCodecPreference::AVC1,
        "av1" => VideoCodecPreference::AV1,
        "any" => VideoCodecPreference::Any,
        _ => VideoCodecPreference::Any,
    }
}

/// 오디오 코덱 파싱
fn parse_audio_codec(codec: &str) -> AudioCodecPreference {
    match codec.to_lowercase().as_str() {
        "opus" => AudioCodecPreference::Opus,
        "aac" => AudioCodecPreference::AAC,
        "mp3" => AudioCodecPreference::MP3,
        "any" => AudioCodecPreference::Any,
        _ => AudioCodecPreference::Any,
    }
}

/// 파일명에 사용할 수 없는 문자 제거
fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

// CLI 구조체 복제 지원
impl Clone for Cli {
    fn clone(&self) -> Self {
        Self {
            urls: self.urls.clone(),
            quality: self.quality.clone(),
            output: self.output.clone(),
            concurrent: self.concurrent,
            audio_only: self.audio_only,
            verbose: self.verbose,
            audio_quality: self.audio_quality.clone(),
            video_codec: self.video_codec.clone(),
            audio_codec: self.audio_codec.clone(),
            skip_subtitles: self.skip_subtitles,
        }
    }
}
