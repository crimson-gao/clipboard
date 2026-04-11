export type ClipKind = 'text' | 'image' | 'file';

export type ClipFilter = 'all' | 'favorite' | ClipKind;

export type ClipItem = {
  id: number;
  type: ClipKind;
  contentText: string;
  isFavorite: boolean;
  createdAt: string;
  updatedAt: string;
  contentPath: string | null;
  filePaths: string[];
  imageWidth: number | null;
  imageHeight: number | null;
};

export type ClipImagePreview = {
  bytes: number[];
  mimeType: string;
};

export type WindowState = {
  isPinned: boolean;
  shortcut: string;
};

export type ClipCounts = {
  text: number;
  image: number;
  file: number;
  favorite: number;
  dataVersion: number;
};

export type PaginatedClips = {
  items: ClipItem[];
  total: number;
  hasMore: boolean;
};
