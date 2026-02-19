import UsageBar from "./UsageBar";
import ExtraUsage from "./ExtraUsage";
import "./UsagePanel.css";

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

interface UsagePanelProps {
  data: UsageData | null;
  loading: boolean;
  error: string | null;
  pinned: boolean;
  refreshing: boolean;
  onRefresh: () => void;
  onTogglePin: () => void;
}

export default function UsagePanel({
  data,
  loading,
  error,
  pinned,
  refreshing,
  onRefresh,
  onTogglePin,
}: UsagePanelProps) {
  return (
    <div className="usage-panel">
      <div className="usage-panel__header">
        <span className="usage-panel__title">Claude Usage</span>
        <div className="usage-panel__actions">
          <button
            className={`usage-panel__btn ${refreshing ? "usage-panel__btn--spinning" : ""}`}
            onClick={onRefresh}
            title="Refresh"
          >
            &#8635;
          </button>
          <button
            className={`usage-panel__btn ${pinned ? "usage-panel__btn--active" : ""}`}
            onClick={onTogglePin}
            title={pinned ? "Unpin window" : "Pin window on top"}
          >
            {pinned ? "\uD83D\uDCCD" : "\uD83D\uDCCC"}
          </button>
        </div>
      </div>

      {error && <div className="usage-panel__error">{error}</div>}

      {loading && !data && (
        <div className="usage-panel__loading">Loading...</div>
      )}

      {data && (
        <>
          <div className="usage-panel__section">
            <div className="usage-panel__section-title">Session</div>
            <UsageBar
              label={data.session.label}
              percent={data.session.percent_used}
              resetInfo={data.session.reset_info}
            />
          </div>

          <div className="usage-panel__section">
            <div className="usage-panel__section-title">Weekly limits</div>
            <UsageBar
              label={data.weekly_all.label}
              percent={data.weekly_all.percent_used}
              resetInfo={data.weekly_all.reset_info}
            />
            <UsageBar
              label={data.weekly_sonnet.label}
              percent={data.weekly_sonnet.percent_used}
              resetInfo={data.weekly_sonnet.reset_info}
            />
          </div>

          <div className="usage-panel__section">
            <div className="usage-panel__section-title">Extra usage</div>
            <ExtraUsage
              dollarsSpent={data.extra.dollars_spent}
              percent={data.extra.percent_used}
              resetDate={data.extra.reset_date}
              enabled={data.extra.enabled}
            />
          </div>

          <div className="usage-panel__footer">
            Updated: {new Date(data.fetched_at).toLocaleTimeString()}
          </div>
        </>
      )}
    </div>
  );
}
