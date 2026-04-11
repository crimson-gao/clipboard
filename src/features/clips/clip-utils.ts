import type { ClipFilter, ClipItem, WindowState } from '../../types';

export type CategoryKey = 'all' | 'text' | 'image' | 'file' | 'favorite';

export type Category = {
  key: CategoryKey;
  label: string;
  count?: number;
  icon: (active: boolean) => JSX.Element;
};

export type TabSnapshot = {
  items: ClipItem[];
  offset: number;
  hasMore: boolean;
  selectedClipId: number | null;
  expandedClipIds: number[];
  lastLoadedVersion: number;
};

export type SettingsDraft = {
  shortcut: string;
  shortcutEnabled: boolean;
  showTrayIcon: boolean;
};

export const CATEGORY_ORDER: CategoryKey[] = [
  'all',
  'text',
  'image',
  'file',
  'favorite',
];

export const PAGE_SIZE = 30;

export const EMPTY_TAB_SNAPSHOT: TabSnapshot = {
  items: [],
  offset: 0,
  hasMore: false,
  selectedClipId: null,
  expandedClipIds: [],
  lastLoadedVersion: 0,
};

export function buildTabStateKey(query: string, filter: ClipFilter): string {
  return `${query}::${filter}`;
}

export function formatUpdatedAt(value: string): string {
  const date = new Date(value);
  const diff = Date.now() - date.getTime();
  const minute = 60_000;
  const hour = 60 * minute;

  if (diff < minute) {
    return '刚刚';
  }

  if (diff < hour) {
    return `${Math.max(1, Math.floor(diff / minute))} 分钟前`;
  }

  return new Intl.DateTimeFormat('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  }).format(date);
}

export function summarizeText(text: string): string {
  return text
    .split('\n')
    .map((line) => line.replace(/[ \t]+/g, ' '))
    .join('\n');
}

export function getFileName(filePath: string): string {
  const parts = filePath.split('/');
  return parts[parts.length - 1] || filePath;
}

export function getClipSummary(clip: ClipItem): string {
  if (clip.type === 'image') {
    if (clip.imageWidth && clip.imageHeight) {
      return `${clip.imageWidth} x ${clip.imageHeight}`;
    }
    return '图片';
  }

  if (clip.type === 'file') {
    return clip.filePaths[0] ?? clip.contentText;
  }

  return summarizeText(clip.contentText);
}

export function getClipMeta(clip: ClipItem): string {
  if (clip.type === 'image') {
    return clip.imageWidth && clip.imageHeight
      ? `${clip.imageWidth} x ${clip.imageHeight}`
      : '图片';
  }

  if (clip.type === 'file') {
    return clip.filePaths.length > 1
      ? `${clip.filePaths.length} 个文件`
      : '文件';
  }

  return `${clip.contentText.length} 字符`;
}

export function buildSettingsDraft(windowState: WindowState): SettingsDraft {
  return {
    shortcut: windowState.shortcut,
    shortcutEnabled: windowState.shortcutEnabled,
    showTrayIcon: windowState.showTrayIcon,
  };
}

export function hasSettingsDraftChanges(
  windowState: WindowState,
  draft: SettingsDraft,
): boolean {
  return (
    draft.shortcut.trim() !== windowState.shortcut.trim() ||
    draft.shortcutEnabled !== windowState.shortcutEnabled ||
    draft.showTrayIcon !== windowState.showTrayIcon
  );
}
