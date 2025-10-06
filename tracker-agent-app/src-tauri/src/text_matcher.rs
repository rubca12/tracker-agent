use crate::freelo::FreeloTask;
use tracing::info;

/// Výsledek textového matchingu
#[derive(Debug, Clone)]
pub struct MatchResult {
    pub task_id: Option<i32>,
    pub task_name: Option<String>,
    pub confidence: f32,
    pub detected_application: String,
    pub matched_keywords: Vec<String>,
    pub activity_description: String, // Popis co uživatel dělá
}

/// Normalizace textu pro porovnávání
fn normalize_text(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

/// Výpočet podobnosti mezi dvěma texty (Jaccard similarity)
fn calculate_similarity(text1: &str, text2: &str) -> f32 {
    let words1: std::collections::HashSet<&str> = text1.split_whitespace().collect();
    let words2: std::collections::HashSet<&str> = text2.split_whitespace().collect();
    
    if words1.is_empty() && words2.is_empty() {
        return 1.0;
    }
    
    let intersection = words1.intersection(&words2).count();
    let union = words1.union(&words2).count();
    
    if union == 0 {
        return 0.0;
    }
    
    intersection as f32 / union as f32
}

/// Detekce aplikace z OCR textu
fn detect_application(ocr_text: &str) -> String {
    let normalized = normalize_text(ocr_text);

    info!("🔍 Detekce aplikace z OCR textu...");
    info!("   Normalizovaný text (prvních 200 znaků): {}",
        if normalized.len() > 200 { &normalized[..200] } else { &normalized });

    // Detekce známých aplikací podle klíčových slov
    if normalized.contains("visual studio code") || normalized.contains("vscode") {
        info!("   ✓ Detekována: Visual Studio Code");
        return "Visual Studio Code".to_string();
    }
    if normalized.contains("chrome") || normalized.contains("google chrome") {
        info!("   ✓ Detekována: Google Chrome");
        return "Google Chrome".to_string();
    }
    if normalized.contains("firefox") {
        info!("   ✓ Detekována: Firefox");
        return "Firefox".to_string();
    }
    if normalized.contains("safari") {
        info!("   ✓ Detekována: Safari");
        return "Safari".to_string();
    }
    if normalized.contains("freelo") {
        info!("   ✓ Detekována: Freelo");
        return "Freelo".to_string();
    }
    if normalized.contains("slack") {
        info!("   ✓ Detekována: Slack");
        return "Slack".to_string();
    }
    if normalized.contains("terminal") || normalized.contains("iterm") {
        info!("   ✓ Detekována: Terminal");
        return "Terminal".to_string();
    }

    // Pokud nenajdeme specifickou aplikaci, vrátíme obecný název
    info!("   ⚠️  Aplikace nerozpoznána");
    "Unknown Application".to_string()
}

/// Najde nejlepší matching task z OCR textu
pub fn find_best_matching_task(ocr_text: &str, tasks: &[FreeloTask]) -> MatchResult {
    let normalized_ocr = normalize_text(ocr_text);
    
    info!("🔍 Hledám matching task v OCR textu ({} znaků)...", ocr_text.len());
    
    // Detekce aplikace
    let detected_app = detect_application(ocr_text);
    
    if tasks.is_empty() {
        info!("⚠️  Žádné tasky k dispozici");
        return MatchResult {
            task_id: None,
            task_name: None,
            confidence: 0.0,
            detected_application: detected_app.clone(),
            matched_keywords: vec![],
            activity_description: format!("{} - práce mimo Freelo", detected_app),
        };
    }
    
    // Najdi nejlepší match
    info!("📋 Porovnávám s {} tasky...", tasks.len());
    let mut best_match: Option<(&FreeloTask, f32, Vec<String>)> = None;

    for task in tasks {
        // Porovnej s názvem tasku
        let task_name_normalized = normalize_text(&task.name);
        let name_similarity = calculate_similarity(&normalized_ocr, &task_name_normalized);

        // Porovnej s názvem projektu
        let project_name_normalized = normalize_text(&task.project_name);
        let project_similarity = calculate_similarity(&normalized_ocr, &project_name_normalized);

        // Najdi konkrétní klíčová slova z tasku v OCR textu
        let task_words: Vec<&str> = task_name_normalized.split_whitespace().collect();
        let matched_keywords: Vec<String> = task_words
            .iter()
            .filter(|word| word.len() > 3 && normalized_ocr.contains(*word))
            .map(|s| s.to_string())
            .collect();

        // Celková confidence = váhovaný průměr
        let keyword_bonus = if !matched_keywords.is_empty() {
            0.3 * (matched_keywords.len() as f32 / task_words.len() as f32)
        } else {
            0.0
        };

        let confidence = (name_similarity * 0.5) + (project_similarity * 0.2) + keyword_bonus;

        // Debug log pro každý task s confidence > 0.1
        if confidence > 0.1 {
            info!(
                "   Task '{}': name_sim={:.2}, proj_sim={:.2}, keywords={}, confidence={:.0}%",
                task.name, name_similarity, project_similarity, matched_keywords.len(), confidence * 100.0
            );
        }

        if let Some((_, best_confidence, _)) = best_match {
            if confidence > best_confidence {
                best_match = Some((task, confidence, matched_keywords));
            }
        } else {
            best_match = Some((task, confidence, matched_keywords));
        }
    }
    
    // Vytvoř základní popis aktivity z detekované aplikace a OCR textu
    let activity_desc = format!("{} - {}",
        detected_app,
        ocr_text.chars().take(50).collect::<String>().trim()
    );

    if let Some((task, confidence, keywords)) = best_match {
        // Threshold pro přiřazení tasku
        if confidence > 0.3 {
            info!(
                "✅ Nalezen matching task: '{}' (confidence: {:.0}%)",
                task.name,
                confidence * 100.0
            );
            return MatchResult {
                task_id: Some(task.id),
                task_name: Some(task.name.clone()),
                confidence,
                detected_application: detected_app,
                matched_keywords: keywords,
                activity_description: activity_desc,
            };
        } else {
            info!(
                "⚠️  Nejlepší match '{}' má nízkou confidence ({:.0}%), nepoužívám",
                task.name,
                confidence * 100.0
            );
        }
    }

    // Žádný dostatečně dobrý match
    MatchResult {
        task_id: None,
        task_name: None,
        confidence: 0.0,
        detected_application: detected_app,
        matched_keywords: vec![],
        activity_description: activity_desc,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_normalize_text() {
        assert_eq!(normalize_text("Hello World!"), "hello world");
        assert_eq!(normalize_text("Test  123"), "test 123");
    }
    
    #[test]
    fn test_calculate_similarity() {
        assert_eq!(calculate_similarity("hello world", "hello world"), 1.0);
        assert_eq!(calculate_similarity("hello", "world"), 0.0);
        assert!(calculate_similarity("hello world", "hello") > 0.0);
    }
    
    #[test]
    fn test_detect_application() {
        assert_eq!(detect_application("Visual Studio Code - file.rs"), "Visual Studio Code");
        assert_eq!(detect_application("Google Chrome - Tab"), "Google Chrome");
    }
}

