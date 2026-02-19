use std::sync::Mutex;

use reqwest::Client;
use serde::Deserialize;

use crate::usage_fetcher::{ExtraUsage, UsageData, UsageMetric};

pub struct CodexState {
    pub last_data: Mutex<Option<UsageData>>,
}

impl CodexState {
    pub fn new() -> Self {
        Self {
            last_data: Mutex::new(None),
        }
    }
}

// --- Auth file types ---

#[derive(Deserialize)]
struct CodexAuth {
    tokens: Option<CodexTokens>,
}

#[derive(Deserialize)]
struct CodexTokens {
    access_token: Option<String>,
}

// --- API response types ---

#[derive(Deserialize)]
struct WhamUsageResponse {
    #[serde(default)]
    plan_type: Option<String>,
    rate_limit: Option<RateLimitDetails>,
    #[serde(default)]
    additional_rate_limits: Option<Vec<AdditionalRateLimit>>,
    credits: Option<Credits>,
}

#[derive(Deserialize)]
struct RateLimitDetails {
    #[serde(default)]
    limit_reached: bool,
    primary_window: Option<WindowSnapshot>,
    secondary_window: Option<WindowSnapshot>,
}

#[derive(Deserialize)]
struct WindowSnapshot {
    #[serde(default)]
    used_percent: f64,
    #[serde(default)]
    limit_window_seconds: i64,
    #[serde(default)]
    reset_after_seconds: i64,
}

#[derive(Deserialize)]
struct AdditionalRateLimit {
    #[serde(default)]
    limit_name: String,
    rate_limit: Option<RateLimitDetails>,
}

#[derive(Deserialize)]
struct Credits {
    #[serde(default)]
    has_credits: bool,
    #[serde(default)]
    unlimited: bool,
    #[serde(default)]
    balance: Option<String>,
}

fn read_codex_token() -> Result<String, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    let auth_path = home.join(".codex/auth.json");

    if !auth_path.exists() {
        return Err("Codex CLI not configured (~/.codex/auth.json not found)".into());
    }

    let content =
        std::fs::read_to_string(&auth_path).map_err(|e| format!("Failed to read auth.json: {}", e))?;

    let auth: CodexAuth =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse auth.json: {}", e))?;

    auth.tokens
        .and_then(|t| t.access_token)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| "No access token found in Codex auth.json".into())
}

pub async fn fetch_codex_usage(client: &Client) -> Result<UsageData, String> {
    let token = read_codex_token()?;

    let resp = client
        .get("https://chatgpt.com/backend-api/wham/usage")
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "codex-cli")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Codex request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Codex API returned {}: {}", status, &body[..body.len().min(200)]));
    }

    let payload: WhamUsageResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Codex response: {}", e))?;

    Ok(convert_payload(payload))
}

fn format_seconds(secs: i64) -> String {
    if secs <= 0 {
        return "Resets soon".into();
    }
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    if hours > 0 {
        format!("Resets in {}h {}m", hours, mins)
    } else {
        format!("Resets in {}m", mins)
    }
}

fn window_label(secs: i64) -> String {
    let hours = secs / 3600;
    if hours >= 24 {
        let days = hours / 24;
        format!("{}-day window", days)
    } else {
        format!("{}-hour window", hours)
    }
}

fn convert_payload(payload: WhamUsageResponse) -> UsageData {
    let plan = payload.plan_type.unwrap_or_else(|| "unknown".into());

    // Primary window (5-hour session)
    let mut session = match payload.rate_limit.as_ref().and_then(|rl| rl.primary_window.as_ref()) {
        Some(w) => UsageMetric {
            label: window_label(w.limit_window_seconds),
            percent_used: w.used_percent as f64,
            reset_info: format_seconds(w.reset_after_seconds),
        },
        None => UsageMetric {
            label: "Session".into(),
            percent_used: 0.0,
            reset_info: "No data".into(),
        },
    };

    if payload.rate_limit.as_ref().map(|rl| rl.limit_reached).unwrap_or(false) {
        session.reset_info = format!("LIMIT REACHED - {}", session.reset_info);
    }

    // Secondary window (7-day weekly)
    let weekly = match payload.rate_limit.as_ref().and_then(|rl| rl.secondary_window.as_ref()) {
        Some(w) => UsageMetric {
            label: window_label(w.limit_window_seconds),
            percent_used: w.used_percent as f64,
            reset_info: format_seconds(w.reset_after_seconds),
        },
        None => UsageMetric {
            label: "Weekly".into(),
            percent_used: 0.0,
            reset_info: "No data".into(),
        },
    };

    // Additional rate limits (e.g., GPT-5.3-Codex-Spark)
    let model_limit = payload
        .additional_rate_limits
        .as_ref()
        .and_then(|limits| limits.first())
        .and_then(|l| {
            let rl = l.rate_limit.as_ref()?;
            let pw = rl.primary_window.as_ref()?;
            Some(UsageMetric {
                label: l.limit_name.clone(),
                percent_used: pw.used_percent as f64,
                reset_info: format_seconds(pw.reset_after_seconds),
            })
        })
        .unwrap_or_else(|| UsageMetric {
            label: format!("Plan: {}", plan),
            percent_used: 0.0,
            reset_info: "---".into(),
        });

    let extra = match payload.credits {
        Some(c) => {
            let balance: f64 = c.balance.as_deref().unwrap_or("0").parse().unwrap_or(0.0);
            ExtraUsage {
                dollars_spent: balance,
                percent_used: 0.0,
                reset_date: if c.unlimited { "Unlimited".into() } else { "---".into() },
                enabled: c.has_credits || c.unlimited,
            }
        }
        None => ExtraUsage {
            dollars_spent: 0.0,
            percent_used: 0.0,
            reset_date: "---".into(),
            enabled: false,
        },
    };

    UsageData {
        session,
        weekly_all: weekly,
        weekly_sonnet: model_limit,
        extra,
        fetched_at: chrono::Utc::now().to_rfc3339(),
    }
}
