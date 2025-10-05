# Migrace Tracker Agent do Tauri

## 🎯 Cíl
Přepsat konzolovou aplikaci `rust_tracker_agent` do plnohodnotné Tauri desktop aplikace s GUI.

## 📦 Struktura projektu

```
tracker-agent-app/
├── src/                    # Frontend (TypeScript + HTML + CSS)
│   ├── main.ts            # Hlavní TypeScript soubor
│   ├── styles.css         # Styly
│   └── components/        # UI komponenty
├── src-tauri/             # Backend (Rust)
│   ├── src/
│   │   ├── main.rs       # Tauri entry point
│   │   ├── tracker.rs    # Screenshot capture + tracking logika
│   │   ├── freelo.rs     # Freelo API integrace
│   │   └── ai.rs         # OpenRouter AI integrace
│   └── Cargo.toml
└── index.html             # Hlavní HTML
```

## 🔄 Migrace kroky

### Fáze 1: Základní struktura ✅
- [x] Vytvoření Tauri projektu
- [x] Přidání dependencies
- [ ] Vytvoření základního UI
- [ ] Systémový tray

### Fáze 2: Backend migrace
- [ ] Přesun screenshot capture logiky
- [ ] Přesun Freelo API integrace
- [ ] Přesun OpenRouter AI integrace
- [ ] Přesun tracking logiky

### Fáze 3: Frontend
- [ ] Zobrazení logů v reálném čase
- [ ] Nastavení (API klíče, interval)
- [ ] Přehled aktivního trackingu
- [ ] Historie událostí

### Fáze 4: Pokročilé funkce
- [ ] Notifikace
- [ ] Auto-start při spuštění systému
- [ ] Minimalizace do tray
- [ ] Export logů

## 🚀 Spuštění

```bash
cd tracker-agent-app
npm install
npm run tauri dev
```

## 📝 Poznámky

- Původní kód: `rust_tracker_agent/src/main.rs`
- Původní PWA server: `rust_pwa_server/src/main.rs`
- Tauri používá IPC (Inter-Process Communication) pro komunikaci frontend ↔ backend

