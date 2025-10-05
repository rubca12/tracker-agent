# Build Instructions

## 📦 Produkční Build

### Automatický build (GitHub Actions)

Při push do `main` nebo vytvoření tagu `v*` se automaticky spustí build pro všechny platformy:
- **Linux**: `.deb`, `.AppImage`
- **Windows**: `.msi`, `.exe` (NSIS installer)
- **macOS**: `.app`, `.dmg` (Intel i ARM)

Výsledky najdeš v GitHub Actions artifacts nebo v GitHub Releases (při tagu).

### Manuální build

#### macOS (aktuální platforma)

```bash
cd tracker-agent-app
npm run tauri build
```

**Výstup:**
- `src-tauri/target/release/tracker-agent-app` - spustitelný soubor
- `src-tauri/target/release/bundle/macos/tracker-agent-app.app` - macOS aplikace
- `src-tauri/target/release/bundle/dmg/tracker-agent-app_0.1.0_aarch64.dmg` - DMG installer

#### Windows (cross-compile není podporován)

Na Windows stroji:
```bash
cd tracker-agent-app
npm install
npm run tauri build
```

**Výstup:**
- `src-tauri/target/release/tracker-agent-app.exe`
- `src-tauri/target/release/bundle/msi/tracker-agent-app_0.1.0_x64_en-US.msi`
- `src-tauri/target/release/bundle/nsis/tracker-agent-app_0.1.0_x64-setup.exe`

#### Linux (cross-compile není podporován)

Na Ubuntu/Debian:
```bash
# Instalace závislostí
sudo apt-get update
sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev \
  libappindicator3-dev librsvg2-dev patchelf \
  libx11-dev libxcb1-dev libxcb-render0-dev \
  libxcb-shape0-dev libxcb-xfixes0-dev

# Build
cd tracker-agent-app
npm install
npm run tauri build
```

**Výstup:**
- `src-tauri/target/release/tracker-agent-app`
- `src-tauri/target/release/bundle/deb/tracker-agent-app_0.1.0_amd64.deb`
- `src-tauri/target/release/bundle/appimage/tracker-agent-app_0.1.0_amd64.AppImage`

## 🚀 Vytvoření Release

1. **Vytvoř tag:**
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

2. **GitHub Actions automaticky:**
   - Buildne pro všechny platformy
   - Vytvoří GitHub Release
   - Nahraje všechny instalátory

## 🛠️ Development Build

```bash
cd tracker-agent-app
npm run tauri dev
```

## 📋 Požadavky

### Všechny platformy
- Node.js 20+
- Rust 1.70+

### Linux
- GTK 3
- WebKit2GTK 4.1
- AppIndicator3
- librsvg2
- X11 libraries

### Windows
- Visual Studio Build Tools
- WebView2 (automaticky nainstalováno)

### macOS
- Xcode Command Line Tools

