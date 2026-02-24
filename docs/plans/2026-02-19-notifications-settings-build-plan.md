# Notifications, Settings Menu & Build - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add native macOS notifications at configurable thresholds, a right-click tray menu with settings (refresh interval, notification threshold, start at login), and package the app as a `.dmg`.

**Architecture:** New `settings.rs` module persists user preferences to a JSON file. New `notifications.rs` module tracks per-metric threshold crossings and fires macOS notifications. The existing tray icon in `lib.rs` gets a right-click context menu built with Tauri's `Menu`/`Submenu`/`CheckMenuItem` APIs. Two new Tauri plugins: `tauri-plugin-notification` and `tauri-plugin-autostart`.

**Tech Stack:** Tauri 2, tauri-plugin-notification, tauri-plugin-autostart, Rust (serde, dirs), React+TypeScript (no frontend changes)

---

### Task 1: Add Tauri Plugin Dependencies

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `package.json`

**Step 1: Add Rust dependencies**

Add to `src-tauri/Cargo.toml` under `[dependencies]`:

```toml
tauri-plugin-notification = "2"
tauri-plugin-autostart = "2"
```

**Step 2: Add npm plugin packages**

Run:
```bash
cd /Users/israelmirsky/claude-usage-widget && npm install @tauri-apps/plugin-notification @tauri-apps/plugin-autostart
```

**Step 3: Create capabilities file for permissions**

Create `src-tauri/capabilities/default.json`:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Default permissions for the app",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "shell:allow-open",
    "notification:default",
    "autostart:allow-enable",
    "autostart:allow-disable",
    "autostart:allow-is-enabled"
  ]
}
```

**Step 4: Verify it compiles**

Run:
```bash
cd /Users/israelmirsky/claude-usage-widget && cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: Compiles without errors.

**Step 5: Commit**

```bash
cd /Users/israelmirsky/claude-usage-widget && git add src-tauri/Cargo.toml package.json package-lock.json src-tauri/capabilities/ && git commit -m "chore: add notification and autostart plugin dependencies"
```

---

### Task 2: Build Settings Module

**Files:**
- Create: `src-tauri/src/settings.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod settings;`)

**Step 1: Create `src-tauri/src/settings.rs`**

```rust
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
```

**Step 2: Add module declaration to lib.rs**

At the top of `src-tauri/src/lib.rs`, add:
```rust
mod settings;
```

**Step 3: Verify it compiles**

Run:
```bash
cd /Users/israelmirsky/claude-usage-widget && cargo check --manifest-path src-tauri/Cargo.toml
```

**Step 4: Commit**

```bash
cd /Users/israelmirsky/claude-usage-widget && git add src-tauri/src/settings.rs src-tauri/src/lib.rs && git commit -m "feat: add settings module with JSON persistence"
```

---

### Task 3: Build Notifications Module

**Files:**
- Create: `src-tauri/src/notifications.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod notifications;`)

**Step 1: Create `src-tauri/src/notifications.rs`**

```rust
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

struct Metric<'a> {
    key: &'a str,
    label: &'a str,
    percent: f64,
    reset_info: &'a str,
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
            key: &format!("{}_session", provider),
            label: &format!("{} session", provider),
            percent: data.session.percent_used,
            reset_info: &data.session.reset_info,
        },
        Metric {
            key: &format!("{}_weekly", provider),
            label: &format!("{} weekly", provider),
            percent: data.weekly_all.percent_used,
            reset_info: &data.weekly_all.reset_info,
        },
        Metric {
            key: &format!("{}_sonnet", provider),
            label: &data.weekly_sonnet.label,
            percent: data.weekly_sonnet.percent_used,
            reset_info: &data.weekly_sonnet.reset_info,
        },
        Metric {
            key: &format!("{}_extra", provider),
            label: &format!("{} extra usage", provider),
            percent: data.extra.percent_used,
            reset_info: &data.extra.reset_date,
        },
    ];

    let mut notified = state.notified.lock().unwrap();

    for m in &metrics {
        let was_notified = notified.get(m.key).copied().unwrap_or(false);

        if m.percent >= threshold_f && !was_notified {
            // Crossed above threshold - fire notification
            let title = format!("{} at {:.0}%", m.label, m.percent);
            let body = m.reset_info.to_string();
            let _ = app
                .notification()
                .builder()
                .title(&title)
                .body(&body)
                .show();
            notified.insert(m.key.to_string(), true);
        } else if m.percent < threshold_f && was_notified {
            // Dropped back below threshold - reset
            notified.insert(m.key.to_string(), false);
        }
    }
}
```

**Step 2: Add module declaration to lib.rs**

Add to the module declarations at top of `src-tauri/src/lib.rs`:
```rust
mod notifications;
```

**Step 3: Verify it compiles**

Run:
```bash
cd /Users/israelmirsky/claude-usage-widget && cargo check --manifest-path src-tauri/Cargo.toml
```

**Step 4: Commit**

```bash
cd /Users/israelmirsky/claude-usage-widget && git add src-tauri/src/notifications.rs src-tauri/src/lib.rs && git commit -m "feat: add notification module with threshold crossing detection"
```

---

### Task 4: Build Right-Click Tray Menu

This is the biggest task. Replace the current bare tray setup in `lib.rs` with a full context menu.

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Step 1: Add menu imports**

At the top of `lib.rs`, update imports to:

```rust
mod codex_fetcher;
mod cookie_reader;
mod notifications;
mod settings;
mod usage_fetcher;

use notifications::NotificationState;
use settings::SettingsState;
use codex_fetcher::CodexState;
use tauri::{
    image::Image,
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
use tauri_plugin_autostart::MacosLauncher;
use usage_fetcher::{UsageData, UsageState};
```

**Step 2: Add Tauri commands for settings**

Add these commands to `lib.rs`:

```rust
#[tauri::command]
fn get_settings(state: tauri::State<'_, SettingsState>) -> settings::Settings {
    state.get()
}

#[tauri::command]
fn get_refresh_interval(state: tauri::State<'_, SettingsState>) -> u64 {
    state.get().refresh_interval_secs
}
```

**Step 3: Build the tray menu in setup**

Replace the entire `setup` closure in `run()` with:

```rust
.setup(|app| {
    // Initialize settings
    let data_dir = app.path().app_data_dir().expect("no app data dir");
    let settings_state = SettingsState::new(data_dir);
    let initial_settings = settings_state.get();
    app.manage(settings_state);
    app.manage(NotificationState::new());

    // Sync autostart with saved setting
    {
        use tauri_plugin_autostart::ManagerExt;
        let mgr = app.autolaunch();
        if initial_settings.start_at_login {
            let _ = mgr.enable();
        } else {
            let _ = mgr.disable();
        }
    }

    // --- Build tray menu ---
    let show_hide = MenuItem::with_id(app, "show_hide", "Show Widget", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let refresh_now = MenuItem::with_id(app, "refresh_now", "Refresh Now", true, None::<&str>)?;

    // Refresh interval submenu
    let intervals: [(u64, &str); 5] = [
        (60, "1 min"), (120, "2 min"), (300, "5 min"), (600, "10 min"), (900, "15 min"),
    ];
    let mut interval_items: Vec<CheckMenuItem<tauri::Wry>> = Vec::new();
    for (secs, label) in &intervals {
        let item = CheckMenuItem::with_id(
            app,
            format!("interval_{}", secs),
            *label,
            true,
            *secs == initial_settings.refresh_interval_secs,
            None::<&str>,
        )?;
        interval_items.push(item);
    }
    let interval_refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> =
        interval_items.iter().map(|i| i as &dyn tauri::menu::IsMenuItem<tauri::Wry>).collect();
    let refresh_sub = Submenu::with_id_and_items(app, "refresh_sub", "Refresh Every", true, &interval_refs)?;

    // Notification threshold submenu
    let thresholds: [(u32, &str); 5] = [
        (70, "70%"), (80, "80%"), (90, "90%"), (95, "95%"), (0, "Off"),
    ];
    let mut threshold_items: Vec<CheckMenuItem<tauri::Wry>> = Vec::new();
    for (pct, label) in &thresholds {
        let checked = if *pct == 0 {
            !initial_settings.notifications_enabled
        } else {
            initial_settings.notifications_enabled && *pct == initial_settings.notify_threshold
        };
        let item = CheckMenuItem::with_id(
            app,
            format!("notify_{}", pct),
            *label,
            true,
            checked,
            None::<&str>,
        )?;
        threshold_items.push(item);
    }
    let threshold_refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> =
        threshold_items.iter().map(|i| i as &dyn tauri::menu::IsMenuItem<tauri::Wry>).collect();
    let notify_sub = Submenu::with_id_and_items(app, "notify_sub", "Notify At", true, &threshold_refs)?;

    // Start at login
    let start_login = CheckMenuItem::with_id(
        app, "start_login", "Start at Login", true,
        initial_settings.start_at_login, None::<&str>,
    )?;

    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[
        &show_hide,
        &sep1,
        &refresh_now,
        &refresh_sub,
        &notify_sub,
        &start_login,
        &sep2,
        &quit,
    ])?;

    // Build tray
    let icon = Image::new(&[0u8; 4], 1, 1);
    let _tray = TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(true)
        .title("C:--% X:--%")
        .tooltip("Usage Widget")
        .menu(&menu)
        .menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .on_menu_event(move |app, event| {
            let id = event.id().as_ref();
            match id {
                "show_hide" => {
                    if let Some(w) = app.get_webview_window("main") {
                        if w.is_visible().unwrap_or(false) {
                            let _ = w.hide();
                        } else {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                }
                "refresh_now" => {
                    let _ = app.emit("usage-refresh-tick", ());
                }
                "quit" => {
                    app.exit(0);
                }
                s if s.starts_with("interval_") => {
                    if let Ok(secs) = s.strip_prefix("interval_").unwrap().parse::<u64>() {
                        let ss = app.state::<SettingsState>();
                        let _ = ss.update(|s| s.refresh_interval_secs = secs);
                        // Update radio checks
                        for (v, _) in &intervals {
                            if let Some(item) = app.menu().and_then(|m| m.get(format!("interval_{}", v).as_str())) {
                                if let Some(check) = item.as_check_menuitem() {
                                    let _ = check.set_checked(*v == secs);
                                }
                            }
                        }
                        // Restart timer
                        let _ = app.emit("refresh-interval-changed", secs);
                    }
                }
                s if s.starts_with("notify_") => {
                    if let Ok(pct) = s.strip_prefix("notify_").unwrap().parse::<u32>() {
                        let ss = app.state::<SettingsState>();
                        let _ = ss.update(|s| {
                            if pct == 0 {
                                s.notifications_enabled = false;
                            } else {
                                s.notifications_enabled = true;
                                s.notify_threshold = pct;
                            }
                        });
                        // Update radio checks
                        for (v, _) in &thresholds {
                            if let Some(item) = app.menu().and_then(|m| m.get(format!("notify_{}", v).as_str())) {
                                if let Some(check) = item.as_check_menuitem() {
                                    let expected = if *v == 0 { pct == 0 } else { pct != 0 && *v == pct };
                                    let _ = check.set_checked(expected);
                                }
                            }
                        }
                    }
                }
                "start_login" => {
                    let ss = app.state::<SettingsState>();
                    let new_val = !ss.get().start_at_login;
                    let _ = ss.update(|s| s.start_at_login = new_val);
                    // Toggle autostart
                    {
                        use tauri_plugin_autostart::ManagerExt;
                        let mgr = app.autolaunch();
                        if new_val { let _ = mgr.enable(); } else { let _ = mgr.disable(); }
                    }
                    // Update check state
                    if let Some(item) = app.menu().and_then(|m| m.get("start_login")) {
                        if let Some(check) = item.as_check_menuitem() {
                            let _ = check.set_checked(new_val);
                        }
                    }
                }
                _ => {}
            }
        })
        .build(app)?;

    // Auto-refresh timer - reads interval from settings
    let handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let secs = {
                let ss = handle.state::<SettingsState>();
                ss.get().refresh_interval_secs
            };
            tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
            let _ = handle.emit("usage-refresh-tick", ());
        }
    });

    Ok(())
})
```

**Step 4: Register plugins and new state in the builder**

Update the builder chain (before `.setup()`):

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_shell::init())
    .plugin(tauri_plugin_notification::init())
    .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None::<Vec<&str>>))
    .manage(UsageState::new())
    .manage(CodexState::new())
    .setup(|app| {
        // ... (the setup closure above)
    })
    .invoke_handler(tauri::generate_handler![
        fetch_claude_usage,
        get_cached_claude,
        fetch_codex_usage,
        get_cached_codex,
        update_tray_text,
        toggle_pin,
        get_settings,
        get_refresh_interval,
    ])
```

**Step 5: Verify it compiles**

Run:
```bash
cd /Users/israelmirsky/claude-usage-widget && cargo check --manifest-path src-tauri/Cargo.toml
```

**Step 6: Commit**

```bash
cd /Users/israelmirsky/claude-usage-widget && git add src-tauri/src/lib.rs && git commit -m "feat: add right-click tray menu with settings, notifications, autostart"
```

---

### Task 5: Wire Notifications Into Fetch Commands

**Files:**
- Modify: `src-tauri/src/lib.rs` (update `fetch_claude_usage` and `fetch_codex_usage`)

**Step 1: Update fetch_claude_usage to fire notifications**

```rust
#[tauri::command]
async fn fetch_claude_usage(
    app: tauri::AppHandle,
    state: tauri::State<'_, UsageState>,
    settings: tauri::State<'_, SettingsState>,
    notif_state: tauri::State<'_, NotificationState>,
) -> Result<UsageData, String> {
    let cookies = cookie_reader::read_claude_cookies().map_err(|e| e.to_string())?;
    let data = usage_fetcher::fetch_usage(&cookies, &state.client).await?;
    *state.last_data.lock().unwrap() = Some(data.clone());

    let s = settings.get();
    notifications::check_and_notify(
        &app, "Claude", &data, s.notify_threshold, s.notifications_enabled, &notif_state,
    );

    Ok(data)
}
```

**Step 2: Update fetch_codex_usage similarly**

```rust
#[tauri::command]
async fn fetch_codex_usage(
    app: tauri::AppHandle,
    usage_state: tauri::State<'_, UsageState>,
    codex_state: tauri::State<'_, CodexState>,
    settings: tauri::State<'_, SettingsState>,
    notif_state: tauri::State<'_, NotificationState>,
) -> Result<UsageData, String> {
    let data = codex_fetcher::fetch_codex_usage(&usage_state.client).await?;
    *codex_state.last_data.lock().unwrap() = Some(data.clone());

    let s = settings.get();
    notifications::check_and_notify(
        &app, "Codex", &data, s.notify_threshold, s.notifications_enabled, &notif_state,
    );

    Ok(data)
}
```

**Step 3: Verify it compiles**

Run:
```bash
cd /Users/israelmirsky/claude-usage-widget && cargo check --manifest-path src-tauri/Cargo.toml
```

**Step 4: Commit**

```bash
cd /Users/israelmirsky/claude-usage-widget && git add src-tauri/src/lib.rs && git commit -m "feat: wire notifications into usage fetch commands"
```

---

### Task 6: Test Dev Mode

**Step 1: Run the app**

```bash
cd /Users/israelmirsky/claude-usage-widget && npm run tauri dev
```

**Step 2: Verify**

- Left-click tray text: panel shows/hides
- Right-click tray text: context menu appears with all items
- "Refresh Now" triggers data fetch
- "Refresh Every" submenu shows radio checks, changing selection works
- "Notify At" submenu shows radio checks, changing selection works
- "Start at Login" toggles
- "Quit" exits app
- Settings persist across restart (check `~/Library/Application Support/com.israelmirsky.claude-codex-usage/settings.json`)

**Step 3: Commit any fixes**

```bash
cd /Users/israelmirsky/claude-usage-widget && git add -A && git commit -m "fix: address issues found during dev testing"
```

---

### Task 7: Build and Package .dmg

**Files:**
- Modify: `src-tauri/tauri.conf.json` (if needed)

**Step 1: Build the production app**

Run:
```bash
cd /Users/israelmirsky/claude-usage-widget && npm run tauri build
```

Expected: `.app` and `.dmg` in `src-tauri/target/release/bundle/`

**Step 2: Test the built app**

Open the `.dmg`, drag app to a temporary location, launch it. Verify:
- Tray text appears
- Left-click opens panel
- Right-click opens settings menu
- All settings work
- Notifications fire (if a metric is above threshold)
- Quit works

**Step 3: Commit**

```bash
cd /Users/israelmirsky/claude-usage-widget && git add -A && git commit -m "chore: production build verified"
```

---

## Task Dependency Order

```
Task 1 (Dependencies)
  ├→ Task 2 (Settings module)
  ├→ Task 3 (Notifications module)
  └─── both feed into ───→ Task 4 (Tray menu)
                               └→ Task 5 (Wire notifications)
                                    └→ Task 6 (Dev test)
                                         └→ Task 7 (Build .dmg)
```

Tasks 2 and 3 can run in parallel after Task 1.
