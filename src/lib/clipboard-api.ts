import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { ClipCounts, ClipFilter, ClipItem, PaginatedClips, WindowState } from '../types';

type Unsubscribe = () => void;

type EventUnlisten = () => void;

export const clipboardApi = {
  listClips: (query?: string, filter?: ClipFilter): Promise<ClipItem[]> =>
    invoke('list_clips', { query, filter }),
  listClipsPage: (
    query?: string,
    filter?: ClipFilter,
    offset?: number,
    limit?: number,
  ): Promise<PaginatedClips> => invoke('list_clips_page', { query, filter, offset, limit }),
  getClipCounts: (): Promise<ClipCounts> => invoke('get_clip_counts'),
  toggleFavorite: (id: number): Promise<ClipItem | null> =>
    invoke('toggle_favorite', { id }),
  copyClip: (id: number): Promise<boolean> => invoke('copy_clip', { id }),
  writeClipToClipboard: (id: number): Promise<boolean> => invoke('copy_clip', { id }),
  clearCurrentClips: (ids: number[]): Promise<boolean> => invoke('clear_current_clips', { ids }),
  pasteClipAndHide: (id: number): Promise<boolean> => invoke('paste_clip_and_hide', { id }),
  showClipInFinder: (id: number): Promise<boolean> => invoke('show_clip_in_finder', { id }),
  togglePinWindow: (): Promise<WindowState> => invoke('toggle_pin_window'),
  getWindowState: (): Promise<WindowState> => invoke('get_window_state'),
  subscribeClipsChanged: (listener: () => void): Unsubscribe => {
    let unlisten: EventUnlisten | null = null;
    void listen('clips:changed', () => {
      listener();
    }).then((dispose) => {
      unlisten = dispose;
    });

    return () => {
      unlisten?.();
    };
  },
};
