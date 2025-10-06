mod freelo;
mod screenshot;
mod tracker;
mod ocr;
mod text_matcher;
mod ai_matcher;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tracker::{Tracker, TrackerConfig};

// --- Data Structures ---

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Settings {
    interval: u64,
    freelo_email: String,
    freelo_key: String,
    openrouter_key: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct LogEvent {
    level: String,
    message: String,
}

// --- Application State ---

struct AppState {
    tracker: Arc<Tracker>,
}

// --- Tauri Commands ---

#[tauri::command]
async fn start_tracking(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    state.tracker.start(app).await
}

#[tauri::command]
async fn stop_tracking(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    state.tracker.stop(app).await
}

#[tauri::command]
async fn save_settings(
    state: tauri::State<'_, AppState>,
    settings: Settings,
    app: AppHandle,
) -> Result<(), String> {
    // Convert to TrackerConfig
    let config = TrackerConfig {
        interval_seconds: settings.interval,
        freelo_email: settings.freelo_email.clone(),
        freelo_api_key: settings.freelo_key.clone(),
        openrouter_api_key: settings.openrouter_key.clone(),
    };

    state.tracker.set_config(config).await;

    // Emit log event
    app.emit("log-event", LogEvent {
        level: "success".to_string(),
        message: format!("💾 Nastavení uloženo (interval: {}s)", settings.interval),
    }).map_err(|e| e.to_string())?;

    Ok(())
}

// --- Main Entry Point ---

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    tracing::info!("🚀 Tracker Agent starting...");

    let tracker = Arc::new(Tracker::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            tracker,
        })
        .invoke_handler(tauri::generate_handler![
            start_tracking,
            stop_tracking,
            save_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
