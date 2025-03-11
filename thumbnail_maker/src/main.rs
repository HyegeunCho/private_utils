use std::fs;
use std::path::Path;
use walkdir::WalkDir;
use webp::Encoder;
use clap::Parser;
use indicatif::ProgressBar;
use std::fs::File;
use image::imageops::FilterType;
use image::{GenericImageView, ImageEncoder};
use image::codecs::png::{PngEncoder, CompressionType, FilterType as PngFilter};


#[derive(Parser)]
struct Args {
    #[arg(long, default_value_t=String::from("./input"), help="Source image folder path")]
    input_path: String,
    #[arg(long, default_value_t=String::from("./output"), help="Path to save thumbnail images")]
    output_path: String,
    #[arg(long, default_value_t=35, help="Image size percent")]
    size_percent: u32,
    #[arg(long, default_value_t=80.0, help="Image quality")]
    quality: f32,
    #[arg(short, long, help="Compress png files")]
    png_compress: bool
}


fn main() {

    let args: Args = Args::parse();

    let is_png_compress = args.png_compress;

    let input_folder = &args.input_path;
    let output_folder = &args.output_path;
    
    let target_percent = args.size_percent;
    let quality = args.quality;

    fs::create_dir_all(output_folder).unwrap();


    let png_files = WalkDir::new(input_folder).into_iter().filter_map(|e| e.ok()).filter(|e| e.path().extension().map_or(false, |ext| ext == "png"));
    let png_file_count = png_files.count();
    
    if is_png_compress {
        println!("총 {png_file_count}개의 파일이 압축됩니다.");
        let pb = ProgressBar::new(png_file_count as u64);
        for entry in WalkDir::new(input_folder).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "png") {
                if let Err(e) = optimize_png(path, output_folder, CompressionType::Best) {
                    eprintln!("변환 실패: {:?}, 오류: {}", path, e);
                }
                pb.inc(1);
            }
        }
        println!("압축 완료");
    }
    else {
        println!("총 {png_file_count}개의 파일이 변환됩니다.");
        let pb = ProgressBar::new(png_file_count as u64);
        for entry in WalkDir::new(input_folder).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "png") {
                if let Err(e) = convert_png_to_webp(path, output_folder, target_percent, quality) {
                    eprintln!("변환 실패: {:?}, 오류: {}", path, e);
                }
                pb.inc(1);
            }
        }
        println!("변환 완료");
    }

    

}

fn optimize_png(input_path: &Path, output_path: &str, quality: CompressionType) -> Result<(), Box<dyn std::error::Error>> {
    // PNG 이미지 로드
    let img = image::open(input_path).expect("이미지를 불러올 수 없습니다.");

    let output_path = Path::new(output_path).join(input_path.file_name().unwrap()).with_extension("png");
    // PNG 저장 옵션 적용 (최적화 압축)
    let output_file = File::create(output_path).expect("출력 파일 생성 실패");
    let encoder = PngEncoder::new_with_quality(output_file, quality, PngFilter::Paeth);


    // 이미지 저장 (투명도 제거 시 RGB로 변환)
    let (w, h) = img.dimensions();
    let img_rgb = img.to_rgb8();
    encoder.write_image(&img_rgb, w, h, image::ExtendedColorType::Rgb8).expect("PNG 저장 실패");

    Ok(())
}


fn convert_png_to_webp(input_path: &Path, output_folder: &str, target_percent: u32, quality: f32) -> Result<(), Box<dyn std::error::Error>> {
    let img = image::open(input_path)?;

    let width = img.width() * target_percent / 100;
    let height = img.height() * target_percent / 100;

    let resized = img.resize_exact(width, height, FilterType::Lanczos3);

    let encoder = Encoder::from_image(&resized)?;
    let webp_data = encoder.encode(quality); // 품질 80%

    let output_path = Path::new(output_folder).join(input_path.file_name().unwrap()).with_extension("webp");
    fs::write(output_path, webp_data.to_vec())?;

    Ok(())
}
