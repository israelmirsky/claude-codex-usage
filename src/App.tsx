import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import UsagePanel from "./components/UsagePanel";

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
  const [data, setData] = useState<UsageData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [pinned, setPinned] = useState(false);
  const [refreshing, setRefreshing] = useState(false);

  const fetchUsage = useCallback(async () => {
    try {
      setRefreshing(true);
      setError(null);
      const result = await invoke<UsageData>("fetch_usage_data");
      setData(result);
      await invoke("update_tray_text", {
        sessionPct: Math.round(result.session.percent_used),
        weeklyPct: Math.round(result.weekly_all.percent_used),
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  }, []);

  const handleTogglePin = useCallback(async () => {
    const next = !pinned;
    try {
      await invoke("toggle_pin", { pinned: next });
      setPinned(next);
    } catch (err) {
      console.error("Failed to toggle pin:", err);
    }
  }, [pinned]);

  useEffect(() => {
    // Try cached data first for instant display
    invoke<UsageData | null>("get_cached_usage").then((cached) => {
      if (cached) {
        setData(cached);
        setLoading(false);
      }
    });

    // Then fetch fresh data
    fetchUsage();

    // Listen for periodic refresh ticks from the Rust backend
    const unlisten = listen("usage-refresh-tick", () => {
      fetchUsage();
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [fetchUsage]);

  return (
    <UsagePanel
      data={data}
      loading={loading}
      error={error}
      pinned={pinned}
      refreshing={refreshing}
      onRefresh={fetchUsage}
      onTogglePin={handleTogglePin}
    />
  );
}

export default App;
