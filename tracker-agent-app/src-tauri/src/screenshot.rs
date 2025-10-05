use base64::{engine::general_purpose, Engine as _};
use image::ImageFormat;
use screenshots::Screen;
use std::io::Cursor;

pub fn capture_and_encode() -> Result<String, String> {
    // Get first screen
    let screens = Screen::all().map_err(|e| format!("Failed to get screens: {}", e))?;
    let screen = screens
        .into_iter()
        .next()
        .ok_or("No screens found")?;

    // Capture screenshot
    let image = screen
        .capture()
        .map_err(|e| format!("Failed to capture screen: {}", e))?;

    // Convert to DynamicImage
    let img = image::DynamicImage::ImageRgba8(
        image::RgbaImage::from_raw(
            image.width(),
            image.height(),
            image.to_vec(),
        )
        .ok_or("Failed to create image from buffer")?,
    );

    // Resize if too large (max width 1920px)
    let img = if img.width() > 1920 {
        img.resize(1920, (1920 * img.height()) / img.width(), image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    // Encode to JPEG
    let mut buffer = Cursor::new(Vec::new());
    img.write_to(&mut buffer, ImageFormat::Jpeg)
        .map_err(|e| format!("Failed to encode image: {}", e))?;

    // Base64 encode
    let base64_string = general_purpose::STANDARD.encode(buffer.into_inner());

    Ok(base64_string)
}

