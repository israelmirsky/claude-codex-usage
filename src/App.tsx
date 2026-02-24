import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import UsagePanel from "./components/UsagePanel";
import SettingsModal from "./components/SettingsModal";

type Provider = "claude" | "codex" | "both";

interface UsageCategory {
  label: string;
  percent_used: number;
  reset_info: string;
}

interface ExtraData {
  dollars_spent: number;
  percent_used: number;
  reset_date: string;
  enabled: boolean;
}

interface UsageData {
  session: UsageCategory;
  weekly_all: UsageCategory;
  weekly_sonnet: UsageCategory;
  extra: ExtraData;
  fetched_at: string;
}

interface OpenRouterCreditsData {
  total_credits: number;
  total_usage: number;
  remaining_credits: number;
  fetched_at: string;
}

interface OpenRouterKeyStatus {
  configured: boolean;
  masked_key: string | null;
}

function App() {
  const [provider, setProvider] = useState<Provider>("claude");
  const [claudeData, setClaudeData] = useState<UsageData | null>(null);
  const [codexData, setCodexData] = useState<UsageData | null>(null);
  const [openRouterData, setOpenRouterData] = useState<OpenRouterCreditsData | null>(null);
  const [openRouterError, setOpenRouterError] = useState<string | null>(null);
  const [openRouterKeyStatus, setOpenRouterKeyStatus] = useState<OpenRouterKeyStatus | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsSaving, setSettingsSaving] = useState(false);
  const [settingsError, setSettingsError] = useState<string | null>(null);
  const [openRouterKeyDraft, setOpenRouterKeyDraft] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [pinned, setPinned] = useState(false);
  const [refreshing, setRefreshing] = useState(false);

  const data = provider === "claude" ? claudeData : provider === "codex" ? codexData : claudeData;

  const loadOpenRouterKeyStatus = useCallback(async () => {
    try {
      const status = await invoke<OpenRouterKeyStatus>("get_openrouter_key_status");
      setOpenRouterKeyStatus(status);
    } catch (err) {
      console.error("Failed to load OpenRouter key status:", err);
    }
  }, []);

  const fetchUsage = useCallback(async () => {
    try {
      setRefreshing(true);
      setError(null);

      // Fetch both providers in parallel
      const [claudeResult, codexResult, openRouterResult] = await Promise.allSettled([
        invoke<UsageData>("fetch_claude_usage"),
        invoke<UsageData>("fetch_codex_usage"),
        invoke<OpenRouterCreditsData>("fetch_openrouter_credits"),
      ]);

      if (claudeResult.status === "fulfilled") {
        setClaudeData(claudeResult.value);
      }
      if (codexResult.status === "fulfilled") {
        setCodexData(codexResult.value);
      }
      if (openRouterResult.status === "fulfilled") {
        setOpenRouterData(openRouterResult.value);
        setOpenRouterError(null);
      } else {
        const reason = String(openRouterResult.reason);
        const missingKey = reason.toLowerCase().includes("openrouter_api_key is not set")
          || reason.toLowerCase().includes("openrouter_api_key is empty");
        setOpenRouterData(null);
        setOpenRouterError(missingKey ? null : reason);
      }

      // Update tray text with both providers
      const cSession = claudeResult.status === "fulfilled"
        ? Math.round(claudeResult.value.session.percent_used) : null;
      const cWeekly = claudeResult.status === "fulfilled"
        ? Math.round(claudeResult.value.weekly_all.percent_used) : null;
      const xSession = codexResult.status === "fulfilled"
        ? Math.round(codexResult.value.session.percent_used) : null;
      const xWeekly = codexResult.status === "fulfilled"
        ? Math.round(codexResult.value.weekly_all.percent_used) : null;
      const openRouterRemaining = openRouterResult.status === "fulfilled"
        ? openRouterResult.value.remaining_credits
        : null;

      await invoke("update_tray_text", {
        claudeSession: cSession ?? -1,
        claudeWeekly: cWeekly ?? -1,
        codexSession: xSession ?? -1,
        codexWeekly: xWeekly ?? -1,
        openrouterRemaining: openRouterRemaining ?? -1,
      });

      // Show error only if the active provider failed
      if (provider === "claude" && claudeResult.status === "rejected") {
        setError(String(claudeResult.reason));
      } else if (provider === "codex" && codexResult.status === "rejected") {
        setError(String(codexResult.reason));
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  }, [provider]);

  const handleTogglePin = useCallback(async () => {
    const next = !pinned;
    try {
      await invoke("toggle_pin", { pinned: next });
      setPinned(next);
    } catch (err) {
      console.error("Failed to toggle pin:", err);
    }
  }, [pinned]);

  const handleSwitchProvider = useCallback((p: Provider) => {
    setProvider(p);
    setError(null);
  }, []);

  const handleOpenSettings = useCallback(() => {
    setSettingsError(null);
    setOpenRouterKeyDraft("");
    setSettingsOpen(true);
  }, []);

  const handleSaveOpenRouterKey = useCallback(async () => {
    const key = openRouterKeyDraft.trim();
    if (!key) {
      setSettingsError("Enter an API key.");
      return;
    }
    try {
      setSettingsSaving(true);
      setSettingsError(null);
      await invoke("set_openrouter_key", { apiKey: key });
      setOpenRouterKeyDraft("");
      await loadOpenRouterKeyStatus();
      await fetchUsage();
      setSettingsOpen(false);
    } catch (err) {
      setSettingsError(err instanceof Error ? err.message : String(err));
    } finally {
      setSettingsSaving(false);
    }
  }, [openRouterKeyDraft, loadOpenRouterKeyStatus, fetchUsage]);

  const handleClearOpenRouterKey = useCallback(async () => {
    try {
      setSettingsSaving(true);
      setSettingsError(null);
      await invoke("clear_openrouter_key");
      setOpenRouterData(null);
      setOpenRouterError(null);
      setOpenRouterKeyDraft("");
      await loadOpenRouterKeyStatus();
      await fetchUsage();
    } catch (err) {
      setSettingsError(err instanceof Error ? err.message : String(err));
    } finally {
      setSettingsSaving(false);
    }
  }, [loadOpenRouterKeyStatus, fetchUsage]);

  useEffect(() => {
    // Try cached data first
    invoke<UsageData | null>("get_cached_claude").then((cached) => {
      if (cached) { setClaudeData(cached); setLoading(false); }
    });
    invoke<UsageData | null>("get_cached_codex").then((cached) => {
      if (cached) { setCodexData(cached); }
    });
    invoke<OpenRouterCreditsData | null>("get_cached_openrouter").then((cached) => {
      if (cached) { setOpenRouterData(cached); }
    });
    loadOpenRouterKeyStatus();

    fetchUsage();

    const unlistenUsage = listen("usage-refresh-tick", () => {
      fetchUsage();
    });
    const unlistenSettings = listen("open-settings", () => {
      handleOpenSettings();
    });

    return () => {
      unlistenUsage.then((fn) => fn());
      unlistenSettings.then((fn) => fn());
    };
  }, [fetchUsage, loadOpenRouterKeyStatus, handleOpenSettings]);

  return (
    <>
      <UsagePanel
        data={data}
        claudeData={claudeData}
        codexData={codexData}
        openRouterData={openRouterData}
        openRouterError={openRouterError}
        loading={loading}
        error={error}
        pinned={pinned}
        refreshing={refreshing}
        provider={provider}
        onRefresh={fetchUsage}
        onTogglePin={handleTogglePin}
        onSwitchProvider={handleSwitchProvider}
      />
      <SettingsModal
        open={settingsOpen}
        saving={settingsSaving}
        keyDraft={openRouterKeyDraft}
        keyStatus={openRouterKeyStatus}
        error={settingsError}
        onChangeKey={setOpenRouterKeyDraft}
        onSave={handleSaveOpenRouterKey}
        onClear={handleClearOpenRouterKey}
        onClose={() => setSettingsOpen(false)}
      />
    </>
  );
}

export default App;
