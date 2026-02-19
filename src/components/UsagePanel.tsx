import UsageBar from "./UsageBar";
import ExtraUsage from "./ExtraUsage";
import "./UsagePanel.css";

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

interface UsagePanelProps {
  data: UsageData | null;
  claudeData: UsageData | null;
  codexData: UsageData | null;
  loading: boolean;
  error: string | null;
  pinned: boolean;
  refreshing: boolean;
  provider: Provider;
  onRefresh: () => void;
  onTogglePin: () => void;
  onSwitchProvider: (p: Provider) => void;
}

function ProviderSection({ title, data }: { title: string; data: UsageData }) {
  return (
    <>
      <div className="usage-panel__provider-header">{title}</div>
      <UsageBar
        label={data.session.label}
        percent={data.session.percent_used}
        resetInfo={data.session.reset_info}
      />
      <UsageBar
        label={data.weekly_all.label}
        percent={data.weekly_all.percent_used}
        resetInfo={data.weekly_all.reset_info}
      />
    </>
  );
}

export default function UsagePanel({
  data,
  claudeData,
  codexData,
  loading,
  error,
  pinned,
  refreshing,
  provider,
  onRefresh,
  onTogglePin,
  onSwitchProvider,
}: UsagePanelProps) {
  const isBoth = provider === "both";

  return (
    <div className="usage-panel">
      <div className="usage-panel__tabs">
        <button
          className={`usage-panel__tab ${provider === "claude" ? "usage-panel__tab--active" : ""}`}
          onClick={() => onSwitchProvider("claude")}
        >
          Claude
        </button>
        <button
          className={`usage-panel__tab ${provider === "codex" ? "usage-panel__tab--active" : ""}`}
          onClick={() => onSwitchProvider("codex")}
        >
          Codex
        </button>
        <button
          className={`usage-panel__tab ${provider === "both" ? "usage-panel__tab--active" : ""}`}
          onClick={() => onSwitchProvider("both")}
        >
          Both
        </button>
        <div className="usage-panel__tab-actions">
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

      {loading && !data && !isBoth && (
        <div className="usage-panel__loading">Loading...</div>
      )}
      {loading && isBoth && !claudeData && !codexData && (
        <div className="usage-panel__loading">Loading...</div>
      )}

      {/* Combined "Both" view */}
      {isBoth && (claudeData || codexData) && (
        <>
          {claudeData && (
            <div className="usage-panel__section">
              <ProviderSection title="Claude" data={claudeData} />
            </div>
          )}
          {codexData && (
            <div className="usage-panel__section">
              <ProviderSection title="Codex" data={codexData} />
            </div>
          )}
          <div className="usage-panel__footer">
            Updated: {new Date(
              claudeData?.fetched_at || codexData?.fetched_at || ""
            ).toLocaleTimeString()}
          </div>
        </>
      )}

      {/* Single provider view */}
      {!isBoth && data && (
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
