use crate::ai::{analyze_screenshot, AIAnalysisResult};
use crate::freelo::{ActiveTracking, FreeloClient, FreeloTask};
use crate::screenshot::capture_and_encode;
use std::sync::Arc;
use std::time::SystemTime;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

#[derive(Clone)]
pub struct TrackerConfig {
    pub interval_seconds: u64,
    pub openrouter_key: String,
    pub freelo_email: String,
    pub freelo_api_key: String,
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

            // Capture screenshot
            Self::emit_log(&app, "info", "📸 Zachytávám screenshot...");
            let screenshot = match capture_and_encode() {
                Ok(s) => s,
                Err(e) => {
                    Self::emit_log(&app, "error", &format!("Chyba při screenshotu: {}", e));
                    continue;
                }
            };

            // Get tasks
            let tasks = freelo_tasks_cache.lock().await.clone();
            let tasks_for_ai: Vec<(String, String, String)> = tasks
                .iter()
                .map(|t| (t.id.to_string(), t.project_name.clone(), t.name.clone()))
                .collect();

            // Get previous context
            let previous_context = active_tracking
                .lock()
                .await
                .as_ref()
                .map(|t| t.last_application.clone());

            // Analyze with AI
            Self::emit_log(&app, "info", "🤖 Analyzuji s AI...");
            let analysis = match analyze_screenshot(
                &cfg.openrouter_key,
                &screenshot,
                tasks_for_ai,
                previous_context,
            )
            .await
            {
                Ok(a) => a,
                Err(e) => {
                    Self::emit_log(&app, "error", &format!("AI chyba: {}", e));
                    continue;
                }
            };

            // Log analysis
            Self::emit_log(
                &app,
                "info",
                &format!(
                    "📊 Aktivita: {} | Kontext: {} | Confidence: {:.0}%",
                    analysis.summary,
                    analysis.detected_context,
                    analysis.confidence * 100.0
                ),
            );

            // Update tracking info in UI
            Self::emit_tracking_update(
                &app,
                &analysis.detected_context,
                &analysis.summary,
                analysis.task_id.as_deref().or(analysis.best_match_task_name.as_deref()),
            );

            // Handle tracking logic
            Self::handle_tracking_logic(
                &app,
                &freelo,
                &active_tracking,
                &analysis,
            )
            .await;
        }
    }

    async fn handle_tracking_logic(
        app: &AppHandle,
        freelo: &FreeloClient,
        active_tracking: &Arc<Mutex<Option<ActiveTracking>>>,
        analysis: &AIAnalysisResult,
    ) {
        let new_task_id = if analysis.confidence > 0.8 {
            analysis.task_id.clone()
        } else {
            None
        };

        let tracking_key = new_task_id
            .clone()
            .unwrap_or_else(|| "general_work".to_string());

        let current_application = analysis.detected_context.clone();

        let mut tracking_guard = active_tracking.lock().await;

        // Determine if application changed and if we should restart
        let (application_changed, should_restart) = if let Some(ref tracking) = *tracking_guard {
            let app_changed = tracking.last_application != current_application;

            if app_changed {
                let new_unstable_count = tracking.unstable_count + 1;
                Self::emit_log(
                    app,
                    "info",
                    &format!(
                        "🔍 Aplikace se změnila: {} → {} (nestabilní tick: {}/2)",
                        tracking.last_application, current_application, new_unstable_count
                    ),
                );
                (true, new_unstable_count >= 2)
            } else {
                Self::emit_log(
                    app,
                    "info",
                    &format!("✅ Aplikace stejná: {} (reset počítadla)", current_application),
                );
                (false, false)
            }
        } else {
            (false, false)
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
                if !application_changed {
                    tracking.unstable_count = 0;
                } else {
                    tracking.unstable_count += 1;
                    tracking.last_application = current_application.clone();
                    Self::emit_log(
                        app,
                        "warning",
                        &format!("⚠️  Aplikace se mění, ale čekáme na stabilizaci ({}/2)", tracking.unstable_count),
                    );
                }

                if new_task_id.is_some() {
                    Self::emit_log(app, "success", &format!("✅ TRACKING: Task {} pokračuje", tracking_key));
                } else {
                    Self::emit_log(app, "success", "✅ TRACKING: Obecná práce pokračuje");
                }
            }
        } else if should_restart && tracking_guard.is_some() {

            // A2) Tracking active, application changed significantly (RESTART with hysteresis)
            let tracking = tracking_guard.take().unwrap();
            Self::emit_log(app, "info", "🔄 TRACKING: Aplikace se změnila, restartuji tracking");
            Self::emit_log(app, "info", &format!("   Stará aplikace: {}", tracking.last_application));
            Self::emit_log(app, "info", &format!("   Nová aplikace: {}", current_application));

            // Stop old tracking
            if let Err(e) = freelo.stop_tracking(&tracking.uuid).await {
                Self::emit_log(app, "error", &format!("CHYBA STOP TRACKING: {}", e));
            }

            // Start new tracking
            let note = format!("AI: {}", analysis.summary);
            let task_id_ref = new_task_id.as_ref().map(|s| s.as_str());

            match freelo.start_tracking(task_id_ref, &note).await {
                Ok(uuid) => {
                    *tracking_guard = Some(ActiveTracking {
                        task_id: tracking_key.clone(),
                        uuid: uuid.clone(),
                        start_time: SystemTime::now(),
                        last_context: analysis.detected_context.clone(),
                        last_application: current_application.clone(),
                        unstable_count: 0,
                    });
                    Self::emit_log(app, "success", &format!("▶️  TRACKING: Start s novou aplikací (UUID: {})", uuid));
                }
                Err(e) => {
                    Self::emit_log(app, "error", &format!("CHYBA START TRACKING: {}", e));
                }
            }
        } else if tracking_guard.is_none() {
            // C) No tracking active - START
            let note = format!("AI: {}", analysis.summary);
            let task_id_ref = new_task_id.as_ref().map(|s| s.as_str());

            match freelo.start_tracking(task_id_ref, &note).await {
                Ok(uuid) => {
                    *tracking_guard = Some(ActiveTracking {
                        task_id: tracking_key.clone(),
                        uuid: uuid.clone(),
                        start_time: SystemTime::now(),
                        last_context: analysis.detected_context.clone(),
                        last_application: current_application.clone(),
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

