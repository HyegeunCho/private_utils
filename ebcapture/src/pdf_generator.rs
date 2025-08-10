use crate::error::{EbCaptureError, Result};
use log::{info, debug};
use printpdf::{PdfDocument, PdfDocumentReference, PdfLayerReference, Mm, Image, ImageTransform};
use image::{DynamicImage, io::Reader};
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::BufWriter;

/// 캡쳐된 이미지들을 단일 PDF로 통합합니다
pub async fn create_pdf(image_paths: &[PathBuf], output_dir: &Path) -> Result<PathBuf> {
    info!("PDF 생성 시작: {} 페이지", image_paths.len());
    
    if image_paths.is_empty() {
        return Err(EbCaptureError::PdfGenerationFailure { 
            reason: "생성할 이미지가 없습니다".to_string() 
        });
    }
    
    // PDF 파일명 생성
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let pdf_filename = format!("captured_book_{}.pdf", timestamp);
    let pdf_path = output_dir.join(pdf_filename);
    
    // 첫 번째 이미지를 로드하여 문서 크기 결정
    let first_image = load_image(&image_paths[0])?;
    let (doc_width, doc_height) = calculate_document_size_from_image(&first_image);
    
    info!("PDF 문서 크기: {}mm x {}mm (300 DPI 기준)", doc_width.0, doc_height.0);
    
    // PDF 문서 생성
    let (doc, page1, layer1) = PdfDocument::new("Captured EBook", doc_width, doc_height, "Layer 1");
    let mut current_layer = doc.get_page(page1).get_layer(layer1);
    
    // 첫 번째 이미지 추가 (스케일 없이 그대로)
    add_image_to_pdf(&first_image, &mut current_layer, doc_width, doc_height)?;
    
    // 나머지 이미지들을 새 페이지로 추가
    for (index, image_path) in image_paths.iter().skip(1).enumerate() {
        debug!("PDF에 이미지 추가: {} ({}/{})", 
               image_path.display(), index + 2, image_paths.len());
        
        let image = load_image(image_path)?;
        
        // 새 페이지 추가
        let (page_index, layer_index) = doc.add_page(doc_width, doc_height, "Layer 1");
        let mut layer = doc.get_page(page_index).get_layer(layer_index);
        
        // 이미지를 페이지에 추가
        add_image_to_pdf(&image, &mut layer, doc_width, doc_height)?;
    }
    
    // PDF 파일 저장
    save_pdf_document(doc, &pdf_path)?;
    
    info!("PDF 생성 완료: {}", pdf_path.display());
    Ok(pdf_path)
}

fn load_image(image_path: &Path) -> Result<DynamicImage> {
    Reader::open(image_path)
        .map_err(|e| EbCaptureError::PdfGenerationFailure { 
            reason: format!("이미지 파일 열기 실패 {}: {}", image_path.display(), e) 
        })?
        .decode()
        .map_err(|e| EbCaptureError::PdfGenerationFailure { 
            reason: format!("이미지 디코딩 실패 {}: {}", image_path.display(), e) 
        })
}

fn calculate_document_size_from_image(image: &DynamicImage) -> (Mm, Mm) {
    // 이미지 크기를 기반으로 PDF 페이지 크기 계산 (300 DPI 기준)
    let width = image.width() as f64;
    let height = image.height() as f64;
    
    // 300 DPI로 mm 단위 변환
    let dpi = 300.0;
    let mm_per_inch = 25.4;
    
    let width_mm = width / dpi * mm_per_inch;
    let height_mm = height / dpi * mm_per_inch;
    
    debug!("이미지 {}x{}px -> 문서 {:.1}x{:.1}mm (300 DPI)", 
           width, height, width_mm, height_mm);
    
    (Mm(width_mm as f32), Mm(height_mm as f32))
}

fn add_image_to_pdf(
    image: &DynamicImage,
    layer: &mut PdfLayerReference,
    doc_width: Mm,
    doc_height: Mm,
) -> Result<()> {
    // 이미지를 RGB로 변환
    let rgb_image = image.to_rgb8();
    let width = rgb_image.width();
    let height = rgb_image.height();
    
    debug!("이미지 추가: {}x{}px -> PDF {}x{}mm (1:1 매핑)", 
           width, height, doc_width.0, doc_height.0);
    
    // printpdf 0.6 방식: ImageXObject 직접 생성
    use printpdf::{ImageXObject, ColorSpace, ColorBits, Px};
    
    let image_data = rgb_image.into_raw();
    
    let image_object = ImageXObject {
        width: Px(width as usize),
        height: Px(height as usize),
        color_space: ColorSpace::Rgb,
        bits_per_component: ColorBits::Bit8,
        interpolate: true,
        image_data,
        image_filter: None,
        clipping_bbox: None,
    };
    
    let pdf_image = Image::from(image_object);
    
    // 간단한 변환: 스케일 없이 좌하단 (0,0)에 배치
    // PDF 문서 크기가 이미지 크기에 맞춰져 있으므로 변환 불필요
    let transform = ImageTransform {
        translate_x: Some(Mm(0.0)), // 좌하단 시작
        translate_y: Some(Mm(0.0)),
        scale_x: Some(1.0),        // 1:1 스케일
        scale_y: Some(1.0),
        rotate: None,
        dpi: Some(300.0),          // 300 DPI 설정
    };
    
    // PDF 레이어에 이미지 추가
    pdf_image.add_to_layer(layer.clone(), transform);
    
    Ok(())
}

fn save_pdf_document(doc: PdfDocumentReference, pdf_path: &Path) -> Result<()> {
    let file = File::create(pdf_path)
        .map_err(|e| EbCaptureError::PdfGenerationFailure { 
            reason: format!("PDF 파일 생성 실패: {}", e) 
        })?;
    
    let mut writer = BufWriter::new(file);
    
    doc.save(&mut writer)
        .map_err(|e| EbCaptureError::PdfGenerationFailure { 
            reason: format!("PDF 저장 실패: {}", e) 
        })?;
    
    Ok(())
} 