import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type {
  ClipCounts,
  ClipFilter,
  ClipImagePreview,
  ClipItem,
  PaginatedClips,
  WindowState,
} from '../types';

type Unsubscribe = () => void;

type EventUnlisten = () => void;

function subscribeEvent(event: string, listener: () => void): Unsubscribe {
  let unlisten: EventUnlisten | null = null;
  void listen(event, () => {
    listener();
  }).then((dispose) => {
    unlisten = dispose;
  });

  return () => {
    unlisten?.();
  };
}

export const clipboardApi = {
  listClipsPage: (
    query?: string,
    filter?: ClipFilter,
    offset?: number,
    limit?: number,
  ): Promise<PaginatedClips> =>
    invoke('list_clips_page', { query, filter, offset, limit }),
  getClipImagePreview: (id: number): Promise<ClipImagePreview | null> =>
    invoke('get_clip_image_preview', { id }),
  getClipCounts: (): Promise<ClipCounts> => invoke('get_clip_counts'),
  toggleFavorite: (id: number): Promise<ClipItem | null> =>
    invoke('toggle_favorite', { id }),
  copyClip: (id: number): Promise<boolean> => invoke('copy_clip', { id }),
  clearCurrentClips: (ids: number[]): Promise<boolean> =>
    invoke('clear_current_clips', { ids }),
  pasteClipAndHide: (id: number): Promise<boolean> =>
    invoke('paste_clip_and_hide', { id }),
  showClipInFinder: (id: number): Promise<boolean> =>
    invoke('show_clip_in_finder', { id }),
  togglePinWindow: (): Promise<WindowState> => invoke('toggle_pin_window'),
  getWindowState: (): Promise<WindowState> => invoke('get_window_state'),
  updateWindowSettings: (
    shortcut: string,
    shortcutEnabled: boolean,
    showTrayIcon: boolean,
  ): Promise<WindowState> =>
    invoke('update_window_settings', {
      shortcut,
      shortcutEnabled,
      showTrayIcon,
    }),
  openSettingsWindow: (): Promise<void> => invoke('open_settings_window'),
  subscribeClipsChanged: (listener: () => void): Unsubscribe =>
    subscribeEvent('clips:changed', listener),
};
