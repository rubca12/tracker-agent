use reqwest::Client;
use serde::{Deserialize, Serialize};

// Raw structure from Freelo API
#[derive(Debug, Clone, Deserialize)]
struct TaskDetailResponse {
    data: TaskDetailData,
}

#[derive(Debug, Clone, Deserialize)]
struct TaskDetailData {
    tasks: Vec<FreeloTaskRaw>,
}

#[derive(Debug, Clone, Deserialize)]
struct FreeloTaskRaw {
    id: i32,
    name: String,
    project: ProjectInfo,
}

#[derive(Debug, Clone, Deserialize)]
struct ProjectInfo {
    id: i32,
    name: String,
}

// Simplified structure for our use
#[derive(Debug, Clone, Serialize)]
pub struct FreeloTask {
    pub id: i32,
    pub name: String,
    pub project_id: i32,
    pub project_name: String,
}

#[derive(Debug, Clone)]
pub struct ActiveTracking {
    pub task_id: String,
    pub uuid: String,
    pub start_time: std::time::SystemTime,
    pub last_context: String,
    pub last_application: String,
    pub unstable_count: u32,
}

pub struct FreeloClient {
    client: Client,
    email: String,
    api_key: String,
}

impl FreeloClient {
    pub fn new(email: String, api_key: String) -> Self {
        Self {
            client: Client::new(),
            email,
            api_key,
        }
    }

    pub async fn get_active_tasks(&self) -> Result<Vec<FreeloTask>, String> {
        let url = "https://api.freelo.io/v1/all-tasks?states_ids[]=1&limit=100";

        let response = self
            .client
            .get(url)
            .basic_auth(&self.email, Some(&self.api_key))
            .header("User-Agent", "TrackerAgent/1.0 (tracker@agent.io)")
            .send()
            .await
            .map_err(|e| format!("HTTP chyba: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!("Freelo API error {}: {}", status, text));
        }

        let task_response: TaskDetailResponse = response
            .json()
            .await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        // Convert to simplified structure
        let tasks = task_response
            .data
            .tasks
            .into_iter()
            .map(|t| FreeloTask {
                id: t.id,
                name: t.name,
                project_id: t.project.id,
                project_name: t.project.name,
            })
            .collect();

        Ok(tasks)
    }

    pub async fn start_tracking(
        &self,
        task_id: Option<&str>,
        note: &str,
    ) -> Result<String, String> {
        let url = "https://api.freelo.io/v1/timetracking/start";

        let mut body = serde_json::json!({
            "note": note,
        });

        if let Some(id) = task_id {
            body["task_id"] = serde_json::json!(id);
        }

        let response = self
            .client
            .post(url)
            .basic_auth(&self.email, Some(&self.api_key))
            .header("User-Agent", "TrackerAgent/1.0 (tracker@agent.io)")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("HTTP chyba: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!("Freelo start tracking error {}: {}", status, text));
        }

        #[derive(Deserialize)]
        struct StartResponse {
            uuid: String,
        }

        let result: StartResponse = response
            .json()
            .await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        Ok(result.uuid)
    }

    pub async fn stop_tracking(&self, uuid: &str) -> Result<(), String> {
        let url = "https://api.freelo.io/v1/timetracking/stop";

        let body = serde_json::json!({
            "uuid": uuid,
        });

        let response = self
            .client
            .post(url)
            .basic_auth(&self.email, Some(&self.api_key))
            .header("User-Agent", "TrackerAgent/1.0 (tracker@agent.io)")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("HTTP chyba: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!("Freelo stop tracking error {}: {}", status, text));
        }

        Ok(())
    }
}

