import { execFile } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { promisify } from 'node:util';
import { fileURLToPath } from 'node:url';

import {
  app,
  BrowserWindow,
  clipboard,
  dialog,
  globalShortcut,
  ipcMain,
  nativeImage,
  shell,
} from 'electron';

import { ClipboardWatcher } from './clipboard';
import { deleteClipsByIds, getClipById, initDatabase, listClips, toggleFavorite } from './db';
import { getPreferences, preferenceStore } from './store';
import type { ClipFilter, WindowState } from './types';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const isDev = !app.isPackaged;
const execFileAsync = promisify(execFile);

let mainWindow: BrowserWindow | null = null;
let isAppQuitting = false;
let watcher: ClipboardWatcher | null = null;

function getWindowState(): WindowState {
  const preferences = getPreferences();
  return {
    isPinned: preferences.isPinned,
    shortcut: preferences.shortcut,
  };
}

function notifyClipsChanged(): void {
  mainWindow?.webContents.send('clips:changed');
}

function applyPinnedState(window: BrowserWindow, isPinned: boolean): void {
  window.setAlwaysOnTop(isPinned, 'screen-saver');
}

function showWindow(): void {
  if (!mainWindow) {
    return;
  }

  mainWindow.showInactive();
}

function hideWindow(): void {
  mainWindow?.hide();
}

function toggleWindow(): void {
  if (!mainWindow) {
    return;
  }

  if (mainWindow.isVisible() && mainWindow.isFocused()) {
    hideWindow();
    return;
  }

  showWindow();
}

async function createWindow(): Promise<BrowserWindow> {
  const window = new BrowserWindow({
    width: 980,
    height: 720,
    minWidth: 860,
    minHeight: 560,
    resizable: true,
    movable: true,
    show: false,
    title: 'Clipboard',
    titleBarStyle: 'hiddenInset',
    backgroundColor: '#f4f0ea',
    autoHideMenuBar: true,
    webPreferences: {
      preload: path.join(__dirname, 'preload.mjs'),
      contextIsolation: true,
      nodeIntegration: false,
    },
  });

  applyPinnedState(window, getPreferences().isPinned);

  window.on('blur', () => {
    if (!getPreferences().isPinned) {
      window.hide();
    }
  });

  window.on('close', (event) => {
    if (!isAppQuitting) {
      event.preventDefault();
      window.hide();
    }
  });

  if (isDev && process.env.VITE_DEV_SERVER_URL) {
    await window.loadURL(process.env.VITE_DEV_SERVER_URL);
    window.webContents.openDevTools({ mode: 'detach' });
  } else {
    await window.loadFile(path.join(__dirname, '../dist/index.html'));
  }

  return window;
}

async function pasteViaSystemShortcut(): Promise<boolean> {
  try {
    await execFileAsync('osascript', [
      '-e',
      'tell application "System Events" to keystroke "v" using command down',
    ]);
    return true;
  } catch (error) {
    console.warn('[paste] failed to send system paste shortcut', error);
    return false;
  }
}

function registerShortcuts(): void {
  const shortcut = getPreferences().shortcut;
  const registered = globalShortcut.register(shortcut, () => {
    toggleWindow();
  });

  if (!registered) {
    console.error(`[shortcut] failed to register ${shortcut}`);
  }
}

function writeClipToSystemClipboard(id: number): boolean {
  const clip = getClipById(id);
  if (!clip) {
    return false;
  }

  watcher?.suppressClip(clip);

  if (clip.type === 'image') {
    if (!clip.contentPath || !fs.existsSync(clip.contentPath)) {
      return false;
    }

    clipboard.writeImage(nativeImage.createFromPath(clip.contentPath));
    return true;
  }

  if (clip.type === 'file') {
    const text = clip.filePaths.join('\n');
    clipboard.writeText(text);
    return true;
  }

  clipboard.writeText(clip.contentText);
  return true;
}

function installIpcHandlers(): void {
  ipcMain.handle('clips:list', (_event, payload?: { query?: string; filter?: ClipFilter }) => {
    return listClips(payload?.query, payload?.filter);
  });

  ipcMain.handle('clips:toggle-favorite', (_event, id: number) => {
    const item = toggleFavorite(id);
    notifyClipsChanged();
    return item;
  });

  const writeClip = (_event: Electron.IpcMainInvokeEvent, id: number) => writeClipToSystemClipboard(id);

  ipcMain.handle('clips:copy', writeClip);
  ipcMain.handle('clips:write', writeClip);
  ipcMain.handle('clips:show-in-finder', (_event, id: number) => {
    const clip = getClipById(id);
    if (!clip) {
      return false;
    }

    const targetPath =
      clip.type === 'file' ? clip.filePaths[0] :
      clip.type === 'image' ? clip.contentPath :
      null;

    if (!targetPath || !fs.existsSync(targetPath)) {
      return false;
    }

    shell.showItemInFolder(targetPath);
    return true;
  });
  ipcMain.handle('clips:clear-current', async (_event, ids: number[]) => {
    if (!Array.isArray(ids) || ids.length === 0) {
      return false;
    }

    const targetCount = ids.length;
    const response = await dialog.showMessageBox(mainWindow ?? undefined, {
      type: 'warning',
      buttons: ['取消', '清空'],
      defaultId: 1,
      cancelId: 0,
      title: '清空当前列表',
      message: `确认清空当前列表中的 ${targetCount} 条记录？`,
      detail: '这会删除当前筛选结果中的记录，并可能影响其他 tab 中显示的内容。',
      noLink: true,
    });

    if (response.response !== 1) {
      return false;
    }

    const deleted = deleteClipsByIds(ids);
    if (deleted > 0) {
      notifyClipsChanged();
    }
    return deleted > 0;
  });
  ipcMain.handle('clips:paste-and-hide', async (_event, id: number) => {
    if (!writeClipToSystemClipboard(id)) {
      return false;
    }
    hideWindow();

    await new Promise((resolve) => {
      setTimeout(resolve, 120);
    });

    return pasteViaSystemShortcut();
  });

  ipcMain.handle('window:toggle-pin', () => {
    const next = !getPreferences().isPinned;
    preferenceStore.set('isPinned', next);

    if (mainWindow) {
      applyPinnedState(mainWindow, next);
    }

    return getWindowState();
  });

  ipcMain.handle('window:get-state', () => getWindowState());
}

async function bootstrap(): Promise<void> {
  await app.whenReady();

  initDatabase();
  installIpcHandlers();
  mainWindow = await createWindow();

   if (process.env.ELECTRON_START_VISIBLE === '1') {
    showWindow();
  }

  registerShortcuts();

  watcher = new ClipboardWatcher(() => {
    notifyClipsChanged();
  });
  watcher.start();

  app.on('activate', () => {
    if (!mainWindow) {
      return;
    }

    showWindow();
  });
}

app.on('before-quit', () => {
  isAppQuitting = true;
  watcher?.stop();
  globalShortcut.unregisterAll();
});

void bootstrap();
