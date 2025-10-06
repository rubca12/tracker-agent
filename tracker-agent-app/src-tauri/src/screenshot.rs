use base64::{engine::general_purpose, Engine as _};
use image::ImageFormat;
use std::io::Cursor;
use tracing::info;
use xcap::Monitor;

/// Zachytí celou obrazovku
pub fn capture_and_encode() -> Result<String, String> {
    info!("🔍 Screenshot: Získávám seznam monitorů pomocí xcap...");

    // Get all monitors
    let monitors = Monitor::all().map_err(|e| {
        let err_msg = format!("Failed to get monitors: {}. DŮLEŽITÉ: Aplikace potřebuje Screen Recording permission!", e);
        info!("❌ {}", err_msg);
        err_msg
    })?;

    // Get primary monitor, fallback to first monitor
    let monitor = monitors
        .into_iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .or_else(|| Monitor::all().ok()?.into_iter().next())
        .ok_or_else(|| {
            let err_msg = "No monitors found".to_string();
            info!("❌ {}", err_msg);
            err_msg
        })?;

    let monitor_name = monitor.name().unwrap_or_else(|_| "Unknown".to_string());
    let monitor_width = monitor.width().unwrap_or(0);
    let monitor_height = monitor.height().unwrap_or(0);

    info!("📸 Screenshot: Zachytávám monitor '{}' ({}x{})...",
        monitor_name, monitor_width, monitor_height);

    // Capture screenshot
    let image = monitor.capture_image().map_err(|e| {
        let err_msg = format!("Failed to capture monitor: {}", e);
        info!("❌ {}", err_msg);
        err_msg
    })?;

    info!("✅ Screenshot: Zachyceno {}x{} pixelů", image.width(), image.height());

    // xcap vrací RgbaImage, konvertujeme na DynamicImage
    let img = image::DynamicImage::ImageRgba8(image);

    info!("📦 Screenshot: Kóduji do JPEG...");

    // Encode to JPEG
    let mut buffer = Cursor::new(Vec::new());
    img.write_to(&mut buffer, ImageFormat::Jpeg)
        .map_err(|e| format!("Failed to encode image: {}", e))?;

    // Base64 encode
    let base64_string = general_purpose::STANDARD.encode(buffer.into_inner());

    info!("✅ Screenshot: Hotovo ({} bytů base64)", base64_string.len());

    Ok(base64_string)
}
