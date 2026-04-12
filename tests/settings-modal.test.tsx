import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { SettingsModal } from '../src/features/settings/SettingsModal';
import type { SettingsDraft } from '../src/features/clips/clip-utils';

describe('SettingsModal', () => {
  it('updates draft fields through callbacks', async () => {
    const user = userEvent.setup();
    const updates: Array<(draft: SettingsDraft) => SettingsDraft> = [];

    render(
      <SettingsModal
        draft={{
          shortcut: 'CommandOrControl+Shift+S',
          shortcutEnabled: true,
          showTrayIcon: true,
        }}
        isSaving={false}
        bindTooltip={() => ({
          onMouseEnter: () => {},
          onMouseMove: () => {},
          onMouseLeave: () => {},
          onFocus: () => {},
          onBlur: () => {},
        })}
        onClose={() => {}}
        onDraftChange={(updater) => {
          updates.push(updater);
        }}
      />,
    );

    await user.click(screen.getByRole('checkbox', { name: '已启用' }));
    const shortcutInput = screen.getByPlaceholderText(
      'CommandOrControl+Shift+S',
    );
    await user.click(shortcutInput);
    fireEvent.keyDown(shortcutInput, {
      key: 'k',
      metaKey: true,
      shiftKey: true,
    });

    expect(updates).toHaveLength(2);
    expect(updates[1]({
      shortcut: 'CommandOrControl+Shift+S',
      shortcutEnabled: true,
      showTrayIcon: true,
    })).toEqual({
      shortcut: 'CommandOrControl+Shift+K',
      shortcutEnabled: true,
      showTrayIcon: true,
    });
  });

  it('clears shortcut when delete is pressed during recording', async () => {
    const user = userEvent.setup();
    const updates: Array<(draft: SettingsDraft) => SettingsDraft> = [];

    render(
      <SettingsModal
        draft={{
          shortcut: 'CommandOrControl+Shift+S',
          shortcutEnabled: true,
          showTrayIcon: true,
        }}
        isSaving={false}
        bindTooltip={() => ({
          onMouseEnter: () => {},
          onMouseMove: () => {},
          onMouseLeave: () => {},
          onFocus: () => {},
          onBlur: () => {},
        })}
        onClose={() => {}}
        onDraftChange={(updater) => {
          updates.push(updater);
        }}
      />,
    );

    const shortcutInput = screen.getByPlaceholderText(
      'CommandOrControl+Shift+S',
    );
    await user.click(shortcutInput);
    fireEvent.keyDown(shortcutInput, {
      key: 'Backspace',
    });

    expect(updates).toHaveLength(1);
    expect(updates[0]({
      shortcut: 'CommandOrControl+Shift+S',
      shortcutEnabled: true,
      showTrayIcon: true,
    })).toEqual({
      shortcut: '',
      shortcutEnabled: true,
      showTrayIcon: true,
    });
  });
});
