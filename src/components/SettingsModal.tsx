import "./SettingsModal.css";

interface OpenRouterKeyStatus {
  configured: boolean;
  masked_key: string | null;
}

interface SettingsModalProps {
  open: boolean;
  saving: boolean;
  keyDraft: string;
  keyStatus: OpenRouterKeyStatus | null;
  error: string | null;
  onChangeKey: (value: string) => void;
  onSave: () => void;
  onClear: () => void;
  onClose: () => void;
}

export default function SettingsModal({
  open,
  saving,
  keyDraft,
  keyStatus,
  error,
  onChangeKey,
  onSave,
  onClear,
  onClose,
}: SettingsModalProps) {
  if (!open) return null;

  return (
    <div className="settings-modal__overlay">
      <div className="settings-modal">
        <div className="settings-modal__header">
          <div className="settings-modal__title">Settings</div>
        </div>

        <div className="settings-modal__section">
          <label className="settings-modal__label">OpenRouter API key</label>
          <input
            type="password"
            className="settings-modal__input"
            value={keyDraft}
            onChange={(e) => onChangeKey(e.target.value)}
            placeholder="sk-or-..."
            autoFocus
          />
          <div className="settings-modal__meta">
            Current: {keyStatus?.configured ? keyStatus.masked_key : "Not configured"}
          </div>
        </div>

        {error && <div className="settings-modal__error">{error}</div>}

        <div className="settings-modal__actions">
          <button className="settings-modal__btn" onClick={onClose} disabled={saving}>
            Cancel
          </button>
          <button
            className="settings-modal__btn settings-modal__btn--danger"
            onClick={onClear}
            disabled={saving || !keyStatus?.configured}
          >
            Clear key
          </button>
          <button
            className="settings-modal__btn settings-modal__btn--primary"
            onClick={onSave}
            disabled={saving || keyDraft.trim().length === 0}
          >
            {saving ? "Saving..." : "Save"}
          </button>
        </div>
      </div>
    </div>
  );
}
