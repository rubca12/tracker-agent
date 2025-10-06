use crate::freelo::{ActiveTracking, FreeloClient, FreeloTask};
use crate::screenshot::capture_and_encode;
use crate::ocr::extract_text_from_screenshot;
use crate::text_matcher::{find_best_matching_task, MatchResult};
use crate::ai_matcher::match_task_with_ai;
use std::sync::Arc;
use std::time::SystemTime;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

#[derive(Clone)]
pub struct TrackerConfig {
    pub interval_seconds: u64,
    pub freelo_email: String,
    pub freelo_api_key: String,
    pub openrouter_api_key: Option<String>,
}

pub struct Tracker {
    config: Arc<Mutex<Option<TrackerConfig>>>,
    is_running: Arc<Mutex<bool>>,
    active_tracking: Arc<Mutex<Option<ActiveTracking>>>,
    freelo_tasks_cache: Arc<Mutex<Vec<FreeloTask>>>,
}

impl Tracker {
    pub fn new() -> Self {
        Self {
            config: Arc::new(Mutex::new(None)),
            is_running: Arc::new(Mutex::new(false)),
            active_tracking: Arc::new(Mutex::new(None)),
            freelo_tasks_cache: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn set_config(&self, config: TrackerConfig) {
        let mut cfg = self.config.lock().await;
        *cfg = Some(config);
    }

    pub async fn start(&self, app: AppHandle) -> Result<(), String> {
        let mut is_running = self.is_running.lock().await;
        if *is_running {
            return Err("Tracker už běží".to_string());
        }
        *is_running = true;
        drop(is_running);

        // Clone everything we need for the background task
        let config = self.config.clone();
        let is_running = self.is_running.clone();
        let active_tracking = self.active_tracking.clone();
        let freelo_tasks_cache = self.freelo_tasks_cache.clone();

        // Spawn background task
        tokio::spawn(async move {
            Self::tracking_loop(app, config, is_running, active_tracking, freelo_tasks_cache).await;
        });

        Ok(())
    }

    pub async fn stop(&self, app: AppHandle) -> Result<(), String> {
        let mut is_running = self.is_running.lock().await;
        if !*is_running {
            return Err("Tracker neběží".to_string());
        }
        *is_running = false;
        drop(is_running);

        // Stop active tracking if any
        let mut tracking = self.active_tracking.lock().await;
        if let Some(active) = tracking.take() {
            if let Some(cfg) = self.config.lock().await.as_ref() {
                let freelo = FreeloClient::new(
                    cfg.freelo_email.clone(),
                    cfg.freelo_api_key.clone(),
                );
                
                if let Err(e) = freelo.stop_tracking(&active.uuid).await {
                    Self::emit_log(&app, "error", &format!("Chyba při zastavení Freelo trackingu: {}", e));
                } else {
                    Self::emit_log(&app, "success", "Freelo tracking zastaven");
                }
            }
        }

        Ok(())
    }

    async fn tracking_loop(
        app: AppHandle,
        config: Arc<Mutex<Option<TrackerConfig>>>,
        is_running: Arc<Mutex<bool>>,
        active_tracking: Arc<Mutex<Option<ActiveTracking>>>,
        freelo_tasks_cache: Arc<Mutex<Vec<FreeloTask>>>,
    ) {
        // Get config
        let cfg = {
            let config_guard = config.lock().await;
            match config_guard.as_ref() {
                Some(c) => c.clone(),
                None => {
                    Self::emit_log(&app, "error", "Konfigurace není nastavena");
                    return;
                }
            }
        };

        let freelo = FreeloClient::new(cfg.freelo_email.clone(), cfg.freelo_api_key.clone());

        // Load Freelo tasks
        Self::emit_log(&app, "info", "Načítám Freelo tasky...");
        match freelo.get_active_tasks().await {
            Ok(tasks) => {
                let count = tasks.len();
                *freelo_tasks_cache.lock().await = tasks;
                Self::emit_log(&app, "success", &format!("Načteno {} aktivních tasků", count));
            }
            Err(e) => {
                Self::emit_log(&app, "error", &format!("Chyba při načítání tasků: {}", e));
                return;
            }
        }

        // Main loop
        let mut ticker = interval(Duration::from_secs(cfg.interval_seconds));
        
        Self::emit_log(&app, "info", &format!("Tracking spuštěn (interval: {}s)", cfg.interval_seconds));

        loop {
            ticker.tick().await;

            // Check if still running
            if !*is_running.lock().await {
                Self::emit_log(&app, "info", "Tracking loop ukončen");
                break;
            }

            // Skrýt okno před screenshotem
            Self::emit_log(&app, "info", "📸 Skrývám okno pro screenshot...");
            if let Some(window) = app.get_webview_window("main") {
                if let Err(e) = window.hide() {
                    Self::emit_log(&app, "error", &format!("Chyba při skrývání okna: {}", e));
                }
                // Počkat 300ms aby se okno stihlo skrýt
                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            }

            // Capture screenshot
            Self::emit_log(&app, "info", "📸 Zachytávám screenshot...");
            let screenshot = match capture_and_encode() {
                Ok(s) => s,
                Err(e) => {
                    Self::emit_log(&app, "error", &format!("Chyba při screenshotu: {}", e));
                    // Zobrazit okno zpět i při chybě
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                    }
                    continue;
                }
            };

            // Zobrazit okno zpět
            if let Some(window) = app.get_webview_window("main") {
                if let Err(e) = window.show() {
                    Self::emit_log(&app, "error", &format!("Chyba při zobrazení okna: {}", e));
                }
            }

            // Get tasks
            let tasks = freelo_tasks_cache.lock().await.clone();

            // OCR - extrakce textu ze screenshotu (v samostatném vlákně)
            // DEBUG MODE: save_debug = true pro ukládání mezikroků
            Self::emit_log(&app, "info", "📖 Spouštím OCR (debug mode)...");
            let screenshot_clone = screenshot.clone();
            let ocr_result = tokio::task::spawn_blocking(move || {
                extract_text_from_screenshot(&screenshot_clone, true) // true = debug mode
            })
            .await;

            let ocr_text = match ocr_result {
                Ok(Ok(text)) => text,
                Ok(Err(e)) => {
                    Self::emit_log(&app, "error", &format!("OCR chyba: {}", e));
                    continue;
                }
                Err(e) => {
                    Self::emit_log(&app, "error", &format!("OCR task chyba: {}", e));
                    continue;
                }
            };

            Self::emit_log(&app, "info", &format!("✅ OCR: Extrahováno {} znaků", ocr_text.len()));

            // Zkus AI matching pokud máme OpenRouter API key
            let match_result = if let Some(ref openrouter_key) = cfg.openrouter_api_key {
                Self::emit_log(&app, "info", "🤖 Zkouším AI matching...");

                match match_task_with_ai(&ocr_text, &tasks, openrouter_key).await {
                    Ok(ai_result) => {
                        Self::emit_log(
                            &app,
                            "info",
                            &format!("✅ AI Match: confidence={}%, activity={}", ai_result.confidence, ai_result.activity_description)
                        );

                        // Převeď AI výsledek na MatchResult
                        let task_name = ai_result.task_id.and_then(|id| {
                            tasks.iter().find(|t| t.id == id).map(|t| t.name.clone())
                        });

                        MatchResult {
                            task_id: ai_result.task_id,
                            task_name,
                            confidence: ai_result.confidence / 100.0, // AI vrací 0-100, MatchResult očekává 0-1
                            detected_application: "AI Detection".to_string(),
                            matched_keywords: vec![],
                            activity_description: ai_result.activity_description,
                        }
                    }
                    Err(e) => {
                        Self::emit_log(&app, "warning", &format!("⚠️  AI matching selhal: {}. Používám fallback.", e));
                        Self::emit_log(&app, "info", "🔍 Fallback: Textové porovnání...");
                        find_best_matching_task(&ocr_text, &tasks)
                    }
                }
            } else {
                // Bez OpenRouter API key - použij klasický text matching
                Self::emit_log(&app, "info", "🔍 Hledám matching task (textové porovnání)...");
                find_best_matching_task(&ocr_text, &tasks)
            };

            // Log match result
            Self::emit_log(
                &app,
                "info",
                &format!(
                    "📊 Aplikace: {} | Task: {} | Confidence: {:.0}%",
                    match_result.detected_application,
                    match_result.task_name.as_deref().unwrap_or("Žádný"),
                    match_result.confidence * 100.0
                ),
            );

            if !match_result.matched_keywords.is_empty() {
                Self::emit_log(
                    &app,
                    "info",
                    &format!("🔑 Matched keywords: {}", match_result.matched_keywords.join(", ")),
                );
            }

            // Update tracking info in UI
            Self::emit_tracking_update(
                &app,
                &match_result.detected_application,
                &format!("OCR: {} znaků", ocr_text.len()),
                match_result.task_name.as_deref(),
            );

            // Handle tracking logic
            Self::handle_tracking_logic(
                &app,
                &freelo,
                &active_tracking,
                &match_result,
            )
            .await;
        }
    }

    async fn handle_tracking_logic(
        app: &AppHandle,
        freelo: &FreeloClient,
        active_tracking: &Arc<Mutex<Option<ActiveTracking>>>,
        match_result: &MatchResult,
    ) {
        let new_task_id = if match_result.confidence > 0.3 {
            match_result.task_id.map(|id| id.to_string())
        } else {
            None
        };

        let tracking_key = new_task_id
            .clone()
            .unwrap_or_else(|| "general_work".to_string());

        let current_application = match_result.detected_application.clone();
        let current_activity = match_result.activity_description.clone();

        let mut tracking_guard = active_tracking.lock().await;

        // Determine if application or activity changed and if we should restart
        let (application_changed, activity_changed, should_restart) = if let Some(ref tracking) = *tracking_guard {
            let app_changed = tracking.last_application != current_application;
            let activity_changed = tracking.last_activity_description != current_activity;

            if app_changed || activity_changed {
                let new_unstable_count = tracking.unstable_count + 1;

                if app_changed && activity_changed {
                    Self::emit_log(
                        app,
                        "info",
                        &format!(
                            "🔍 Aplikace i aktivita se změnily: {} → {} | {} → {} (nestabilní tick: {}/2)",
                            tracking.last_application, current_application,
                            tracking.last_activity_description, current_activity,
                            new_unstable_count
                        ),
                    );
                } else if app_changed {
                    Self::emit_log(
                        app,
                        "info",
                        &format!(
                            "🔍 Aplikace se změnila: {} → {} (nestabilní tick: {}/2)",
                            tracking.last_application, current_application, new_unstable_count
                        ),
                    );
                } else {
                    Self::emit_log(
                        app,
                        "info",
                        &format!(
                            "🔍 Aktivita se změnila: {} → {} (nestabilní tick: {}/2)",
                            tracking.last_activity_description, current_activity, new_unstable_count
                        ),
                    );
                }

                (app_changed, activity_changed, new_unstable_count >= 2)
            } else {
                Self::emit_log(
                    app,
                    "info",
                    &format!("✅ Aplikace i aktivita stejné: {} (reset počítadla)", current_application),
                );
                (false, false, false)
            }
        } else {
            (false, false, false)
        };

        // Check current state
        let should_continue_same_task = if let Some(ref tracking) = *tracking_guard {
            tracking.task_id == tracking_key && !should_restart
        } else {
            false
        };

        if should_continue_same_task {
            // A) Tracking active, same task, no restart
            if let Some(ref mut tracking) = *tracking_guard {
                if !application_changed && !activity_changed {
                    tracking.unstable_count = 0;
                } else {
                    tracking.unstable_count += 1;
                    tracking.last_application = current_application.clone();
                    tracking.last_activity_description = current_activity.clone();
                    Self::emit_log(
                        app,
                        "warning",
                        &format!("⚠️  Kontext se mění, ale čekáme na stabilizaci ({}/2)", tracking.unstable_count),
                    );
                }

                if new_task_id.is_some() {
                    Self::emit_log(app, "success", &format!("✅ TRACKING: Task {} pokračuje", tracking_key));
                } else {
                    Self::emit_log(app, "success", "✅ TRACKING: Obecná práce pokračuje");
                }
            }
        } else if should_restart && tracking_guard.is_some() {

            // A2) Tracking active, context changed significantly (RESTART with hysteresis)
            let tracking = tracking_guard.take().unwrap();
            Self::emit_log(app, "info", "🔄 TRACKING: Kontext se změnil, restartuji tracking");
            if application_changed {
                Self::emit_log(app, "info", &format!("   Stará aplikace: {}", tracking.last_application));
                Self::emit_log(app, "info", &format!("   Nová aplikace: {}", current_application));
            }
            if activity_changed {
                Self::emit_log(app, "info", &format!("   Stará aktivita: {}", tracking.last_activity_description));
                Self::emit_log(app, "info", &format!("   Nová aktivita: {}", current_activity));
            }

            // Stop old tracking
            if let Err(e) = freelo.stop_tracking(&tracking.uuid).await {
                Self::emit_log(app, "error", &format!("CHYBA STOP TRACKING: {}", e));
            }

            // Start new tracking
            let note = &match_result.activity_description;
            let task_id_ref = new_task_id.as_ref().map(|s| s.as_str());

            match freelo.start_tracking(task_id_ref, note).await {
                Ok(uuid) => {
                    *tracking_guard = Some(ActiveTracking {
                        task_id: tracking_key.clone(),
                        uuid: uuid.clone(),
                        start_time: SystemTime::now(),
                        last_context: current_application.clone(),
                        last_application: current_application.clone(),
                        last_activity_description: current_activity.clone(),
                        unstable_count: 0,
                    });
                    Self::emit_log(app, "success", &format!("▶️  TRACKING: Start s novým kontextem (UUID: {})", uuid));
                }
                Err(e) => {
                    Self::emit_log(app, "error", &format!("CHYBA START TRACKING: {}", e));
                }
            }
        } else if tracking_guard.is_none() {
            // C) No tracking active - START
            let note = &match_result.activity_description;
            let task_id_ref = new_task_id.as_ref().map(|s| s.as_str());

            match freelo.start_tracking(task_id_ref, note).await {
                Ok(uuid) => {
                    *tracking_guard = Some(ActiveTracking {
                        task_id: tracking_key.clone(),
                        uuid: uuid.clone(),
                        start_time: SystemTime::now(),
                        last_context: current_application.clone(),
                        last_application: current_application.clone(),
                        last_activity_description: current_activity.clone(),
                        unstable_count: 0,
                    });

                    if new_task_id.is_some() {
                        Self::emit_log(app, "success", &format!("▶️  TRACKING: Start s taskem {} (UUID: {})", tracking_key, uuid));
                    } else {
                        Self::emit_log(app, "success", &format!("▶️  TRACKING: Start obecné práce (UUID: {})", uuid));
                    }
                }
                Err(e) => {
                    Self::emit_log(app, "error", &format!("CHYBA START TRACKING: {}", e));
                }
            }
        }
    }

    fn emit_log(app: &AppHandle, level: &str, message: &str) {
        tracing::info!("{}: {}", level.to_uppercase(), message);
        let _ = app.emit("log-event", serde_json::json!({
            "level": level,
            "message": message,
        }));
    }

    fn emit_tracking_update(app: &AppHandle, application: &str, activity: &str, task: Option<&str>) {
        let _ = app.emit("tracking-update", serde_json::json!({
            "application": application,
            "activity": activity,
            "task": task.unwrap_or("Žádný"),
            "since": chrono::Local::now().format("%H:%M:%S").to_string(),
        }));
    }
}

