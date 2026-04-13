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
import { useDebouncedValue } from './hooks/useDebouncedValue';
import { clipboardApi } from './lib/clipboard-api';
import type { WindowState } from './types';

const DEFAULT_SETTINGS_DRAFT: SettingsDraft = {
  shortcut: '',
  shortcutEnabled: true,
  showTrayIcon: true,
};

export function SettingsApp() {
  const [windowState, setWindowState] = useState<WindowState | null>(null);
  const [draft, setDraft] = useState<SettingsDraft>(DEFAULT_SETTINGS_DRAFT);
  const [isSaving, setIsSaving] = useState(false);
  const debouncedDraft = useDebouncedValue(draft, 220);

  useEffect(() => {
    void clipboardApi.getWindowState().then((nextWindowState) => {
      setWindowState(nextWindowState);
      setDraft(buildSettingsDraft(nextWindowState));
    });
  }, []);

  useEffect(() => {
    if (!windowState || !hasSettingsDraftChanges(windowState, debouncedDraft)) {
      return;
    }

    let isCurrent = true;

    setIsSaving(true);
    void clipboardApi
      .updateWindowSettings(
        debouncedDraft.shortcut.trim(),
        debouncedDraft.shortcutEnabled,
        debouncedDraft.showTrayIcon,
      )
      .then((nextWindowState) => {
        if (!isCurrent) {
          return;
        }

        setWindowState(nextWindowState);
        setDraft(buildSettingsDraft(nextWindowState));
      })
      .finally(() => {
        if (isCurrent) {
          setIsSaving(false);
        }
      });

    return () => {
      isCurrent = false;
    };
  }, [debouncedDraft, windowState]);

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
