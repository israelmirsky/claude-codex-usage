use std::sync::Mutex;

use reqwest::Client;
use serde::{Deserialize, Serialize};

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

/// Fetch usage data from Claude's API.
/// The exact endpoints will be filled in after API discovery (Task 3).
pub async fn fetch_usage(session_cookie: &str, client: &Client) -> Result<UsageData, String> {
    // TODO: Replace with real API calls after discovery
    // For now, return placeholder data so the UI can be developed
    let _ = (session_cookie, client);

    Ok(UsageData {
        session: UsageMetric {
            label: "Session".into(),
            percent_used: 0.0,
            reset_info: "Waiting for API discovery...".into(),
        },
        weekly_all: UsageMetric {
            label: "All models".into(),
            percent_used: 0.0,
            reset_info: "Waiting for API discovery...".into(),
        },
        weekly_sonnet: UsageMetric {
            label: "Sonnet only".into(),
            percent_used: 0.0,
            reset_info: "Waiting for API discovery...".into(),
        },
        extra: ExtraUsage {
            dollars_spent: 0.0,
            percent_used: 0.0,
            reset_date: "---".into(),
            enabled: false,
        },
        fetched_at: chrono::Utc::now().to_rfc3339(),
    })
}
