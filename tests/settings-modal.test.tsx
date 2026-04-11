import { render, screen } from '@testing-library/react';
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
    await user.type(
      screen.getByPlaceholderText('CommandOrControl+Shift+S'),
      'A',
    );

    expect(updates).toHaveLength(2);
  });
});
