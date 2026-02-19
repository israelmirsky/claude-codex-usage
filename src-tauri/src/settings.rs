//! Persists user preferences (refresh interval, notification threshold, autostart)
//! to a JSON file in the app's data directory.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Refresh interval in seconds (60, 120, 300, 600, 900)
    pub refresh_interval_secs: u64,
    /// Notification threshold percentage (70, 80, 90, 95) or 0 to disable
    pub notify_threshold: u32,
    /// Whether notifications are enabled
    pub notifications_enabled: bool,
    /// Whether app starts at login
    pub start_at_login: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            refresh_interval_secs: 300,
            notify_threshold: 80,
            notifications_enabled: true,
            start_at_login: false,
        }
    }
}

pub struct SettingsState {
    pub settings: Mutex<Settings>,
    data_dir: PathBuf,
}

impl SettingsState {
    pub fn new(data_dir: PathBuf) -> Self {
        let settings = Self::load_from(&data_dir).unwrap_or_default();
        Self {
            settings: Mutex::new(settings),
            data_dir,
        }
    }

    fn settings_path(data_dir: &PathBuf) -> PathBuf {
        data_dir.join(SETTINGS_FILE)
    }

    fn load_from(data_dir: &PathBuf) -> Option<Settings> {
        let path = Self::settings_path(data_dir);
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn save(&self) -> Result<(), String> {
        std::fs::create_dir_all(&self.data_dir).map_err(|e| e.to_string())?;
        let path = Self::settings_path(&self.data_dir);
        let settings = self.settings.lock().unwrap();
        let json = serde_json::to_string_pretty(&*settings).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())
    }

    pub fn update<F: FnOnce(&mut Settings)>(&self, f: F) -> Result<Settings, String> {
        {
            let mut settings = self.settings.lock().unwrap();
            f(&mut settings);
        }
        self.save()?;
        Ok(self.settings.lock().unwrap().clone())
    }

    pub fn get(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }
}
