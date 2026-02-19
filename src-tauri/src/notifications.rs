//! Tracks per-metric threshold crossings and fires macOS notifications
//! only when a metric transitions from below to above the threshold.

use std::collections::HashMap;
use std::sync::Mutex;

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

use crate::usage_fetcher::UsageData;

/// Tracks which metrics have already triggered a notification so we
/// don't spam the user on every refresh while they're above threshold.
pub struct NotificationState {
    /// Maps metric key -> whether we've already notified for this crossing
    notified: Mutex<HashMap<String, bool>>,
}

impl NotificationState {
    pub fn new() -> Self {
        Self {
            notified: Mutex::new(HashMap::new()),
        }
    }
}

struct Metric {
    key: String,
    label: String,
    percent: f64,
    reset_info: String,
}

/// Check usage data against threshold and fire notifications for any
/// metrics that just crossed above it. Call this after every successful fetch.
pub fn check_and_notify(
    app: &AppHandle,
    provider: &str,
    data: &UsageData,
    threshold: u32,
    enabled: bool,
    state: &NotificationState,
) {
    if !enabled || threshold == 0 {
        return;
    }

    let threshold_f = threshold as f64;

    let metrics = [
        Metric {
            key: format!("{}_session", provider),
            label: format!("{} session", provider),
            percent: data.session.percent_used,
            reset_info: data.session.reset_info.clone(),
        },
        Metric {
            key: format!("{}_weekly", provider),
            label: format!("{} weekly", provider),
            percent: data.weekly_all.percent_used,
            reset_info: data.weekly_all.reset_info.clone(),
        },
        Metric {
            key: format!("{}_sonnet", provider),
            label: data.weekly_sonnet.label.clone(),
            percent: data.weekly_sonnet.percent_used,
            reset_info: data.weekly_sonnet.reset_info.clone(),
        },
        Metric {
            key: format!("{}_extra", provider),
            label: format!("{} extra usage", provider),
            percent: data.extra.percent_used,
            reset_info: data.extra.reset_date.clone(),
        },
    ];

    let mut notified = state.notified.lock().unwrap();

    for m in &metrics {
        let was_notified = notified.get(&m.key).copied().unwrap_or(false);

        if m.percent >= threshold_f && !was_notified {
            // Crossed above threshold - fire notification
            let title = format!("{} at {:.0}%", m.label, m.percent);
            let body = m.reset_info.clone();
            let _ = app
                .notification()
                .builder()
                .title(&title)
                .body(&body)
                .show();
            notified.insert(m.key.clone(), true);
        } else if m.percent < threshold_f && was_notified {
            // Dropped back below threshold - reset
            notified.insert(m.key.clone(), false);
        }
    }
}
