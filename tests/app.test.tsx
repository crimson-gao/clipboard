import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import App from '../src/App';
import type { ClipCounts, ClipItem, PaginatedClips, WindowState } from '../src/types';

const {
  mockListClipsPage,
  mockGetClipCounts,
  mockGetWindowState,
  mockCopyClip,
  mockPasteClipAndHide,
  mockToggleFavorite,
  mockUpdateWindowSettings,
} = vi.hoisted(() => ({
  mockListClipsPage: vi.fn<
    (query?: string, filter?: string, offset?: number, limit?: number) => Promise<PaginatedClips>
  >(),
  mockGetClipCounts: vi.fn<() => Promise<ClipCounts>>(),
  mockGetWindowState: vi.fn<() => Promise<WindowState>>(),
  mockCopyClip: vi.fn<(id: number) => Promise<boolean>>(),
  mockPasteClipAndHide: vi.fn<(id: number) => Promise<boolean>>(),
  mockToggleFavorite: vi.fn<(id: number) => Promise<ClipItem | null>>(),
  mockUpdateWindowSettings: vi.fn<
    (
      shortcut: string,
      shortcutEnabled: boolean,
      showTrayIcon: boolean,
    ) => Promise<WindowState>
  >(),
}));

vi.mock('../src/lib/clipboard-api', () => ({
  clipboardApi: {
    listClipsPage: mockListClipsPage,
    getClipCounts: mockGetClipCounts,
    getWindowState: mockGetWindowState,
    copyClip: mockCopyClip,
    pasteClipAndHide: mockPasteClipAndHide,
    toggleFavorite: mockToggleFavorite,
    updateWindowSettings: mockUpdateWindowSettings,
    getClipImagePreview: vi.fn(),
    clearCurrentClips: vi.fn().mockResolvedValue(true),
    showClipInFinder: vi.fn().mockResolvedValue(true),
    togglePinWindow: vi.fn(),
    subscribeClipsChanged: vi.fn(() => () => {}),
    subscribeOpenSettings: vi.fn(() => () => {}),
  },
}));

const clips: ClipItem[] = [
  {
    id: 1,
    type: 'text',
    contentText: 'first clip',
    isFavorite: false,
    createdAt: '2026-04-12T04:00:00.000Z',
    updatedAt: '2026-04-12T04:00:00.000Z',
    contentPath: null,
    filePaths: [],
    imageWidth: null,
    imageHeight: null,
  },
  {
    id: 2,
    type: 'text',
    contentText: 'second clip',
    isFavorite: false,
    createdAt: '2026-04-12T05:00:00.000Z',
    updatedAt: '2026-04-12T05:00:00.000Z',
    contentPath: null,
    filePaths: [],
    imageWidth: null,
    imageHeight: null,
  },
];

const counts: ClipCounts = {
  text: 2,
  image: 0,
  file: 0,
  favorite: 0,
  dataVersion: 1,
};

const windowState: WindowState = {
  isPinned: false,
  shortcut: 'CommandOrControl+Shift+S',
  shortcutEnabled: true,
  showTrayIcon: true,
};

function setupDomApis() {
  Object.defineProperty(window.HTMLElement.prototype, 'scrollIntoView', {
    configurable: true,
    value: vi.fn(),
  });

  Object.defineProperty(navigator, 'clipboard', {
    configurable: true,
    value: {
      writeText: vi.fn(),
    },
  });

  class MockIntersectionObserver {
    disconnect() {}
    observe() {}
  }

  vi.stubGlobal('IntersectionObserver', MockIntersectionObserver);
}

describe('App', () => {
  beforeEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
    setupDomApis();

    mockListClipsPage.mockResolvedValue({
      items: clips,
      total: clips.length,
      hasMore: false,
    });
    mockGetClipCounts.mockResolvedValue(counts);
    mockGetWindowState.mockResolvedValue(windowState);
    mockCopyClip.mockResolvedValue(true);
    mockPasteClipAndHide.mockResolvedValue(true);
    mockToggleFavorite.mockResolvedValue(null);
    mockUpdateWindowSettings.mockImplementation(
      async (shortcut, shortcutEnabled, showTrayIcon) => ({
        isPinned: false,
        shortcut: shortcut || windowState.shortcut,
        shortcutEnabled,
        showTrayIcon,
      }),
    );
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('uses keyboard navigation for paste and copy actions', async () => {
    render(<App />);

    await screen.findByText('first clip');
    await screen.findByText('second clip');

    await waitFor(() => {
      expect(screen.getByText('2 条')).toBeInTheDocument();
    });

    fireEvent.keyDown(window, { key: 'ArrowDown' });
    fireEvent.keyDown(window, { key: 'Enter' });
    fireEvent.keyDown(window, { key: 'c', ctrlKey: true });

    await waitFor(() => {
      expect(mockPasteClipAndHide).toHaveBeenCalledWith(2);
      expect(mockCopyClip).toHaveBeenCalledWith(2);
    });
  });

  it('debounces settings autosave through the backend API', async () => {
    const user = userEvent.setup();

    render(<App />);

    await screen.findByRole('button', { name: '设置' });
    await user.click(screen.getByRole('button', { name: '设置' }));

    const shortcutInput = await screen.findByPlaceholderText(
      'CommandOrControl+Shift+S',
    );

    await user.click(shortcutInput);
    fireEvent.keyDown(shortcutInput, {
      key: 'k',
      metaKey: true,
      shiftKey: true,
    });

    expect(mockUpdateWindowSettings).not.toHaveBeenCalled();

    await waitFor(() => {
      expect(mockUpdateWindowSettings).toHaveBeenLastCalledWith(
        'CommandOrControl+Shift+K',
        true,
        true,
      );
    });
  });
});
