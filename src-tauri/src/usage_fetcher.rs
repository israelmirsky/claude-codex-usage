//! Fetches Claude.ai usage data via the claude.ai internal API.
//!
//! Calls `GET https://claude.ai/api/organizations/{org_id}/usage` using cookies
//! from the Claude desktop app. Returns session (5-hour), weekly (7-day), and
//! model-specific utilization percentages along with reset times.

use std::sync::Mutex;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::cookie_reader::ClaudeCookies;

// --- Types shared with the frontend via Tauri IPC ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageData {
    pub session: UsageMetric,
    pub weekly_all: UsageMetric,
    pub weekly_sonnet: UsageMetric,
    pub extra: ExtraUsage,
    pub fetched_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageMetric {
    pub label: String,
    pub percent_used: f64,
    pub reset_info: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtraUsage {
    pub dollars_spent: f64,
    pub percent_used: f64,
    pub reset_date: String,
    pub enabled: bool,
}

// --- API response types ---

#[derive(Deserialize)]
struct ApiResponse {
    five_hour: Option<WindowUsage>,
    seven_day: Option<WindowUsage>,
    seven_day_sonnet: Option<WindowUsage>,
    extra_usage: Option<ApiExtraUsage>,
}

#[derive(Deserialize)]
struct WindowUsage {
    utilization: Option<f64>,
    resets_at: Option<String>,
}

#[derive(Deserialize)]
struct ApiExtraUsage {
    is_enabled: Option<bool>,
    monthly_limit: Option<f64>,
    used_credits: Option<f64>,
    utilization: Option<f64>,
}

// --- State ---

pub struct UsageState {
    pub last_data: Mutex<Option<UsageData>>,
    pub client: Client,
}

impl UsageState {
    pub fn new() -> Self {
        Self {
            last_data: Mutex::new(None),
            client: Client::new(),
        }
    }
}

pub async fn fetch_usage(cookies: &ClaudeCookies, client: &Client) -> Result<UsageData, String> {
    let url = format!(
        "https://claude.ai/api/organizations/{}/usage",
        cookies.org_id
    );

    let resp = client
        .get(&url)
        .header("Cookie", &cookies.all_cookies)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("Referer", "https://claude.ai/settings/usage")
        .header("Origin", "https://claude.ai")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
        )
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("API returned {}: {}", status, body));
    }

    let api: ApiResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    let session = match api.five_hour {
        Some(w) => UsageMetric {
            label: "Current session".into(),
            percent_used: w.utilization.unwrap_or(0.0),
            reset_info: format_reset(&w.resets_at),
        },
        None => UsageMetric {
            label: "Current session".into(),
            percent_used: 0.0,
            reset_info: "No data".into(),
        },
    };

    let weekly_all = match api.seven_day {
        Some(w) => UsageMetric {
            label: "All models".into(),
            percent_used: w.utilization.unwrap_or(0.0),
            reset_info: format_reset(&w.resets_at),
        },
        None => UsageMetric {
            label: "All models".into(),
            percent_used: 0.0,
            reset_info: "No data".into(),
        },
    };

    let weekly_sonnet = match api.seven_day_sonnet {
        Some(w) => UsageMetric {
            label: "Sonnet only".into(),
            percent_used: w.utilization.unwrap_or(0.0),
            reset_info: format_reset(&w.resets_at),
        },
        None => UsageMetric {
            label: "Sonnet only".into(),
            percent_used: 0.0,
            reset_info: "No data".into(),
        },
    };

    let extra = match api.extra_usage {
        Some(eu) => {
            let used = eu.used_credits.unwrap_or(0.0);
            let limit = eu.monthly_limit.unwrap_or(0.0);
            ExtraUsage {
                dollars_spent: used,
                percent_used: eu.utilization.unwrap_or(if limit > 0.0 {
                    (used / limit) * 100.0
                } else {
                    0.0
                }),
                reset_date: "Monthly".into(),
                enabled: eu.is_enabled.unwrap_or(false),
            }
        }
        None => ExtraUsage {
            dollars_spent: 0.0,
            percent_used: 0.0,
            reset_date: "---".into(),
            enabled: false,
        },
    };

    Ok(UsageData {
        session,
        weekly_all,
        weekly_sonnet,
        extra,
        fetched_at: chrono::Utc::now().to_rfc3339(),
    })
}

fn format_reset(resets_at: &Option<String>) -> String {
    match resets_at {
        Some(dt) => {
            if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(dt) {
                let now = chrono::Utc::now();
                let diff = parsed.signed_duration_since(now);
                let hours = diff.num_hours();
                let mins = diff.num_minutes() % 60;
                if hours > 0 {
                    format!("Resets in {}h {}m", hours, mins)
                } else if mins > 0 {
                    format!("Resets in {}m", mins)
                } else {
                    "Resets soon".into()
                }
            } else {
                dt.clone()
            }
        }
        None => "---".into(),
    }
}
