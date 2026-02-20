//! Claude/Codex Usage - a macOS menu bar widget for monitoring AI rate limits.
//!
//! This is the Tauri backend that fetches usage data from Claude.ai and OpenAI Codex,
//! manages a system tray icon with live usage percentages, and serves data to the
//! React frontend via Tauri IPC commands.

mod codex_fetcher;
mod cookie_reader;
mod notifications;
mod settings;
mod usage_fetcher;

use codex_fetcher::CodexState;
use notifications::NotificationState;
use settings::SettingsState;
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::TrayIconBuilder,
    Emitter, Manager,
};
use tauri_plugin_autostart::MacosLauncher;
use usage_fetcher::{UsageData, UsageState};

// --- Tauri commands ---

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

#[tauri::command]
fn get_cached_claude(state: tauri::State<'_, UsageState>) -> Option<UsageData> {
    state.last_data.lock().unwrap().clone()
}

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

#[tauri::command]
fn get_cached_codex(state: tauri::State<'_, CodexState>) -> Option<UsageData> {
    state.last_data.lock().unwrap().clone()
}

#[tauri::command]
fn update_tray_text(
    app: tauri::AppHandle,
    claude_session: i32,
    claude_weekly: i32,
    codex_session: i32,
    codex_weekly: i32,
) -> Result<(), String> {
    if let Some(tray) = app.tray_by_id("main") {
        let mut parts = Vec::new();
        if claude_session >= 0 && claude_weekly >= 0 {
            parts.push(format!("C:{}/{}%", claude_session, claude_weekly));
        }
        if codex_session >= 0 && codex_weekly >= 0 {
            parts.push(format!("X:{}/{}%", codex_session, codex_weekly));
        }
        let text = if parts.is_empty() {
            "Usage: --%".to_string()
        } else {
            parts.join("  ")
        };
        tray.set_title(Some(&text)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn toggle_pin(window: tauri::WebviewWindow, pinned: bool) -> Result<(), String> {
    window
        .set_always_on_top(pinned)
        .map_err(|e| e.to_string())?;
    window.set_decorations(pinned).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_settings(state: tauri::State<'_, SettingsState>) -> settings::Settings {
    state.get()
}

#[tauri::command]
fn get_refresh_interval(state: tauri::State<'_, SettingsState>) -> u64 {
    state.get().refresh_interval_secs
}

// --- App setup ---

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None::<Vec<&str>>,
        ))
        .manage(UsageState::new())
        .manage(CodexState::new())
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

            // --- Build tray context menu ---
            let show_hide =
                MenuItem::with_id(app, "show_hide", "Show Widget", true, None::<&str>)?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let refresh_now =
                MenuItem::with_id(app, "refresh_now", "Refresh Now", true, None::<&str>)?;

            // Refresh interval submenu (radio-style check items)
            let intervals: [(u64, &str); 5] = [
                (60, "1 min"),
                (120, "2 min"),
                (300, "5 min"),
                (600, "10 min"),
                (900, "15 min"),
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
            let interval_refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = interval_items
                .iter()
                .map(|i| i as &dyn tauri::menu::IsMenuItem<tauri::Wry>)
                .collect();
            let refresh_sub = Submenu::with_id_and_items(
                app,
                "refresh_sub",
                "Refresh Every",
                true,
                &interval_refs,
            )?;

            // Notification threshold submenu (radio-style check items)
            let thresholds: [(u32, &str); 5] =
                [(70, "70%"), (80, "80%"), (90, "90%"), (95, "95%"), (0, "Off")];
            let mut threshold_items: Vec<CheckMenuItem<tauri::Wry>> = Vec::new();
            for (pct, label) in &thresholds {
                let checked = if *pct == 0 {
                    !initial_settings.notifications_enabled
                } else {
                    initial_settings.notifications_enabled
                        && *pct == initial_settings.notify_threshold
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
            let threshold_refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = threshold_items
                .iter()
                .map(|i| i as &dyn tauri::menu::IsMenuItem<tauri::Wry>)
                .collect();
            let notify_sub = Submenu::with_id_and_items(
                app,
                "notify_sub",
                "Notify At",
                true,
                &threshold_refs,
            )?;

            // Start at login toggle
            let start_login = CheckMenuItem::with_id(
                app,
                "start_login",
                "Start at Login",
                true,
                initial_settings.start_at_login,
                None::<&str>,
            )?;

            let sep2 = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[
                    &show_hide,
                    &sep1,
                    &refresh_now,
                    &refresh_sub,
                    &notify_sub,
                    &start_login,
                    &sep2,
                    &quit,
                ],
            )?;

            // Build tray (text-only, no icon)
            let menu_ref = menu.clone();
            let _tray = TrayIconBuilder::with_id("main")
                .title("C:--% X:--%")
                .tooltip("Usage Widget")
                .menu(&menu)
                .show_menu_on_left_click(true)
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
                                // Update radio checks - uncheck all, check selected
                                for (v, _) in &intervals {
                                    let item_id = format!("interval_{}", v);
                                    if let Some(item) = menu_ref.get(&item_id) {
                                        if let Some(check) = item.as_check_menuitem() {
                                            let _ = check.set_checked(*v == secs);
                                        }
                                    }
                                }
                                // Notify frontend about interval change
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
                                    let item_id = format!("notify_{}", v);
                                    if let Some(item) = menu_ref.get(&item_id) {
                                        if let Some(check) = item.as_check_menuitem() {
                                            let expected = if *v == 0 {
                                                pct == 0
                                            } else {
                                                pct != 0 && *v == pct
                                            };
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
                                if new_val {
                                    let _ = mgr.enable();
                                } else {
                                    let _ = mgr.disable();
                                }
                            }
                            // Update check state
                            if let Some(item) = menu_ref.get("start_login") {
                                if let Some(check) = item.as_check_menuitem() {
                                    let _ = check.set_checked(new_val);
                                }
                            }
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            // Auto-refresh timer - reads interval from settings dynamically
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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
