# Claude/Codex Usage

A macOS menu bar widget that shows your Claude.ai and OpenAI Codex rate limit usage at a glance.

![macOS](https://img.shields.io/badge/platform-macOS-lightgrey)
![Tauri 2](https://img.shields.io/badge/Tauri-2-blue)

## What it does

- Displays live usage percentages in the macOS menu bar: `C:25/62%  X:0/17%`
- Click the tray to open a floating panel with detailed usage bars
- Three tabs: **Claude**, **Codex**, and **Both** (combined view)
- Auto-refreshes every 5 minutes
- Pin the widget to keep it always visible

### Claude tab
- **Session** (5-hour window) utilization
- **Weekly** (7-day) all-model and Sonnet-specific limits
- **Extra usage** spending and on/off status

### Codex tab
- **Primary window** (5-hour session) utilization
- **Secondary window** (7-day weekly) utilization
- **Model-specific limits** (e.g., GPT-5.3-Codex-Spark)
- **Credit balance**

## Prerequisites

- **macOS 13+**
- **Claude desktop app** installed and signed in (the widget reads its cookies to authenticate with claude.ai)
- **Codex CLI** installed and authenticated via `codex --login` (stores token at `~/.codex/auth.json`)

Either or both can be configured - the widget gracefully handles missing providers.

## How it authenticates

No API keys or passwords are stored in this app. Authentication works by reading locally-stored credentials:

| Provider | Source | What it reads |
|----------|--------|---------------|
| Claude | `~/Library/Application Support/Claude/Cookies` | Encrypted session cookies (decrypted via macOS Keychain) |
| Codex | `~/.codex/auth.json` | OAuth access token written by `codex --login` |

All credential access stays local. The app makes API calls to:
- `https://claude.ai/api/organizations/{org_id}/usage` (Claude)
- `https://chatgpt.com/backend-api/wham/usage` (Codex)

## Building from source

### Requirements

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) (18+)

### Steps

```bash
git clone https://github.com/israelmirsky/claude-codex-usage.git
cd claude-codex-usage
npm install
npm run tauri dev     # development mode
npm run tauri build   # production .app bundle
```

The built app will be in `src-tauri/target/release/bundle/macos/`.

## Project structure

```
src/                          # React frontend
  App.tsx                     # Main app - manages state, fetches data
  components/
    UsagePanel.tsx             # Tab UI with Claude/Codex/Both views
    UsageBar.tsx               # Reusable progress bar component
    ExtraUsage.tsx             # Extra usage display with On/Off badge

src-tauri/src/                # Rust backend
  lib.rs                      # Tauri app setup, tray icon, IPC commands
  cookie_reader.rs            # Claude desktop app cookie decryption
  usage_fetcher.rs            # Claude.ai usage API client
  codex_fetcher.rs            # OpenAI Codex usage API client
```

## License

MIT
