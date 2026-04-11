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
  previewDataUrl: string | null;
};

export type WindowState = {
  isPinned: boolean;
  shortcut: string;
};
