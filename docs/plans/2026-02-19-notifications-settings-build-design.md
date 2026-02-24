# Notifications, Settings Menu & Build - Design Doc

## Overview

Three additions to the Claude/Codex Usage Widget:
1. Native macOS notifications when usage approaches configurable limits
2. Right-click tray context menu with settings (refresh interval, notification threshold, start at login, show/hide, quit)
3. Production build and `.dmg` packaging

## 1. Notifications

**Mechanism:** macOS native notifications via `tauri-plugin-notification`.

**Trigger logic:** On each data fetch, compare every metric against the user's threshold. Fire a notification only when a metric **crosses** the threshold (transition from below to above), not on every refresh while above it. Track "already notified" state per metric, reset when the metric drops back below threshold.

**Metrics checked (all of them):**
- Claude: session, weekly (all models), weekly (Sonnet), extra usage
- Codex: session (primary window), weekly (secondary window), model-specific, credits

**Notification format:** `"Claude session at 85% - resets in 2h 15m"`

**Default threshold:** 80%. Configurable via tray menu.

## 2. Right-Click Tray Menu

Native macOS context menu (Tauri `Menu` + `MenuItem` API):

```
Show/Hide Widget
────────────────────
Refresh Now
Refresh Every  ▸  1 min / 2 min / 5 min ✓ / 10 min / 15 min
Notify At      ▸  70% / 80% ✓ / 90% / 95% / Off
Start at Login    ✓
────────────────────
Quit
```

- **Show/Hide Widget:** Toggles the floating panel visibility.
- **Refresh Now:** Triggers an immediate data fetch.
- **Refresh Every:** Submenu with radio-style check on the active interval. Changes the background timer.
- **Notify At:** Submenu with radio-style check on the active threshold. "Off" disables notifications.
- **Start at Login:** Checkbox toggle. Uses `tauri-plugin-autostart`.
- **Quit:** Exits the application.

Left-click continues to toggle the panel as before.

## 3. Settings Persistence

**File:** `~/Library/Application Support/com.israelmirsky.claude-usage-widget/settings.json`

**Schema:**
```json
{
  "refresh_interval_secs": 300,
  "notify_threshold": 80,
  "notifications_enabled": true,
  "start_at_login": false
}
```

Loaded on app start with defaults if missing. Written on every settings change from the tray menu.

## 4. Start at Login

Use `tauri-plugin-autostart` which manages macOS login items natively.

## 5. Build & Package

- Set app metadata in `tauri.conf.json` (product name, identifier, minimum macOS version)
- Run `npm run tauri build` to produce `.app` bundle and `.dmg`
- Output in `src-tauri/target/release/bundle/`

## Architecture Impact

**New Rust modules:**
- `settings.rs` - Load/save/default settings, expose as Tauri managed state
- `notifications.rs` - Threshold comparison, crossing detection, macOS notification dispatch

**Modified modules:**
- `lib.rs` - Add tray right-click menu, wire settings state, register new commands, restart timer on interval change
- `usage_fetcher.rs` / `codex_fetcher.rs` - No changes (notification logic lives in lib.rs after fetch)

**New Tauri plugins:**
- `tauri-plugin-notification`
- `tauri-plugin-autostart`

**Frontend changes:** None. All settings are in the native tray menu.
