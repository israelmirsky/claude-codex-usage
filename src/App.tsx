import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import UsagePanel from "./components/UsagePanel";

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

function App() {
  const [provider, setProvider] = useState<Provider>("claude");
  const [claudeData, setClaudeData] = useState<UsageData | null>(null);
  const [codexData, setCodexData] = useState<UsageData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [pinned, setPinned] = useState(false);
  const [refreshing, setRefreshing] = useState(false);

  const data = provider === "claude" ? claudeData : provider === "codex" ? codexData : claudeData;

  const fetchUsage = useCallback(async () => {
    try {
      setRefreshing(true);
      setError(null);

      // Fetch both providers in parallel
      const [claudeResult, codexResult] = await Promise.allSettled([
        invoke<UsageData>("fetch_claude_usage"),
        invoke<UsageData>("fetch_codex_usage"),
      ]);

      if (claudeResult.status === "fulfilled") {
        setClaudeData(claudeResult.value);
      }
      if (codexResult.status === "fulfilled") {
        setCodexData(codexResult.value);
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

      await invoke("update_tray_text", {
        claudeSession: cSession ?? -1,
        claudeWeekly: cWeekly ?? -1,
        codexSession: xSession ?? -1,
        codexWeekly: xWeekly ?? -1,
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

  useEffect(() => {
    // Try cached data first
    invoke<UsageData | null>("get_cached_claude").then((cached) => {
      if (cached) { setClaudeData(cached); setLoading(false); }
    });
    invoke<UsageData | null>("get_cached_codex").then((cached) => {
      if (cached) { setCodexData(cached); }
    });

    fetchUsage();

    const unlisten = listen("usage-refresh-tick", () => {
      fetchUsage();
    });

    return () => { unlisten.then((fn) => fn()); };
  }, [fetchUsage]);

  return (
    <UsagePanel
      data={data}
      claudeData={claudeData}
      codexData={codexData}
      loading={loading}
      error={error}
      pinned={pinned}
      refreshing={refreshing}
      provider={provider}
      onRefresh={fetchUsage}
      onTogglePin={handleTogglePin}
      onSwitchProvider={handleSwitchProvider}
    />
  );
}

export default App;
