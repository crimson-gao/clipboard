import React, { useEffect, useState } from 'react';
import ReactDOM from 'react-dom/client';

import './styles.css';
import './settings-window.css';
import {
  buildSettingsDraft,
  hasSettingsDraftChanges,
  type SettingsDraft,
} from './features/clips/clip-utils';
import { SettingsPanel } from './features/settings/SettingsPanel';
import { clipboardApi } from './lib/clipboard-api';
import type { WindowState } from './types';

export function SettingsApp() {
  const [windowState, setWindowState] = useState<WindowState | null>(null);
  const [draft, setDraft] = useState<SettingsDraft>({
    shortcut: '',
    shortcutEnabled: true,
    showTrayIcon: true,
  });
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    void clipboardApi.getWindowState().then((nextWindowState) => {
      setWindowState(nextWindowState);
      setDraft(buildSettingsDraft(nextWindowState));
    });
  }, []);

  useEffect(() => {
    if (!windowState || !hasSettingsDraftChanges(windowState, draft)) {
      return;
    }

    const timer = window.setTimeout(() => {
      setIsSaving(true);
      void clipboardApi
        .updateWindowSettings(
          draft.shortcut.trim(),
          draft.shortcutEnabled,
          draft.showTrayIcon,
        )
        .then((nextWindowState) => {
          setWindowState(nextWindowState);
        })
        .finally(() => {
          setIsSaving(false);
        });
    }, 220);

    return () => {
      window.clearTimeout(timer);
    };
  }, [draft, windowState]);

  return (
    <main className="settings-window-shell">
      <section className="settings-window-hero">
        <p className="settings-window-eyebrow">Clipboard</p>
        <h1>偏好设置</h1>
        <p className="settings-window-summary">
          管理全局快捷键和菜单栏显示方式。这些设置会立即同步到主应用。
        </p>
      </section>

      <SettingsPanel
        draft={draft}
        isSaving={isSaving}
        onDraftChange={setDraft}
      />
    </main>
  );
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <SettingsApp />
  </React.StrictMode>,
);
