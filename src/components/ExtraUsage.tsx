import "./UsageBar.css";

interface ExtraUsageProps {
  dollarsSpent: number;
  percent: number;
  resetDate: string;
  enabled: boolean;
}

export default function ExtraUsage({
  dollarsSpent,
  percent,
  resetDate,
  enabled,
}: ExtraUsageProps) {
  const clamped = Math.min(100, Math.max(0, percent));

  return (
    <div className="usage-bar">
      <div className="usage-bar__header">
        <span className="usage-bar__label">
          ${dollarsSpent.toFixed(2)} spent
        </span>
        <span className="usage-bar__percent">
          <span
            style={{
              display: "inline-block",
              fontSize: 9,
              padding: "1px 5px",
              borderRadius: 3,
              marginRight: 6,
              background: enabled ? "#2d4a2d" : "#4a2d2d",
              color: enabled ? "#6fcf6f" : "#cf6f6f",
            }}
          >
            {enabled ? "On" : "Off"}
          </span>
          {Math.round(percent)}% used
        </span>
      </div>
      <div className="usage-bar__track">
        <div
          className="usage-bar__fill"
          style={{ width: `${clamped}%` }}
        />
      </div>
      {resetDate && (
        <div className="usage-bar__reset">Resets {resetDate}</div>
      )}
    </div>
  );
}
