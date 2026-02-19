# Claude.ai Usage API Discovery

Research conducted: 2026-02-18

## Table of Contents

1. [Summary of Approaches](#summary-of-approaches)
2. [Approach 1: OAuth Usage API (Recommended)](#approach-1-oauth-usage-api-recommended)
3. [Approach 2: Cookie-Based Web API](#approach-2-cookie-based-web-api)
4. [Approach 3: Admin API (Organization-Level)](#approach-3-admin-api-organization-level)
5. [Approach 4: Enterprise Analytics API](#approach-4-enterprise-analytics-api)
6. [Response JSON Structure](#response-json-structure)
7. [Authentication Details](#authentication-details)
8. [Known Gotchas and Caveats](#known-gotchas-and-caveats)
9. [Open-Source Implementations](#open-source-implementations)
10. [Sources](#sources)

---

## Summary of Approaches

There are four distinct ways to access Claude usage data, each with different authentication requirements and data granularity:

| Approach | Endpoint | Auth Method | Data Returned | Best For |
|----------|----------|-------------|---------------|----------|
| OAuth Usage API | `api.anthropic.com/api/oauth/usage` | OAuth Bearer token | 5h/7d utilization % | Personal usage widget |
| Cookie-Based Web API | `claude.ai/api/organizations/{orgId}/usage` | `sessionKey` cookie | 5h/7d utilization % | Browser extensions |
| Admin API | `api.anthropic.com/v1/organizations/usage_report/messages` | Admin API key | Token counts, cost data | Org-level reporting |
| Enterprise Analytics | `api.anthropic.com/v1/organizations/analytics/*` | API key with `read:analytics` | User activity, seats | Enterprise dashboards |

---

## Approach 1: OAuth Usage API (Recommended)

This is the most reliable approach for personal usage tracking. It uses the same OAuth token that Claude Code stores locally.

### Endpoint

```
GET https://api.anthropic.com/api/oauth/usage
```

### Required Headers

```http
Accept: application/json, text/plain, */*
Content-Type: application/json
User-Agent: claude-code/2.0.32
Authorization: Bearer <oauth_access_token>
anthropic-beta: oauth-2025-04-20
Accept-Encoding: gzip, compress, deflate, br
Host: api.anthropic.com
```

### How to Get the OAuth Token

Claude Code stores its OAuth credentials in the macOS Keychain. Retrieve them with:

```bash
security find-generic-password -s "Claude Code-credentials" -w
```

This returns a JSON string containing:

```json
{
  "claudeAiOauth": {
    "accessToken": "sk-ant-oat01-...",
    "refreshToken": "sk-ant-ort01-...",
    "expiresAt": 1738972800000,
    "scopes": ["user:inference", "user:profile"]
  }
}
```

The `accessToken` is used as the Bearer token. Access tokens start with `sk-ant-oat01-`.

### Response JSON

```json
{
  "five_hour": {
    "utilization": 37.0,
    "resets_at": "2026-02-08T04:59:59.000000+00:00"
  },
  "seven_day": {
    "utilization": 26.0,
    "resets_at": "2026-02-12T14:59:59.771647+00:00"
  },
  "seven_day_oauth_apps": null,
  "seven_day_opus": {
    "utilization": 0.0,
    "resets_at": null
  },
  "seven_day_sonnet": {
    "utilization": 12.0,
    "resets_at": "2026-02-12T14:59:59.771647+00:00"
  },
  "extra_usage": {
    "is_enabled": false,
    "monthly_limit": null,
    "used_credits": null,
    "utilization": null
  },
  "iguana_necktie": null
}
```

### Scope Requirement

The usage endpoint requires the `user:profile` OAuth scope. Tokens generated via `claude setup-token` only grant `user:inference` and will be rejected with a 403. The token must come from a full Claude Code login flow.

---

## Approach 2: Cookie-Based Web API

This is the approach used by browser extensions. It authenticates using the `sessionKey` cookie from claude.ai.

### Endpoints

**Get Organizations:**
```
GET https://claude.ai/api/organizations
```

**Get Usage Data:**
```
GET https://claude.ai/api/organizations/{organizationId}/usage
```

**Get Subscription/Tier Info (via Statsig):**
```
GET https://claude.ai/api/bootstrap/{organizationId}/statsig
```

**Other useful endpoints:**
```
GET https://claude.ai/api/organizations/{orgId}/overage_spend_limit
GET https://claude.ai/api/account_profile
GET https://claude.ai/api/organizations/{orgId}/memory
GET https://claude.ai/api/settings/billing
```

### Authentication

Authentication is cookie-based. The key cookie is `sessionKey`, whose value starts with `sk-ant-sid01-`.

```http
Cookie: sessionKey=sk-ant-sid01-...
```

Additional useful cookies:
- `lastActiveOrg` - Contains the UUID of the user's last active organization

### Request Headers

```http
Accept: application/json
Content-Type: application/json
Cookie: sessionKey=sk-ant-sid01-...
```

Some implementations also include:
```http
anthropic-client-platform: web_claude_ai
User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/115.0
Sec-Fetch-Dest: empty
Sec-Fetch-Mode: cors
Sec-Fetch-Site: same-origin
```

### How to Get the sessionKey

1. Log in to https://claude.ai
2. Open browser DevTools (F12)
3. Go to Application > Cookies > claude.ai
4. Find the `sessionKey` cookie
5. Copy its value (starts with `sk-ant-sid01-`)

### Organization ID

The Organization ID is a UUID. Retrieve it by:
- Calling `GET /api/organizations` and taking `[0].uuid` from the response
- Or checking https://claude.ai/settings/account for the "Organization ID" field
- Or reading the `lastActiveOrg` cookie value

### Response JSON

The response structure for `/api/organizations/{orgId}/usage` is the same as the OAuth endpoint:

```json
{
  "five_hour": {
    "utilization": <number 0-100>,
    "resets_at": "<ISO 8601 timestamp>"
  },
  "seven_day": {
    "utilization": <number 0-100>,
    "resets_at": "<ISO 8601 timestamp>"
  },
  "seven_day_opus": {
    "utilization": <number>,
    "resets_at": "<timestamp or null>"
  },
  "seven_day_sonnet": {
    "utilization": <number>,
    "resets_at": "<timestamp or null>"
  }
}
```

### Cloudflare Protection

**Important caveat:** The claude.ai web endpoints are protected by Cloudflare. Direct HTTP requests (curl, fetch) will likely be blocked. Several open-source projects work around this by:
- Using Playwright with Chromium and anti-bot-detection flags
- Using `curl_cffi` with browser impersonation
- Using the browser extension API to piggyback on existing browser cookies

---

## Approach 3: Admin API (Organization-Level)

This is the official, documented Anthropic API for organization-level usage reporting. It provides token-level granularity but requires an Admin API key.

### Endpoints

**Usage Report (Messages):**
```
GET https://api.anthropic.com/v1/organizations/usage_report/messages
```

**Cost Report:**
```
GET https://api.anthropic.com/v1/organizations/cost_report
```

**Claude Code Usage Report:**
```
GET https://api.anthropic.com/v1/organizations/usage_report/claude_code
```

### Authentication

Requires an Admin API key (starts with `sk-ant-admin...`). Only organization admins can provision these keys from the Claude Console.

```http
x-api-key: sk-ant-admin...
anthropic-version: 2023-06-01
Content-Type: application/json
```

### Query Parameters

```
starting_at     - ISO 8601 timestamp (required)
ending_at       - ISO 8601 timestamp (required)
bucket_width    - "1m", "1h", or "1d"
group_by[]      - "model", "workspace_id", "api_key_id", "service_tier", etc.
models[]        - Filter by specific models
limit           - Pagination limit
page            - Pagination cursor
```

### Response Structure

```json
{
  "data": [
    {
      "bucket_start_time": "2025-01-01T00:00:00Z",
      "uncached_input_tokens": 12345,
      "cached_input_tokens": 6789,
      "cache_creation_tokens": 1000,
      "output_tokens": 5432,
      "model": "claude-opus-4-6"
    }
  ],
  "has_more": false,
  "next_page": null
}
```

---

## Approach 4: Enterprise Analytics API

For enterprise accounts. Provides user activity, seat utilization, and engagement metrics.

### Base URL

```
https://api.anthropic.com/v1/organizations/analytics/
```

### Endpoints

```
GET /v1/organizations/analytics/users        - Per-user activity
GET /v1/organizations/analytics/summaries    - Daily/weekly/monthly active users
GET /v1/organizations/analytics/apps/chat/projects - Project usage
GET /v1/organizations/analytics/skills       - Skill utilization
GET /v1/organizations/analytics/connectors   - MCP/connector usage
```

### Authentication

Requires API key with `read:analytics` scope.

```http
x-api-key: <key with read:analytics scope>
```

---

## Response JSON Structure

### Usage Response Fields (OAuth & Cookie endpoints)

| Field | Type | Description |
|-------|------|-------------|
| `five_hour` | object or null | 5-hour rolling window usage |
| `five_hour.utilization` | number | Percentage used (0-100) |
| `five_hour.resets_at` | string (ISO 8601) | When the 5-hour window resets |
| `seven_day` | object or null | 7-day rolling window usage |
| `seven_day.utilization` | number | Percentage used (0-100) |
| `seven_day.resets_at` | string (ISO 8601) | When the 7-day window resets |
| `seven_day_opus` | object or null | Opus-specific 7-day usage |
| `seven_day_sonnet` | object or null | Sonnet-specific 7-day usage |
| `seven_day_oauth_apps` | object or null | OAuth apps 7-day usage |
| `extra_usage` | object or null | Extra usage info (Max plans) |
| `extra_usage.is_enabled` | boolean | Whether extra usage is enabled |
| `extra_usage.monthly_limit` | number or null | Monthly extra usage limit |
| `extra_usage.used_credits` | number or null | Credits used |
| `extra_usage.utilization` | number or null | Extra usage percentage |
| `iguana_necktie` | null | Unknown/internal field (always null in observed responses) |

### Understanding Utilization Values

- `utilization` is a percentage from 0 to 100
- It represents how much of the rate limit budget has been consumed
- The five-hour window is a rolling session-based limit
- The seven-day window tracks weekly consumption; old usage "expires" as it ages out of the 7-day window (there is no single reset day)
- Model-specific limits (opus, sonnet) may be null if not applicable to the user's plan

### Organizations Response Structure

```json
[
  {
    "uuid": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
    "name": "Organization Name",
    "settings": { ... },
    "capabilities": [ ... ]
  }
]
```

---

## Authentication Details

### Cookie: `sessionKey`

- **Format:** `sk-ant-sid01-<token>`
- **Where to find:** Browser DevTools > Application > Cookies > claude.ai
- **Expiration:** Periodic; requires manual renewal
- **Usage:** Set as `Cookie: sessionKey=<value>` header
- **Limitations:** Cloudflare protection blocks most non-browser requests

### OAuth Access Token

- **Format:** `sk-ant-oat01-<token>`
- **Where to find:** macOS Keychain under service `"Claude Code-credentials"`
- **Retrieval command:** `security find-generic-password -s "Claude Code-credentials" -w`
- **Expiration:** Token has an `expiresAt` field in milliseconds
- **Required scope:** `user:profile` (tokens from `claude setup-token` only have `user:inference`)
- **Usage:** Set as `Authorization: Bearer <token>` header
- **Limitations:** Claude Code refreshes tokens in-memory but does NOT write updated tokens back to Keychain, so Keychain entries may appear expired while the app still works

### OAuth Refresh Token

- **Format:** `sk-ant-ort01-<token>`
- **Single-use:** Each refresh token can only be used once
- **Danger:** If you manually call the refresh endpoint and don't persist the new access token, the old refresh token is consumed and authentication breaks

### Admin API Key

- **Format:** `sk-ant-admin-<token>`
- **Where to find:** Claude Console > Settings > Admin Keys
- **Requirement:** Organization admin role
- **Usage:** Set as `x-api-key: <key>` header with `anthropic-version: 2023-06-01`

---

## Known Gotchas and Caveats

### Cloudflare Protection (Cookie Approach)
The claude.ai web endpoints are protected by Cloudflare bot detection. Simple HTTP requests will be blocked. Workarounds include:
- Playwright/Puppeteer with anti-detection flags (`--disable-blink-features=AutomationControlled`)
- Overriding `navigator.webdriver` via JavaScript injection
- Using `curl_cffi` with `impersonate="chrome110"`
- Browser extension APIs that piggyback on existing sessions

### OAuth Token Staleness
Claude Code refreshes its OAuth tokens in-memory but never writes them back to the Keychain. The Keychain entry may show an expired token while Claude Code continues to work fine. If you need a fresh token, you may need to restart Claude Code or trigger a re-login.

### Refresh Token Single-Use
OAuth refresh tokens are single-use. If you consume one by calling the refresh endpoint directly, you must persist both the new access token AND the new refresh token. Otherwise, authentication is permanently broken until the user re-authenticates through Claude Code.

### Subscription Tier Detection
To determine the user's subscription tier (Pro, Max 5x, Max 20x, Team), check the Statsig bootstrap endpoint:
```
GET https://claude.ai/api/bootstrap/{organizationId}/statsig
```
The tier is encoded in `user.custom.orgType`. Known values include:
- `claude_pro`
- `claude_team`
- `claude_max_5x`
- `claude_max_20x`

### Rate Limit Response Headers
Every Claude API response (both web and API) includes rate limit headers:
- `anthropic-ratelimit-requests-remaining`
- `anthropic-ratelimit-tokens-remaining`
- `anthropic-ratelimit-requests-reset` (RFC 3339 timestamp)
- `retry-after` (seconds)

These can be used as a complementary signal but only appear in response to actual API calls, not on a standalone usage endpoint.

### Console API (Billing) Endpoints
For API console billing (separate from claude.ai web usage), the following cookie-authenticated endpoints exist:
```
GET https://console.anthropic.com/api/organizations                        - List console orgs
GET https://console.anthropic.com/api/organizations/{orgId}/current_spend  - Current spend
GET https://console.anthropic.com/api/organizations/{orgId}/prepaid/credits - Prepaid credits
```
These use the console's `sessionKey` cookie (different from claude.ai's).

---

## Open-Source Implementations

### Browser Extensions

| Project | Language | Auth Method | URL |
|---------|----------|-------------|-----|
| lugia19/Claude-Usage-Extension | JavaScript | Browser cookies (containerFetch) | https://github.com/lugia19/Claude-Usage-Extension |
| jonis100/claude-quota-tracker | TypeScript | sessionKey + Playwright | https://github.com/jonis100/claude-quota-tracker |

### Native Apps

| Project | Language | Auth Method | URL |
|---------|----------|-------------|-----|
| hamed-elfayome/Claude-Usage-Tracker | Swift | sessionKey cookie + CLI OAuth | https://github.com/hamed-elfayome/Claude-Usage-Tracker |
| aaronvstory/claude-usage-tracker-windows | (Windows port) | sessionKey cookie + CLI OAuth | https://github.com/aaronvstory/claude-usage-tracker-windows |
| masorange/ClaudeUsageTracker | Swift | Local JSONL parsing | https://github.com/masorange/ClaudeUsageTracker |

### CLI Tools

| Project | Language | Auth Method | URL |
|---------|----------|-------------|-----|
| Maciek-roboblog/Claude-Code-Usage-Monitor | Python | Local log parsing | https://github.com/Maciek-roboblog/Claude-Code-Usage-Monitor |

### Unofficial API Libraries

| Project | Language | Auth Method | URL |
|---------|----------|-------------|-----|
| Explosion-Scratch/claude-unofficial-api | JavaScript | sessionKey cookie | https://github.com/Explosion-Scratch/claude-unofficial-api |
| KoushikNavuluri/Claude-API | Python | Full cookie string | https://github.com/KoushikNavuluri/Claude-API |
| AshwinPathi/claude-api-py | Python | sessionKey cookie | https://github.com/AshwinPathi/claude-api-py |
| st1vms/unofficial-claude-api | Python | Firefox session gathering | https://github.com/st1vms/unofficial-claude-api |

---

## Recommended Implementation Strategy

For a macOS usage widget, the **OAuth approach** (Approach 1) is recommended because:

1. **No Cloudflare issues** - api.anthropic.com does not have the same bot protection as claude.ai
2. **Token already available** - If Claude Code is installed, the token is in Keychain
3. **Simple HTTP** - Standard GET request with Bearer auth, no browser automation needed
4. **Same data** - Returns the exact same utilization data as the web UI

### Minimal curl Example

```bash
# 1. Get the OAuth token from Keychain
CREDS=$(security find-generic-password -s "Claude Code-credentials" -w 2>/dev/null)
TOKEN=$(echo "$CREDS" | python3 -c "import sys,json; print(json.loads(sys.stdin.read())['claudeAiOauth']['accessToken'])")

# 2. Fetch usage data
curl -s https://api.anthropic.com/api/oauth/usage \
  -H "Authorization: Bearer $TOKEN" \
  -H "anthropic-beta: oauth-2025-04-20" \
  -H "Content-Type: application/json" \
  -H "User-Agent: claude-code/2.1.5"
```

### Token Refresh Considerations

Since Keychain tokens may be stale, a production implementation should:
1. Try the access token from Keychain first
2. If it returns 401, attempt a token refresh using the refresh token
3. If refresh fails, prompt the user to re-authenticate via Claude Code
4. **Never consume a refresh token without persisting the new tokens**

---

## Sources

- [Claude Code Status Line Guide (Gist)](https://gist.github.com/patyearone/7c753ef536a49839c400efaf640e17de) - Most comprehensive reverse-engineered documentation
- [How to Show Claude Code Usage Limits in Your Statusline (codelynx.dev)](https://codelynx.dev/posts/claude-code-usage-limits-statusline) - Detailed walkthrough with curl examples
- [Claude-Usage-Tracker (hamed-elfayome)](https://github.com/hamed-elfayome/Claude-Usage-Tracker) - Swift macOS app source code
- [Claude-Usage-Extension (lugia19)](https://github.com/lugia19/Claude-Usage-Extension) - Browser extension source code
- [claude-quota-tracker (jonis100)](https://github.com/jonis100/claude-quota-tracker) - VS Code extension source code
- [claude-unofficial-api (Explosion-Scratch)](https://github.com/Explosion-Scratch/claude-unofficial-api) - JavaScript unofficial API
- [Claude-API (KoushikNavuluri)](https://github.com/KoushikNavuluri/Claude-API) - Python unofficial API
- [Usage and Cost API - Official Docs](https://platform.claude.com/docs/en/build-with-claude/usage-cost-api) - Official Admin API documentation
- [Claude Enterprise Analytics API](https://support.claude.com/en/articles/13703965-claude-enterprise-analytics-api-reference-guide) - Enterprise analytics endpoints
- [When Does Claude Code Usage Reset (CometAPI)](https://www.cometapi.com/when-does-claude-code-usage-reset/) - Usage window explanations
