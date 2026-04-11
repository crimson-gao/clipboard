import type { SettingsDraft } from '../clips/clip-utils';

type TooltipProps = {
  onMouseEnter: React.MouseEventHandler<HTMLElement>;
  onMouseMove: React.MouseEventHandler<HTMLElement>;
  onMouseLeave: React.MouseEventHandler<HTMLElement>;
  onFocus: React.FocusEventHandler<HTMLElement>;
  onBlur: React.FocusEventHandler<HTMLElement>;
};

type SettingsModalProps = {
  draft: SettingsDraft;
  isSaving: boolean;
  bindTooltip: (text: string) => TooltipProps;
  onClose: () => void;
  onDraftChange: (updater: (draft: SettingsDraft) => SettingsDraft) => void;
};

export function SettingsModal({
  draft,
  isSaving,
  bindTooltip,
  onClose,
  onDraftChange,
}: SettingsModalProps) {
  return (
    <div className="settings-modal-backdrop" onClick={onClose}>
      <section
        className="settings-modal"
        aria-label="设置"
        onClick={(event) => {
          event.stopPropagation();
        }}
      >
        <div className="settings-modal-header">
          <div>
            <p className="settings-modal-title">设置</p>
            <span className="settings-modal-subtitle">修改后会自动保存</span>
          </div>
          <button
            type="button"
            className="settings-close"
            aria-label="关闭设置"
            {...bindTooltip('关闭设置')}
            onClick={onClose}
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M6 6l12 12M18 6 6 18" />
            </svg>
          </button>
        </div>

        <div className="settings-card-grid">
          <section className="settings-card">
            <div className="settings-card-header">
              <div>
                <p className="settings-label">全局快捷键</p>
                <span className="settings-hint">
                  默认使用 Cmd + Shift + S 呼出面板
                </span>
              </div>
              <label className="settings-switch">
                <input
                  type="checkbox"
                  checked={draft.shortcutEnabled}
                  onChange={(event) => {
                    onDraftChange((current) => ({
                      ...current,
                      shortcutEnabled: event.target.checked,
                    }));
                  }}
                />
                <span>{draft.shortcutEnabled ? '已启用' : '已禁用'}</span>
              </label>
            </div>

            <label className="settings-input">
              <span>快捷键字符串</span>
              <input
                type="text"
                value={draft.shortcut}
                disabled={!draft.shortcutEnabled}
                onChange={(event) => {
                  onDraftChange((current) => ({
                    ...current,
                    shortcut: event.target.value,
                  }));
                }}
                placeholder="CommandOrControl+Shift+S"
              />
            </label>
          </section>

          <section className="settings-card">
            <div className="settings-card-header">
              <div>
                <p className="settings-label">菜单栏图标</p>
                <span className="settings-hint">
                  点击 menu bar 图标也可以呼出面板
                </span>
              </div>
              <label className="settings-switch">
                <input
                  type="checkbox"
                  checked={draft.showTrayIcon}
                  onChange={(event) => {
                    onDraftChange((current) => ({
                      ...current,
                      showTrayIcon: event.target.checked,
                    }));
                  }}
                />
                <span>{draft.showTrayIcon ? '显示中' : '已隐藏'}</span>
              </label>
            </div>
          </section>
        </div>

        <div className="settings-status" aria-live="polite">
          {isSaving ? '正在保存设置…' : '设置已同步'}
        </div>
      </section>
    </div>
  );
}
