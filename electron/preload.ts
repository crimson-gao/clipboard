import { contextBridge, ipcRenderer } from 'electron';

import type { ClipFilter, ClipItem, WindowState } from './types';

type Unsubscribe = () => void;

const api = {
  listClips: (query?: string, filter?: ClipFilter): Promise<ClipItem[]> =>
    ipcRenderer.invoke('clips:list', { query, filter }),
  toggleFavorite: (id: number): Promise<ClipItem | null> => ipcRenderer.invoke('clips:toggle-favorite', id),
  copyClip: (id: number): Promise<boolean> => ipcRenderer.invoke('clips:copy', id),
  writeClipToClipboard: (id: number): Promise<boolean> => ipcRenderer.invoke('clips:write', id),
  clearCurrentClips: (ids: number[]): Promise<boolean> => ipcRenderer.invoke('clips:clear-current', ids),
  pasteClipAndHide: (id: number): Promise<boolean> => ipcRenderer.invoke('clips:paste-and-hide', id),
  showClipInFinder: (id: number): Promise<boolean> => ipcRenderer.invoke('clips:show-in-finder', id),
  togglePinWindow: (): Promise<WindowState> => ipcRenderer.invoke('window:toggle-pin'),
  getWindowState: (): Promise<WindowState> => ipcRenderer.invoke('window:get-state'),
  subscribeClipsChanged: (listener: () => void): Unsubscribe => {
    const wrapped = () => listener();
    ipcRenderer.on('clips:changed', wrapped);
    return () => {
      ipcRenderer.removeListener('clips:changed', wrapped);
    };
  },
};

contextBridge.exposeInMainWorld('clipboardApp', api);
