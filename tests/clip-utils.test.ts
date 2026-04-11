import {
  buildSettingsDraft,
  formatUpdatedAt,
  getClipMeta,
  getClipSummary,
} from '../src/features/clips/clip-utils';
import type { ClipItem, WindowState } from '../src/types';

function buildClip(overrides: Partial<ClipItem>): ClipItem {
  return {
    id: 1,
    type: 'text',
    contentText: 'hello world',
    isFavorite: false,
    createdAt: '2026-04-12T00:00:00.000Z',
    updatedAt: '2026-04-12T00:00:00.000Z',
    contentPath: null,
    filePaths: [],
    imageWidth: null,
    imageHeight: null,
    ...overrides,
  };
}

describe('clip utils', () => {
  it('builds text and file summaries', () => {
    expect(getClipSummary(buildClip({ contentText: 'hello\n  world' }))).toBe(
      'hello\n world',
    );
    expect(
      getClipSummary(
        buildClip({
          type: 'file',
          filePaths: ['/tmp/demo.txt'],
          contentText: '/tmp/demo.txt',
        }),
      ),
    ).toBe('/tmp/demo.txt');
  });

  it('builds metadata for image entries', () => {
    expect(
      getClipMeta(
        buildClip({
          type: 'image',
          imageWidth: 320,
          imageHeight: 200,
        }),
      ),
    ).toBe('320 x 200');
  });

  it('formats recent times and settings draft', () => {
    const now = new Date().toISOString();
    expect(formatUpdatedAt(now)).toBe('刚刚');

    const state: WindowState = {
      isPinned: true,
      shortcut: 'CommandOrControl+Shift+S',
      shortcutEnabled: true,
      showTrayIcon: false,
    };

    expect(buildSettingsDraft(state)).toEqual({
      shortcut: 'CommandOrControl+Shift+S',
      shortcutEnabled: true,
      showTrayIcon: false,
    });
  });
});
