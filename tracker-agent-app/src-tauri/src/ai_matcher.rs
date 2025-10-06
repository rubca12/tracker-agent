use crate::freelo::FreeloTask;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Serialize)]
struct OpenRouterRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
pub struct AIMatchResult {
    pub task_id: Option<i32>,
    pub confidence: f32,
    pub reasoning: String,
    pub activity_description: String, // Krátký popis co uživatel dělá
}

/// Použije AI (OpenRouter) pro matching OCR textu s Freelo tasky
pub async fn match_task_with_ai(
    ocr_text: &str,
    tasks: &[FreeloTask],
    api_key: &str,
) -> Result<AIMatchResult, String> {
    info!("🤖 AI Matching: Posílám OCR text do OpenRouter...");
    
    // Připrav seznam tasků pro AI
    let tasks_list: Vec<String> = tasks
        .iter()
        .map(|t| {
            format!(
                "ID: {}, Název: {}, Projekt: {}",
                t.id,
                t.name,
                t.project_name
            )
        })
        .collect();
    
    let tasks_text = tasks_list.join("\n");
    
    // Vytvoř prompt pro AI
    let prompt = format!(
        r#"Analyzuj následující OCR text z obrazovky uživatele a vyber nejlepší matching Freelo task.

OCR TEXT (co uživatel vidí na obrazovce):
```
{}
```

DOSTUPNÉ FREELO TASKY:
```
{}
```

INSTRUKCE:
1. Analyzuj OCR text a zjisti co uživatel právě dělá
2. Vyber task který nejlépe odpovídá této aktivitě
3. Pokud žádný task neodpovídá dobře, vrať task_id: null
4. Confidence je 0-100 (jak moc si jsi jistý)
5. VŽDY napiš krátký popis aktivity (max 100 znaků) do activity_description

Odpověz POUZE v tomto JSON formátu (bez markdown bloků):
{{
  "task_id": 123,
  "confidence": 85,
  "reasoning": "Uživatel pracuje na...",
  "activity_description": "Editace kódu v tracker-agent-app"
}}

Nebo pokud žádný task neodpovídá:
{{
  "task_id": null,
  "confidence": 0,
  "reasoning": "Žádný task neodpovídá aktivitě...",
  "activity_description": "Prohlížení dokumentace na webu"
}}"#,
        ocr_text.chars().take(3000).collect::<String>(), // Limit na 3000 znaků
        tasks_text
    );
    
    // Vytvoř request pro OpenRouter
    let request = OpenRouterRequest {
        model: "google/gemini-2.5-flash".to_string(), // Gemini 2.0 Flash (free tier)
        messages: vec![Message {
            role: "user".to_string(),
            content: prompt,
        }],
        temperature: 0.3,
        max_tokens: 500,
    };
    
    // Pošli request
    let client = reqwest::Client::new();
    let response = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("OpenRouter request failed: {}", e))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("OpenRouter API error {}: {}", status, error_text));
    }
    
    let openrouter_response: OpenRouterResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse OpenRouter response: {}", e))?;
    
    // Extrahuj AI odpověď
    let ai_response = openrouter_response
        .choices
        .first()
        .ok_or("No choices in OpenRouter response")?
        .message
        .content
        .clone();

    info!("🤖 AI odpověď: {}", ai_response);

    // Odstraň markdown code bloky pokud jsou přítomné
    let json_str = ai_response
        .trim()
        .strip_prefix("```json")
        .unwrap_or(&ai_response)
        .strip_suffix("```")
        .unwrap_or(&ai_response)
        .trim();

    // Parse JSON odpověď
    let result: AIMatchResult = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse AI JSON response: {}. Response was: {}", e, json_str))?;
    
    info!(
        "✅ AI Match: task_id={:?}, confidence={}%, reasoning={}",
        result.task_id, result.confidence, result.reasoning
    );
    
    Ok(result)
}

