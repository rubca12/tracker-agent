# Tracker Agent - Instalační instrukce

## Automatická instalace (Doporučeno)

Aplikace **automaticky nainstaluje Tesseract OCR** při prvním spuštění, pokud není nalezen v systému.

### macOS

Aplikace automaticky použije Homebrew k instalaci Tesseract. Pokud nemáte Homebrew nainstalovaný:

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

### Linux (Ubuntu/Debian)

Aplikace automaticky použije `apt-get` k instalaci Tesseract. Může vyžadovat sudo oprávnění.

### Windows

**Automatická instalace není podporována.** Prosím nainstalujte Tesseract manuálně:

1. Stáhněte Tesseract installer z: https://github.com/UB-Mannheim/tesseract/wiki
2. Spusťte installer a nainstalujte Tesseract
3. Přidejte Tesseract do PATH:
   - Otevřete "Environment Variables"
   - Přidejte `C:\Program Files\Tesseract-OCR` do PATH
4. Restartujte aplikaci

## Manuální instalace (Volitelné)

Pokud chcete nainstalovat Tesseract před prvním spuštěním aplikace:

### macOS

```bash
brew install tesseract tesseract-lang
```

### Ubuntu/Debian

```bash
sudo apt-get update
sudo apt-get install tesseract-ocr tesseract-ocr-eng libtesseract-dev libleptonica-dev
```

### Ověření instalace

```bash
tesseract --version
```

Měli byste vidět verzi Tesseract (např. `tesseract 5.3.0`).

## Kompilace aplikace

### Development

```bash
cd tracker-agent-app
npm install
npm run tauri dev
```

### Production build

```bash
cd tracker-agent-app
npm run tauri build
```

Binárka bude v `src-tauri/target/release/`.

## Poznámky

- **macOS**: Aplikace vyžaduje Screen Recording permission (systém se zeptá při prvním spuštění)
- **Windows**: Může být potřeba spustit jako administrátor pro screenshot permissions
- **Linux**: Může být potřeba nastavit X11/Wayland permissions

## Troubleshooting

### "Tesseract not found"

Ujistěte se že:
1. Tesseract je nainstalovaný (`tesseract --version`)
2. Tesseract je v PATH
3. Restartovali jste terminál po instalaci

### "Failed to load Tesseract"

Na Windows zkontrolujte že:
1. Tesseract je v `C:\Program Files\Tesseract-OCR`
2. PATH obsahuje cestu k Tesseract
3. Máte nainstalovaný English language pack

### Screenshot je prázdný (macOS)

1. Otevřete System Settings > Privacy & Security > Screen Recording
2. Povolte aplikaci "tracker-agent-app"
3. Restartujte aplikaci

