import "./UsageBar.css";

interface UsageBarProps {
  label: string;
  percent: number;
  resetInfo: string;
}

export default function UsageBar({ label, percent, resetInfo }: UsageBarProps) {
  const clamped = Math.min(100, Math.max(0, percent));

  return (
    <div className="usage-bar">
      <div className="usage-bar__header">
        <span className="usage-bar__label">{label}</span>
        <span className="usage-bar__percent">{Math.round(percent)}% used</span>
      </div>
      <div className="usage-bar__track">
        <div
          className="usage-bar__fill"
          style={{ width: `${clamped}%` }}
        />
      </div>
      {resetInfo && <div className="usage-bar__reset">{resetInfo}</div>}
    </div>
  );
}
