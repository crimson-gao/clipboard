import { ClipImagePreview } from './ClipImagePreview';
import {
  formatUpdatedAt,
  getClipMeta,
  getClipSummary,
  getFileName,
} from './clip-utils';
import type { ClipItem } from '../../types';

type TooltipProps = {
  onMouseEnter: React.MouseEventHandler<HTMLElement>;
  onMouseMove: React.MouseEventHandler<HTMLElement>;
  onMouseLeave: React.MouseEventHandler<HTMLElement>;
  onFocus: React.FocusEventHandler<HTMLElement>;
  onBlur: React.FocusEventHandler<HTMLElement>;
};

type ClipListProps = {
  clips: ClipItem[];
  selectedClipId: number | null;
  expandedClipIds: number[];
  hasMore: boolean;
  loadingMore: boolean;
  bindTooltip: (text: string) => TooltipProps;
  onSelect: (id: number) => void;
  onPaste: (id: number) => void;
  onToggleExpanded: (id: number) => void;
  onShowInFinder: (id: number) => void;
  onCopyPath: (path: string) => void;
  setRowRef: (
    id: number,
    preloadRef?: (node: HTMLElement | null) => void,
  ) => (node: HTMLElement | null) => void;
  preloadRef: (node: HTMLElement | null) => void;
};

type ClipRowProps = {
  clip: ClipItem;
  isSelected: boolean;
  rowRef: (node: HTMLElement | null) => void;
  onSelect: (id: number) => void;
  onPaste: (id: number) => void;
  children: React.ReactNode;
};

function FavoriteBadge({
  bindTooltip,
}: {
  bindTooltip: (text: string) => TooltipProps;
}) {
  return (
    <span
      className="entry-star entry-star-inline"
      aria-label="已收藏"
      {...bindTooltip('已收藏')}
    >
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="m12 4 2.5 5.2 5.7.8-4.1 4 1 5.7L12 17l-5.1 2.7 1-5.7-4.1-4 5.7-.8Z" />
      </svg>
    </span>
  );
}

function ClipMetaInfo({
  clip,
  meta,
  index,
  bindTooltip,
}: {
  clip: ClipItem;
  meta: string;
  index: number;
  bindTooltip: (text: string) => TooltipProps;
}) {
  return (
    <div className="text-meta">
      {clip.isFavorite ? <FavoriteBadge bindTooltip={bindTooltip} /> : null}
      <span>{meta}</span>
      <span>{index + 1}</span>
    </div>
  );
}

function ClipRow({
  clip,
  isSelected,
  rowRef,
  onSelect,
  onPaste,
  children,
}: ClipRowProps) {
  const typeClassName =
    clip.type === 'image'
      ? 'entry-image'
      : clip.type === 'file'
        ? 'entry-file'
        : 'entry-text';

  return (
    <article
      ref={rowRef}
      className={`entry-row${isSelected ? ' is-selected' : ''} ${typeClassName}`}
      tabIndex={-1}
      onFocus={() => {
        onSelect(clip.id);
      }}
      onClick={() => {
        onSelect(clip.id);
      }}
      onDoubleClick={() => {
        onPaste(clip.id);
      }}
    >
      {children}
    </article>
  );
}

export function ClipList({
  clips,
  selectedClipId,
  expandedClipIds,
  hasMore,
  loadingMore,
  bindTooltip,
  onSelect,
  onPaste,
  onToggleExpanded,
  onShowInFinder,
  onCopyPath,
  setRowRef,
  preloadRef,
}: ClipListProps) {
  if (clips.length === 0) {
    return (
      <section className="empty-state">
        <p>还没有可显示的记录。</p>
        <span>复制文本、图片或文件后，历史会自动出现在这里。</span>
      </section>
    );
  }

  const expandedClipIdSet = new Set(expandedClipIds);
  const preloadIndex = Math.max(clips.length - 5, 0);

  return (
    <div className="entries-list">
      {clips.map((clip, index) => {
        const isSelected = clip.id === selectedClipId;
        const summary = getClipSummary(clip);
        const meta = getClipMeta(clip);
        const rowRef = index === preloadIndex ? preloadRef : undefined;
        const sharedRowProps = {
          clip,
          isSelected,
          rowRef: setRowRef(clip.id, rowRef),
          onSelect,
          onPaste,
        };

        if (clip.type === 'image') {
          return (
            <ClipRow key={clip.id} {...sharedRowProps}>
              <div className="image-time">
                {formatUpdatedAt(clip.updatedAt)}
              </div>
              <div className="image-card">
                <ClipImagePreview clip={clip} />
                <div className="image-meta">
                  <span>{meta}</span>
                  <div className="image-meta-right">
                    {clip.isFavorite ? (
                      <FavoriteBadge bindTooltip={bindTooltip} />
                    ) : null}
                    <span>{index + 1}</span>
                  </div>
                </div>
              </div>
            </ClipRow>
          );
        }

        if (clip.type === 'file') {
          const primaryFile = clip.filePaths[0] ?? clip.contentText;
          return (
            <ClipRow key={clip.id} {...sharedRowProps}>
              <div className="file-main">
                <div className="file-icon">
                  <svg viewBox="0 0 24 24" aria-hidden="true">
                    <path d="M8 4.5h6l3 3V18a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2V6.5a2 2 0 0 1 2-2Z" />
                    <path d="M14 4.5v4h4" />
                  </svg>
                </div>
                <div className="file-copy">
                  <p className="entry-title">{getFileName(primaryFile)}</p>
                  <div className="file-detail-row">
                    <p className="entry-time">
                      {formatUpdatedAt(clip.updatedAt)}
                    </p>
                    <span className="file-detail-separator" aria-hidden="true">
                      ·
                    </span>
                    <span className="file-path-inline" title={primaryFile}>
                      {primaryFile}
                    </span>
                    <button
                      type="button"
                      className="file-copy-button"
                      onClick={(event) => {
                        event.stopPropagation();
                        onCopyPath(primaryFile);
                      }}
                      aria-label="复制路径"
                      {...bindTooltip('复制路径')}
                    >
                      <svg viewBox="0 0 24 24" aria-hidden="true">
                        <path d="M9 7h8v10H9z" />
                        <path d="M7 17H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v1" />
                      </svg>
                    </button>
                  </div>
                </div>
              </div>

              <div className="file-side">
                <button
                  type="button"
                  className="file-icon-link"
                  onClick={(event) => {
                    event.stopPropagation();
                    onShowInFinder(clip.id);
                  }}
                  aria-label="在 Finder 中显示"
                  {...bindTooltip('在 Finder 中显示')}
                >
                  <svg viewBox="0 0 24 24" aria-hidden="true">
                    <path d="M4.5 8.5h5l1.7 2h9.3v7a2 2 0 0 1-2 2h-12a2 2 0 0 1-2-2v-9a2 2 0 0 1 2-2Z" />
                    <path d="M4.5 10.5h16" />
                  </svg>
                </button>
                <ClipMetaInfo
                  clip={clip}
                  meta={meta}
                  index={index}
                  bindTooltip={bindTooltip}
                />
              </div>
            </ClipRow>
          );
        }

        const isExpanded = expandedClipIdSet.has(clip.id);
        return (
          <ClipRow key={clip.id} {...sharedRowProps}>
            <div className="text-content">
              <p
                className={`entry-title entry-title-text${
                  isExpanded ? ' is-expanded' : ''
                }`}
              >
                {summary}
              </p>
              <div className="text-footer">
                <p className="entry-time">{formatUpdatedAt(clip.updatedAt)}</p>
                {clip.contentText.length > 180 ? (
                  <button
                    type="button"
                    className="entry-toggle"
                    onClick={(event) => {
                      event.stopPropagation();
                      onToggleExpanded(clip.id);
                    }}
                  >
                    <span className="entry-toggle-icon" aria-hidden="true">
                      {isExpanded ? '⌃' : '⌄'}
                    </span>
                    <span>{isExpanded ? '收缩' : '展开'}</span>
                  </button>
                ) : (
                  <span
                    className="entry-toggle entry-toggle-placeholder"
                    aria-hidden="true"
                  />
                )}
                <ClipMetaInfo
                  clip={clip}
                  meta={meta}
                  index={index}
                  bindTooltip={bindTooltip}
                />
              </div>
            </div>
          </ClipRow>
        );
      })}
      {hasMore && loadingMore ? (
        <div className="list-loading-more">加载更多中...</div>
      ) : null}
    </div>
  );
}
