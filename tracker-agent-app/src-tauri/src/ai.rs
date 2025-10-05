use serde::{Deserialize, Serialize};
use reqwest::Client;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIAnalysisResult {
    pub task_id: Option<String>,
    pub confidence: f32,
    pub summary: String,
    pub detected_context: String,
    pub best_match_task_name: Option<String>,
    pub best_match_confidence: Option<f32>,
}

#[derive(Debug, Serialize)]
struct OpenRouterRequest {
    model: String,
    messages: Vec<OpenRouterMessage>,
}

#[derive(Debug, Serialize)]
struct OpenRouterMessage {
    role: String,
    content: Vec<OpenRouterContent>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum OpenRouterContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Serialize)]
struct ImageUrl {
    url: String,
}

#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    choices: Vec<OpenRouterChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterChoice {
    message: OpenRouterResponseMessage,
}

#[derive(Debug, Deserialize)]
struct OpenRouterResponseMessage {
    content: String,
}

pub async fn analyze_screenshot(
    openrouter_key: &str,
    screen_base64: &str,
    freelo_tasks: Vec<(String, String, String)>, // (id, project, name)
    previous_context: Option<String>,
) -> Result<AIAnalysisResult, String> {
    let client = Client::new();

    // Prepare tasks for prompt
    let simplified_tasks: Vec<String> = freelo_tasks
        .iter()
        .map(|(id, project, name)| format!("ID: {}, Projekt: {}, Název: {}", id, project, name))
        .collect();

    let tasks_json = serde_json::to_string(&simplified_tasks)
        .map_err(|e| format!("Chyba serializace tasků: {}", e))?;

    // Previous context hint
    let previous_context_hint = if let Some(ref prev) = previous_context {
        format!(
            "\n\n⚠️ CONSISTENCY HINT:\nPrevious application was: \"{}\"\nIf the screen looks similar, use the SAME application name for consistency!",
            prev
        )
    } else {
        String::new()
    };

    let prompt = format!(
        r#"You are analyzing a WORK COMPUTER SCREENSHOT for time tracking purposes.

Your task: Analyze this screenshot of a developer's work computer and determine what task they are working on.

⚠️ CRITICAL RULES - NO HALLUCINATIONS:
1. Write ONLY what you SEE on screen - NOTHING ELSE!
2. If you don't see "GitHub" text → DON'T write GitHub
3. If you don't see "Laravel" text → DON'T write Laravel
4. If you don't see a URL → DON'T write a URL
5. Detected context = ONLY visible text (window titles, URLs in address bar)

PROCESS:
1. Read VISIBLE text: window titles, URLs in address bar, file names, code editor content
2. Describe what you SEE (not what task it might be!)
3. Compare with Freelo tasks below
4. Return JSON:

{{
  "task_id": "123" or null,
  "confidence": 0.85,
  "summary": "DESCRIBE WHAT YOU SEE - editing Rust code, browsing website, reading email (max 60 chars)",
  "detected_context": "Main application/URL (max 50 chars)",
  "best_match_task_name": "Candidate name" or null,
  "best_match_confidence": 0.45 or null
}}

⚠️ IMPORTANT - SUMMARY vs BEST_MATCH:
- "summary": Describe the ACTIVITY you see ("Editing Rust file main.rs", "Browsing Spotify", "Reading email")
- "best_match_task_name": The Freelo task name that matches this activity
- DON'T copy task name into summary! They are different fields!

EXAMPLES:
✅ GOOD:
{{
  "summary": "Editing Rust code in VS Code",
  "detected_context": "Visual Studio Code",
  "best_match_task_name": "rust_pwa_server"
}}

❌ BAD:
{{
  "summary": "rust_pwa_server",  ← WRONG! This is task name, not activity description!
  "detected_context": "Visual Studio Code",
  "best_match_task_name": "rust_pwa_server"
}}

CONFIDENCE:
- > 0.8: Clear match → return task_id
- 0.3-0.8: Uncertain → task_id: null, fill best_match
- < 0.3: None → all optional: null

CONTEXT - MUST BE STABLE:
✅ GOOD: "app.freelo.io"
✅ GOOD: "Visual Studio Code"
❌ BAD: "app.freelo.io, Freelo API, Rust, debugging..."

Write ONLY the main application or URL, not every detail!
{}

Freelo Tasks:
{}"#,
        previous_context_hint, tasks_json
    );

    // Prepare request
    let image_url = format!("data:image/jpeg;base64,{}", screen_base64);

    let request = OpenRouterRequest {
        model: "openai/gpt-4o-2024-08-06".to_string(),
        messages: vec![OpenRouterMessage {
            role: "user".to_string(),
            content: vec![
                OpenRouterContent::Text {
                    text: prompt,
                },
                OpenRouterContent::ImageUrl {
                    image_url: ImageUrl { url: image_url },
                },
            ],
        }],
    };

    // Send request
    let response = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", openrouter_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("HTTP chyba: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("OpenRouter error {}: {}", status, text));
    }

    let response_data: OpenRouterResponse = response
        .json()
        .await
        .map_err(|e| format!("JSON parse error: {}", e))?;

    let content = response_data
        .choices
        .first()
        .ok_or("No choices in response")?
        .message
        .content
        .clone();

    // Clean markdown code blocks
    let cleaned = clean_json_response(&content);

    // Parse JSON
    let result: AIAnalysisResult = serde_json::from_str(&cleaned)
        .map_err(|e| format!("AI JSON parse error: {} (content: {})", e, cleaned))?;

    Ok(result)
}

fn clean_json_response(text: &str) -> String {
    let mut cleaned = text.trim().to_string();

    // Remove ```json prefix
    if cleaned.starts_with("```json") {
        cleaned = cleaned[7..].to_string();
    } else if cleaned.starts_with("```") {
        cleaned = cleaned[3..].to_string();
    }

    // Remove ``` suffix
    if cleaned.ends_with("```") {
        cleaned = cleaned[..cleaned.len() - 3].to_string();
    }

    cleaned.trim().to_string()
}

