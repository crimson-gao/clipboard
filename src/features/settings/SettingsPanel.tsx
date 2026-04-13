import type { SettingsDraft } from '../clips/clip-utils';
import { ShortcutRecorder } from './ShortcutRecorder';

type SettingsPanelProps = {
  draft: SettingsDraft;
  isSaving: boolean;
  onDraftChange: (updater: (draft: SettingsDraft) => SettingsDraft) => void;
};

export function SettingsPanel({
  draft,
  isSaving,
  onDraftChange,
}: SettingsPanelProps) {
  const updateDraft =
    <K extends keyof SettingsDraft>(key: K) =>
    (value: SettingsDraft[K]) => {
      onDraftChange((current) => ({
        ...current,
        [key]: value,
      }));
    };

  const updateShortcutEnabled = updateDraft('shortcutEnabled');
  const updateShortcut = updateDraft('shortcut');
  const updateShowTrayIcon = updateDraft('showTrayIcon');

  return (
    <section className="settings-window-panel" aria-label="设置">
      <div className="settings-modal-header">
        <div>
          <p className="settings-modal-title">设置</p>
          <span className="settings-modal-subtitle">修改后会自动保存</span>
        </div>
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
                  updateShortcutEnabled(event.target.checked);
                }}
              />
              <span>{draft.shortcutEnabled ? '已启用' : '已禁用'}</span>
            </label>
          </div>

          <label className="settings-input">
            <span>录制快捷键</span>
            <ShortcutRecorder
              value={draft.shortcut}
              disabled={!draft.shortcutEnabled}
              onChange={updateShortcut}
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
                  updateShowTrayIcon(event.target.checked);
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
  );
}
