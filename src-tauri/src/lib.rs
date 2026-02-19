mod cookie_reader;
mod usage_fetcher;

use tauri::{
    tray::TrayIconBuilder,
    Emitter,
    Manager,
};
use usage_fetcher::{UsageData, UsageState};

#[tauri::command]
fn get_session_cookie() -> Result<String, String> {
    cookie_reader::read_claude_session_cookie().map_err(|e| e.to_string())
}

#[tauri::command]
async fn fetch_usage_data(state: tauri::State<'_, UsageState>) -> Result<UsageData, String> {
    let cookie = cookie_reader::read_claude_session_cookie().map_err(|e| e.to_string())?;
    let data = usage_fetcher::fetch_usage(&cookie, &state.client).await?;
    *state.last_data.lock().unwrap() = Some(data.clone());
    Ok(data)
}

#[tauri::command]
fn get_cached_usage(state: tauri::State<'_, UsageState>) -> Option<UsageData> {
    state.last_data.lock().unwrap().clone()
}

#[tauri::command]
fn update_tray_text(app: tauri::AppHandle, session_pct: u32, weekly_pct: u32) -> Result<(), String> {
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_title(Some(&format!("S:{}% W:{}%", session_pct, weekly_pct)))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn toggle_pin(window: tauri::WebviewWindow, pinned: bool) -> Result<(), String> {
    window.set_always_on_top(pinned).map_err(|e| e.to_string())?;
    window.set_decorations(pinned).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(UsageState::new())
        .setup(|app| {
            // Build system tray
            let _tray = TrayIconBuilder::with_id("main")
                .title("S:--% W:--%")
                .tooltip("Claude Usage Widget")
                .on_tray_icon_event(|tray, event| {
                    use tauri::tray::TrayIconEvent;
                    if let TrayIconEvent::Click { .. } = event {
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
                .build(app)?;

            // Auto-refresh timer
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(300)).await;
                    let _ = handle.emit("usage-refresh-tick", ());
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_session_cookie,
            fetch_usage_data,
            get_cached_usage,
            update_tray_text,
            toggle_pin,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
