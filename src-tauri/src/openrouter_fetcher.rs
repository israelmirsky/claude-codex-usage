//! Fetches remaining OpenRouter credits from the OpenRouter credits API.
//!
//! Reads the key from macOS Keychain (settings flow), with `OPENROUTER_API_KEY`
//! as a fallback for terminal/dev workflows.

use std::sync::Mutex;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::openrouter_keychain;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterCreditsData {
    pub total_credits: f64,
    pub total_usage: f64,
    pub remaining_credits: f64,
    pub fetched_at: String,
}

pub struct OpenRouterState {
    pub last_data: Mutex<Option<OpenRouterCreditsData>>,
}

impl OpenRouterState {
    pub fn new() -> Self {
        Self {
            last_data: Mutex::new(None),
        }
    }
}

#[derive(Debug, Deserialize)]
struct OpenRouterCreditsResponse {
    data: Option<OpenRouterCreditsPayload>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterCreditsPayload {
    total_credits: Option<Value>,
    total_usage: Option<Value>,
}

fn value_to_f64(v: Option<Value>) -> f64 {
    match v {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn read_openrouter_key() -> Result<String, String> {
    if let Some(key) = openrouter_keychain::read_openrouter_api_key()? {
        return Ok(key);
    }

    let key = std::env::var("OPENROUTER_API_KEY")
        .map_err(|_| "OPENROUTER_API_KEY is not set".to_string())?;
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err("OPENROUTER_API_KEY is empty".into());
    }
    Ok(trimmed.to_string())
}

pub async fn fetch_openrouter_credits(client: &Client) -> Result<OpenRouterCreditsData, String> {
    let key = read_openrouter_key()?;

    let resp = client
        .get("https://openrouter.ai/api/v1/credits")
        .header("Authorization", format!("Bearer {}", key))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("OpenRouter request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!(
            "OpenRouter API returned {}: {}",
            status,
            &body[..body.len().min(200)]
        ));
    }

    let payload: OpenRouterCreditsResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse OpenRouter response: {}", e))?;

    let data = payload.data.unwrap_or(OpenRouterCreditsPayload {
        total_credits: None,
        total_usage: None,
    });

    let total_credits = value_to_f64(data.total_credits);
    let total_usage = value_to_f64(data.total_usage);
    let remaining_credits = (total_credits - total_usage).max(0.0);

    Ok(OpenRouterCreditsData {
        total_credits,
        total_usage,
        remaining_credits,
        fetched_at: chrono::Utc::now().to_rfc3339(),
    })
}
