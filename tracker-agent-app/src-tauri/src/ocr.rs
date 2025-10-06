use image::DynamicImage;
use tesseract::Tesseract;
use tracing::info;
use std::path::PathBuf;

/// Získání debug adresáře pro ukládání screenshotů
/// Ukládá do tracker-agent-app/debug_screenshots/ (mimo src-tauri aby nerestartoval watch)
fn get_debug_dir() -> PathBuf {
    let mut path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Pokud jsme v src-tauri, jdi o úroveň výš
    if path.ends_with("src-tauri") {
        path.pop();
    }

    path.push("debug_screenshots");

    // Vytvoř adresář pokud neexistuje
    if !path.exists() {
        std::fs::create_dir_all(&path).ok();
    }

    path
}

/// Zkontroluje zda je Tesseract nainstalovaný
fn check_tesseract_installed() -> bool {
    std::process::Command::new("tesseract")
        .arg("--version")
        .output()
        .is_ok()
}

/// Pokusí se automaticky nainstalovat Tesseract
fn auto_install_tesseract() -> Result<(), String> {
    info!("⚠️  Tesseract není nainstalovaný, pokouším se o automatickou instalaci...");

    #[cfg(target_os = "macos")]
    {
        info!("🍎 Detekován macOS, instaluji přes Homebrew...");
        let output = std::process::Command::new("brew")
            .args(&["install", "tesseract", "tesseract-lang"])
            .output()
            .map_err(|e| format!("Chyba při spuštění brew: {}. Nainstalujte Homebrew z https://brew.sh", e))?;

        if !output.status.success() {
            return Err(format!("Instalace selhala: {}", String::from_utf8_lossy(&output.stderr)));
        }

        info!("✅ Tesseract úspěšně nainstalován!");
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        info!("🐧 Detekován Linux, instaluji přes apt-get...");
        let output = std::process::Command::new("sudo")
            .args(&["apt-get", "update"])
            .output()
            .map_err(|e| format!("Chyba při aktualizaci apt: {}", e))?;

        if !output.status.success() {
            return Err("Aktualizace apt selhala".to_string());
        }

        let output = std::process::Command::new("sudo")
            .args(&["apt-get", "install", "-y", "tesseract-ocr", "tesseract-ocr-eng", "libtesseract-dev", "libleptonica-dev"])
            .output()
            .map_err(|e| format!("Chyba při instalaci tesseract: {}", e))?;

        if !output.status.success() {
            return Err(format!("Instalace selhala: {}", String::from_utf8_lossy(&output.stderr)));
        }

        info!("✅ Tesseract úspěšně nainstalován!");
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        Err("Automatická instalace na Windows není podporována. Prosím nainstalujte Tesseract manuálně z https://github.com/UB-Mannheim/tesseract/wiki".to_string())
    }
}

/// Provede OCR na obrázku pomocí Tesseract
fn perform_ocr(img_buffer: &[u8]) -> Result<String, String> {
    // Zkontroluj zda je Tesseract nainstalovaný
    if !check_tesseract_installed() {
        // Pokus o automatickou instalaci
        auto_install_tesseract()?;

        // Znovu zkontroluj
        if !check_tesseract_installed() {
            return Err("Tesseract se nepodařilo nainstalovat. Prosím nainstalujte ho manuálně.".to_string());
        }
    }

    let mut tesseract = Tesseract::new(None, Some("eng"))
        .map_err(|e| format!("Chyba při inicializaci Tesseract: {}", e))?
        .set_variable("tessedit_pageseg_mode", "11")
        .map_err(|e| format!("Chyba při nastavení PSM: {}", e))?
        .set_image_from_mem(img_buffer)
        .map_err(|e| format!("Chyba při načítání obrazu: {}", e))?;

    tesseract
        .get_text()
        .map_err(|e| format!("OCR selhal: {}", e))
}

/// Extrakce textu z obrázku pomocí Tesseract OCR
pub fn extract_text_from_image(img: DynamicImage, save_debug: bool) -> Result<String, String> {
    info!("📖 OCR: Spouštím Tesseract...");

    // Debug: Uložení původního screenshotu
    if save_debug {
        let debug_dir = get_debug_dir();
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let path = debug_dir.join(format!("{}_0_original.png", timestamp));
        if let Err(e) = img.save(&path) {
            info!("⚠️  Nepodařilo se uložit original: {}", e);
        } else {
            info!("💾 Debug: Uloženo original -> {:?}", path);
        }
    }

    // Konverze do PNG bufferu pro Tesseract
    info!("🔧 OCR: Konvertuji do PNG pro Tesseract...");
    let mut buffer = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut buffer), image::ImageFormat::Png)
        .map_err(|e| format!("Chyba při konverzi obrazu: {}", e))?;

    // OCR pomocí Tesseract (s automatickou instalací)
    info!("🔧 OCR: Spouštím Tesseract OCR (PSM 11)...");

    let text = perform_ocr(&buffer)
        .map_err(|e| format!("OCR selhal: {}", e))?;

    info!("✅ OCR: Extrahováno {} znaků", text.len());

    // Debug: Výpis extrahovaného textu
    if save_debug {
        info!("📝 OCR Text (prvních 500 znaků):");
        info!("─────────────────────────────────────");
        // Bezpečné oříznutí na 500 znaků (respektuje UTF-8 boundaries)
        let preview = if text.chars().count() > 500 {
            let truncated: String = text.chars().take(500).collect();
            format!("{}...", truncated)
        } else {
            text.clone()
        };
        for line in preview.lines() {
            info!("  {}", line);
        }
        info!("─────────────────────────────────────");

        // Uložení textu do souboru
        let debug_dir = get_debug_dir();
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let path = debug_dir.join(format!("{}_4_ocr_text.txt", timestamp));
        if let Err(e) = std::fs::write(&path, &text) {
            info!("⚠️  Nepodařilo se uložit OCR text: {}", e);
        } else {
            info!("💾 Debug: Uložen OCR text -> {:?}", path);
        }
    }

    Ok(text)
}

/// Extrakce textu ze screenshotu (base64)
/// save_debug: pokud true, ukládá mezikroky do debug_screenshots/
pub fn extract_text_from_screenshot(screenshot_base64: &str, save_debug: bool) -> Result<String, String> {
    use base64::Engine;

    info!("🔍 OCR: Začínám zpracování screenshotu (debug={})", save_debug);

    // Dekódování base64
    let image_data = base64::engine::general_purpose::STANDARD
        .decode(screenshot_base64)
        .map_err(|e| format!("Chyba při dekódování base64: {}", e))?;

    info!("📦 OCR: Dekódováno {} bytů", image_data.len());

    // Načtení obrazu
    let img = image::load_from_memory(&image_data)
        .map_err(|e| format!("Chyba při načítání obrazu: {}", e))?;

    info!("🖼️  OCR: Načten obrázek {}x{}", img.width(), img.height());

    // OCR
    extract_text_from_image(img, save_debug)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preprocessing() {
        // Vytvoř testovací obrázek
        let img = DynamicImage::new_rgb8(100, 100);
        let processed = preprocess_image(img, false); // false = bez debug ukládání

        assert_eq!(processed.width(), 100);
        assert_eq!(processed.height(), 100);
    }
}

