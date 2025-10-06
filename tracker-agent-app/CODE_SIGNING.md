# Code Signing & Distribution

## ⚠️ Aktuální stav

Aplikace **není podepsaná** Apple certifikátem, proto macOS zobrazuje varování "aplikace je poškozena".

## 🔧 Pro testování (obejití Gatekeeper)

### Metoda 1: Odstranění quarantine flagu
```bash
xattr -cr /Applications/tracker-agent-app.app
```

### Metoda 2: System Settings
1. Zkus otevřít aplikaci (dostaneš chybu)
2. **System Settings → Privacy & Security**
3. Klikni **"Open Anyway"**

### Metoda 3: Ctrl+Click
1. Pravý klik na aplikaci
2. **"Open"**
3. Potvrď **"Open"**

## 🔐 Pro distribuci (code signing)

### Požadavky:
1. **Apple Developer Account** ($99/rok)
2. **Developer ID Application Certificate**

### Postup:

#### 1. Získej certifikát
```bash
# V Keychain Access:
# Certificate Assistant → Request a Certificate from a Certificate Authority
# Nahraj na: https://developer.apple.com/account/resources/certificates/add
```

#### 2. Nastav environment variables
```bash
export APPLE_CERTIFICATE="Developer ID Application: Jan Rubeš (TEAM_ID)"
export APPLE_CERTIFICATE_PASSWORD="your-password"
export APPLE_SIGNING_IDENTITY="Developer ID Application: Jan Rubeš (TEAM_ID)"
export APPLE_ID="jan@digihood.cz"
export APPLE_PASSWORD="app-specific-password"
export APPLE_TEAM_ID="YOUR_TEAM_ID"
```

#### 3. Přidej do tauri.conf.json
```json
{
  "bundle": {
    "macOS": {
      "signingIdentity": "Developer ID Application: Jan Rubeš (TEAM_ID)",
      "entitlements": null,
      "exceptionDomain": null,
      "frameworks": [],
      "providerShortName": null,
      "minimumSystemVersion": "10.13"
    }
  }
}
```

#### 4. Build s podpisem
```bash
npm run tauri build
```

#### 5. Notarize (povinné pro macOS 10.15+)
```bash
# Automaticky při buildu s nastavenými env variables
# Nebo ručně:
xcrun notarytool submit tracker-agent-app.dmg \
  --apple-id "jan@digihood.cz" \
  --password "app-specific-password" \
  --team-id "YOUR_TEAM_ID" \
  --wait
```

## 🚀 Alternativy (bez Apple Developer Account)

### 1. Self-signed certificate (pouze pro interní použití)
```bash
# Vytvoř self-signed certifikát
# Uživatelé musí stejně povolit v System Settings
```

### 2. Distribuce přes Homebrew
```bash
# Vytvoř Homebrew cask
# Uživatelé instalují přes: brew install --cask tracker-agent
```

### 3. Instrukce pro uživatele
Přilož README s instrukcemi jak obejít Gatekeeper (viz výše).

## 📝 Doporučení

**Pro interní testování:**
- Použij `xattr -cr` příkaz
- Nebo instrukce pro kolegy

**Pro veřejnou distribuci:**
- Investuj do Apple Developer Account
- Nastav code signing + notarization
- Nebo použij Homebrew

## 🔗 Užitečné odkazy

- [Apple Developer Program](https://developer.apple.com/programs/)
- [Tauri Code Signing](https://tauri.app/v1/guides/distribution/sign-macos)
- [Notarization Guide](https://developer.apple.com/documentation/security/notarizing_macos_software_before_distribution)

