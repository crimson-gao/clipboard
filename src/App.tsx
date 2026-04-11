import { useEffect, useRef, useState } from 'react';

import { clipboardApi } from './lib/clipboard-api';
import type { ClipCounts, ClipFilter, ClipItem, WindowState } from './types';

type CategoryKey = 'all' | 'text' | 'image' | 'file' | 'favorite';

type Category = {
  key: CategoryKey;
  label: string;
  count?: number;
  icon: JSX.Element;
};

const PAGE_SIZE = 30;

type TabSnapshot = {
  items: ClipItem[];
  offset: number;
  hasMore: boolean;
  selectedClipId: number | null;
  expandedClipIds: number[];
  lastLoadedVersion: number;
};

const EMPTY_TAB_SNAPSHOT: TabSnapshot = {
  items: [],
  offset: 0,
  hasMore: false,
  selectedClipId: null,
  expandedClipIds: [],
  lastLoadedVersion: 0,
};

const imagePreviewUrlCache = new Map<string, string>();

function buildTabStateKey(query: string, filter: ClipFilter): string {
  return `${query}::${filter}`;
}

function formatUpdatedAt(value: string): string {
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

function summarizeText(text: string): string {
  return text
    .split('\n')
    .map((line) => line.replace(/[ \t]+/g, ' '))
    .join('\n');
}

function getFileName(filePath: string): string {
  const parts = filePath.split('/');
  return parts[parts.length - 1] || filePath;
}

function buildImagePreviewCacheKey(clip: ClipItem): string {
  return `${clip.id}:${clip.updatedAt}`;
}

function getClipSummary(clip: ClipItem): string {
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

function getClipMeta(clip: ClipItem): string {
  if (clip.type === 'image') {
    return clip.imageWidth && clip.imageHeight
      ? `${clip.imageWidth} x ${clip.imageHeight}`
      : '图片';
  }

  if (clip.type === 'file') {
    return clip.filePaths.length > 1 ? `${clip.filePaths.length} 个文件` : '文件';
  }

  return `${clip.contentText.length} 字符`;
}

function ClipImagePreview({ clip }: { clip: ClipItem }) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [shouldLoad, setShouldLoad] = useState(() =>
    imagePreviewUrlCache.has(buildImagePreviewCacheKey(clip)),
  );
  const [imageUrl, setImageUrl] = useState<string | null>(() =>
    imagePreviewUrlCache.get(buildImagePreviewCacheKey(clip)) ?? null,
  );

  useEffect(() => {
    if (imagePreviewUrlCache.has(buildImagePreviewCacheKey(clip))) {
      setShouldLoad(true);
      return;
    }

    const node = containerRef.current;
    if (!node) {
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        if (!entries[0]?.isIntersecting) {
          return;
        }

        setShouldLoad(true);
        observer.disconnect();
      },
      {
        root: document.querySelector('.history-scroll'),
        rootMargin: '240px 0px',
        threshold: 0.01,
      },
    );

    observer.observe(node);
    return () => {
      observer.disconnect();
    };
  }, [clip.id, clip.updatedAt]);

  useEffect(() => {
    if (!shouldLoad) {
      return;
    }

    const cacheKey = buildImagePreviewCacheKey(clip);
    const cached = imagePreviewUrlCache.get(cacheKey);
    if (cached) {
      setImageUrl(cached);
      return;
    }

    let disposed = false;
    void clipboardApi.getClipImagePreview(clip.id).then((payload) => {
      if (!payload || disposed) {
        return;
      }

      const nextUrl = URL.createObjectURL(
        new Blob([new Uint8Array(payload.bytes)], { type: payload.mimeType }),
      );
      imagePreviewUrlCache.set(cacheKey, nextUrl);
      if (!disposed) {
        setImageUrl(nextUrl);
      }
    });

    return () => {
      disposed = true;
    };
  }, [clip.id, clip.updatedAt, shouldLoad]);

  if (!imageUrl) {
    return <div ref={containerRef} className="image-preview image-preview-placeholder" aria-hidden="true" />;
  }

  return (
    <div ref={containerRef} className="image-preview">
      <img src={imageUrl} alt="" draggable={false} loading="lazy" />
    </div>
  );
}

function App() {
  const queryRef = useRef('');
  const filterRef = useRef<ClipFilter>('all');
  const observerRef = useRef<IntersectionObserver | null>(null);
  const tabStateRef = useRef<Record<string, TabSnapshot>>({});
  const [searchText, setSearchText] = useState('');
  const [query, setQuery] = useState('');
  const [isComposing, setIsComposing] = useState(false);
  const [activeCategory, setActiveCategory] = useState<CategoryKey>('all');
  const [clips, setClips] = useState<ClipItem[]>([]);
  const [counts, setCounts] = useState<ClipCounts>({
    text: 0,
    image: 0,
    file: 0,
    favorite: 0,
    dataVersion: 0,
  });
  const [hasMore, setHasMore] = useState(false);
  const [offset, setOffset] = useState(0);
  const [windowState, setWindowState] = useState<WindowState | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [busyId, setBusyId] = useState<number | null>(null);
  const [selectedClipId, setSelectedClipId] = useState<number | null>(null);
  const [expandedClipIds, setExpandedClipIds] = useState<number[]>([]);

  const filter: ClipFilter =
    activeCategory === 'favorite' ? 'favorite' : activeCategory === 'all' ? 'all' : activeCategory;
  const tabStateKey = buildTabStateKey(query, filter);

  const writeTabSnapshot = (key: string, snapshot: Partial<TabSnapshot>) => {
    const current = tabStateRef.current[key] ?? EMPTY_TAB_SNAPSHOT;
    tabStateRef.current[key] = { ...current, ...snapshot };
  };

  const loadClips = async (
    nextQuery = queryRef.current,
    nextFilter = filterRef.current,
    nextOffset = 0,
    append = false,
    pageSize = PAGE_SIZE,
  ) => {
    const page = await clipboardApi.listClipsPage(nextQuery, nextFilter, nextOffset, pageSize);
    setClips((current) => {
      return append ? [...current, ...page.items] : page.items;
    });
    setHasMore(page.hasMore);
    setOffset(nextOffset + page.items.length);
    setLoading(false);
    setLoadingMore(false);
  };

  const refreshCounts = async () => {
    const nextCounts = await clipboardApi.getClipCounts();
    setCounts(nextCounts);
  };

  const runWithBusyId = async (id: number, action: () => Promise<unknown>) => {
    setBusyId(id);
    try {
      await action();
    } finally {
      setBusyId(null);
    }
  };

  useEffect(() => {
    queryRef.current = query;
    filterRef.current = filter;
  }, [query, filter]);

  useEffect(() => {
    void clipboardApi.getWindowState().then(setWindowState);
    void refreshCounts();
    void loadClips();

    const unsubscribeClips = clipboardApi.subscribeClipsChanged(() => {
      void refreshCounts();
    });

    return () => {
      unsubscribeClips();
    };
  }, []);

  useEffect(() => {
    if (isComposing) {
      return;
    }

    const timer = window.setTimeout(() => {
      setQuery(searchText);
    }, 120);

    return () => {
      window.clearTimeout(timer);
    };
  }, [isComposing, searchText]);

  useEffect(() => {
    setLoading(true);
    const cached = tabStateRef.current[tabStateKey];
    if (cached) {
      if (cached.lastLoadedVersion !== counts.dataVersion) {
        setLoading(true);
        setSelectedClipId(cached.selectedClipId);
        setExpandedClipIds(cached.expandedClipIds);
        void loadClips(query, filter, 0, false, Math.max(cached.offset, PAGE_SIZE));
        return;
      }
      setClips(cached.items);
      setHasMore(cached.hasMore);
      setOffset(cached.offset);
      setSelectedClipId(cached.selectedClipId);
      setExpandedClipIds(cached.expandedClipIds);
      setLoading(false);
      setLoadingMore(false);
      return;
    }

    setExpandedClipIds([]);
    void loadClips(query, filter, 0, false);
  }, [counts.dataVersion, query, filter, tabStateKey]);

  useEffect(() => {
    if (clips.length === 0) {
      setSelectedClipId(null);
      return;
    }

    if (!clips.some((clip) => clip.id === selectedClipId)) {
      setSelectedClipId(clips[0].id);
    }
  }, [clips, selectedClipId]);

  useEffect(() => {
    writeTabSnapshot(tabStateKey, {
      items: clips,
      offset,
      hasMore,
      selectedClipId,
      expandedClipIds,
      lastLoadedVersion: counts.dataVersion,
    });
  }, [clips, counts.dataVersion, expandedClipIds, hasMore, offset, selectedClipId, tabStateKey]);

  const categories: Category[] = [
    {
      key: 'all',
      label: '全部',
      icon: (
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <rect x="6" y="5" width="12" height="15" rx="2" />
          <path d="M9 9h6M9 13h6M15 3v4M9 3v4" />
        </svg>
      ),
    },
    {
      key: 'text',
      label: '文本',
      count: counts.text,
      icon: (
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M6 7h12M12 7v10M9 17h6" />
        </svg>
      ),
    },
    {
      key: 'image',
      label: '图像',
      count: counts.image,
      icon: (
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <rect x="4.5" y="5.5" width="15" height="13" rx="2" />
          <circle cx="10" cy="10" r="1.25" />
          <path d="M7.5 16l3.2-3.3 2.7 2.6 2-2.1 2.3 2.8" />
        </svg>
      ),
    },
    {
      key: 'file',
      label: '文件',
      count: counts.file,
      icon: (
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M8 4.5h6l3 3V18a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2V6.5a2 2 0 0 1 2-2Z" />
          <path d="M14 4.5v4h4" />
        </svg>
      ),
    },
    {
      key: 'favorite',
      label: '收藏',
      count: counts.favorite,
      icon: (
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="m12 4 2.5 5.2 5.7.8-4.1 4 1 5.7L12 17l-5.1 2.7 1-5.7-4.1-4 5.7-.8Z" />
        </svg>
      ),
    },
  ];

  const selectedClip = clips.find((clip) => clip.id === selectedClipId) ?? null;

  const loadMoreClips = async () => {
    if (loading || loadingMore || !hasMore) {
      return;
    }

    setLoadingMore(true);
    await loadClips(queryRef.current, filterRef.current, offset, true);
  };

  const preloadRef = (node: HTMLElement | null) => {
    observerRef.current?.disconnect();

    if (!node || !hasMore || loadingMore || loading) {
      return;
    }

    observerRef.current = new IntersectionObserver(
      (entries) => {
        if (entries[0]?.isIntersecting) {
          void loadMoreClips();
        }
      },
      {
        root: document.querySelector('.history-scroll'),
        rootMargin: '0px 0px 240px 0px',
        threshold: 0.1,
      },
    );

    observerRef.current.observe(node);
  };

  const handleToggleFavorite = async (id: number) => {
    await runWithBusyId(id, () => clipboardApi.toggleFavorite(id));
  };

  const handleCopy = async (id: number) => {
    await runWithBusyId(id, () => clipboardApi.copyClip(id));
  };

  const handlePasteAndHide = async (id: number) => {
    await runWithBusyId(id, () => clipboardApi.pasteClipAndHide(id));
  };

  const handleClearCurrent = async () => {
    if (activeCategory === 'favorite' || clips.length === 0 || busyId !== null) {
      return;
    }

    await runWithBusyId(-1, () => clipboardApi.clearCurrentClips(clips.map((clip) => clip.id)));
  };

  const handleTogglePin = async () => {
    const next = await clipboardApi.togglePinWindow();
    setWindowState(next);
  };

  const selectedFavoriteLabel = selectedClip?.isFavorite ? '取消收藏' : '收藏';
  const clearDisabled = activeCategory === 'favorite' || clips.length === 0 || busyId !== null;

  const toggleExpanded = (id: number) => {
    setExpandedClipIds((current) =>
      current.includes(id) ? current.filter((value) => value !== id) : [...current, id],
    );
  };

  useEffect(() => {
    return () => {
      observerRef.current?.disconnect();
    };
  }, []);

  return (
    <div className="app-shell">
      <div className="top-chrome">
        <header className="topbar">
          <div className="traffic-space" aria-hidden="true" />

          <label className="search-box">
            <span className="brand-badge" aria-hidden="true">
              <svg viewBox="0 0 24 24">
                <path d="M6 5h12a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1Z" />
                <path d="M8 10h8M8 13h6" />
                <path d="M13 8h3v3" />
              </svg>
            </span>
            <input
              type="text"
              value={searchText}
              placeholder="搜索..."
              onChange={(event) => {
                setSearchText(event.target.value);
              }}
              onCompositionStart={() => {
                setIsComposing(true);
              }}
              onCompositionEnd={(event) => {
                setIsComposing(false);
                setSearchText(event.currentTarget.value);
                setQuery(event.currentTarget.value);
              }}
            />
          </label>

          <div className="topbar-actions">
            <button type="button" className="ghost-icon" aria-label="提醒">
              <svg viewBox="0 0 24 24">
                <path d="M12 5a5 5 0 0 0-5 5v2.6L5.8 15A1 1 0 0 0 6.7 16.5h10.6a1 1 0 0 0 .9-1.5L17 12.6V10a5 5 0 0 0-5-5Z" />
                <path d="M9.8 18a2.2 2.2 0 0 0 4.4 0" />
              </svg>
            </button>
            <button type="button" className="ghost-icon" aria-label="菜单">
              <svg viewBox="0 0 24 24">
                <path d="M5 7h14M5 12h14M5 17h14" />
              </svg>
            </button>
            <button
              type="button"
              className={`ghost-icon${windowState?.isPinned ? ' is-active' : ''}`}
              aria-label="固定窗口"
              onClick={() => {
                void handleTogglePin();
              }}
            >
              <svg viewBox="0 0 24 24">
                <path d="m9 5 6 0 0 4 2.8 2.7-4.3.8-.2 6.5H10.7l-.2-6.5-4.3-.8L9 9Z" />
              </svg>
            </button>
          </div>
        </header>

        <nav className="category-strip" aria-label="内容分类">
          {categories.map((item) => (
            <button
              key={item.key}
              type="button"
              className={`category-item${activeCategory === item.key ? ' is-active' : ''}`}
              onClick={() => {
                setActiveCategory(item.key);
              }}
            >
              <span className="category-icon">{item.icon}</span>
              <span>
                {item.label}
                {item.count && item.count > 0 ? ` (${item.count})` : ''}
              </span>
            </button>
          ))}
        </nav>
      </div>

      <main className="content-frame">
        <section className="history-scroll" aria-label="剪切板历史">
          {clips.length === 0 ? (
            <section className="empty-state">
              <p>还没有可显示的记录。</p>
              <span>复制文本、图片或文件后，历史会自动出现在这里。</span>
            </section>
          ) : (
            <div className="entries-list">
              {clips.map((clip, index) => {
                const isSelected = clip.id === selectedClip?.id;
                const summary = getClipSummary(clip);
                const meta = getClipMeta(clip);
                const preloadIndex = Math.max(clips.length - 5, 0);
                const rowRef = index === preloadIndex ? preloadRef : undefined;

                if (clip.type === 'image') {
                  return (
                    <article
                      key={clip.id}
                      ref={rowRef}
                      className={`entry-row entry-image${isSelected ? ' is-selected' : ''}`}
                      onClick={() => {
                        setSelectedClipId(clip.id);
                      }}
                      onDoubleClick={() => {
                        void handlePasteAndHide(clip.id);
                      }}
                    >
                      <div className="image-time">{formatUpdatedAt(clip.updatedAt)}</div>
                      <div className="image-card">
                        <ClipImagePreview clip={clip} />
                        <div className="image-meta">
                          <span>{meta}</span>
                          <div className="image-meta-right">
                            {clip.isFavorite ? (
                              <span className="entry-star entry-star-inline" aria-label="已收藏">
                                <svg viewBox="0 0 24 24" aria-hidden="true">
                                  <path d="m12 4 2.5 5.2 5.7.8-4.1 4 1 5.7L12 17l-5.1 2.7 1-5.7-4.1-4 5.7-.8Z" />
                                </svg>
                              </span>
                            ) : null}
                            <span>{index + 1}</span>
                          </div>
                        </div>
                      </div>
                    </article>
                  );
                }

                if (clip.type === 'file') {
                  const primaryFile = clip.filePaths[0] ?? clip.contentText;
                  return (
                    <article
                      key={clip.id}
                      ref={rowRef}
                      className={`entry-row entry-file${isSelected ? ' is-selected' : ''}`}
                      onClick={() => {
                        setSelectedClipId(clip.id);
                      }}
                      onDoubleClick={() => {
                        void handlePasteAndHide(clip.id);
                      }}
                    >
                      <div className="file-main">
                        <div className="file-icon">
                          <svg viewBox="0 0 24 24" aria-hidden="true">
                            <path d="M8 4.5h6l3 3V18a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2V6.5a2 2 0 0 1 2-2Z" />
                            <path d="M14 4.5v4h4" />
                          </svg>
                        </div>
                        <div className="file-copy">
                          <p className="entry-title">{getFileName(primaryFile)}</p>
                          <p className="entry-time">{formatUpdatedAt(clip.updatedAt)}</p>
                          <div className="file-hover-card">
                            <button
                              type="button"
                              className="file-icon-link"
                              onClick={(event) => {
                                event.stopPropagation();
                                void clipboardApi.showClipInFinder(clip.id);
                              }}
                              aria-label="在 Finder 中显示"
                            >
                              <svg viewBox="0 0 24 24" aria-hidden="true">
                                <path d="M7 7h10v10H7z" />
                                <path d="M10 14 17 7" />
                              </svg>
                            </button>
                            <span className="file-path" title={primaryFile}>{primaryFile}</span>
                            <button
                              type="button"
                              className="file-copy-button"
                              onClick={(event) => {
                                event.stopPropagation();
                                void navigator.clipboard.writeText(primaryFile);
                              }}
                              aria-label="复制路径"
                            >
                              <svg viewBox="0 0 24 24" aria-hidden="true">
                                <path d="M9 7h8v10H9z" />
                                <path d="M7 17H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v1" />
                              </svg>
                            </button>
                          </div>
                        </div>
                      </div>

                      <div className="text-meta">
                        {clip.isFavorite ? (
                          <span className="entry-star entry-star-inline" aria-label="已收藏">
                            <svg viewBox="0 0 24 24" aria-hidden="true">
                              <path d="m12 4 2.5 5.2 5.7.8-4.1 4 1 5.7L12 17l-5.1 2.7 1-5.7-4.1-4 5.7-.8Z" />
                            </svg>
                          </span>
                        ) : null}
                        <span>{meta}</span>
                        <span>{index + 1}</span>
                      </div>
                    </article>
                  );
                }

                return (
                  <article
                    key={clip.id}
                    ref={rowRef}
                    className={`entry-row entry-text${isSelected ? ' is-selected' : ''}`}
                    onClick={() => {
                      setSelectedClipId(clip.id);
                    }}
                    onDoubleClick={() => {
                      void handlePasteAndHide(clip.id);
                    }}
                  >
                    <div className="text-content">
                      <p className={`entry-title entry-title-text${expandedClipIds.includes(clip.id) ? ' is-expanded' : ''}`}>
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
                              toggleExpanded(clip.id);
                            }}
                          >
                            <span className="entry-toggle-icon" aria-hidden="true">
                              {expandedClipIds.includes(clip.id) ? '⌃' : '⌄'}
                            </span>
                            <span>{expandedClipIds.includes(clip.id) ? '收缩' : '展开'}</span>
                          </button>
                        ) : (
                          <span className="entry-toggle entry-toggle-placeholder" aria-hidden="true" />
                        )}
                        <div className="text-meta">
                          {clip.isFavorite ? (
                            <span className="entry-star entry-star-inline" aria-label="已收藏">
                              <svg viewBox="0 0 24 24" aria-hidden="true">
                                <path d="m12 4 2.5 5.2 5.7.8-4.1 4 1 5.7L12 17l-5.1 2.7 1-5.7-4.1-4 5.7-.8Z" />
                              </svg>
                            </span>
                          ) : null}
                          <span>{meta}</span>
                          <span>{index + 1}</span>
                        </div>
                      </div>
                    </div>
                  </article>
                );
              })}
              {loadingMore ? <div className="list-loading-more">加载更多中...</div> : null}
            </div>
          )}
        </section>

        <aside className="floating-toolbar" aria-label="右侧工具栏">
          <button
            type="button"
            className="tool-action"
            disabled={!selectedClip || busyId === selectedClip.id}
            onClick={() => {
              if (selectedClip) {
                void handleCopy(selectedClip.id);
              }
            }}
          >
            <span className="fab fab-primary">
              <svg viewBox="0 0 24 24">
                <path d="M9 7h8v10H9z" />
                <path d="M7 17H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v1" />
              </svg>
            </span>
            <span className="tool-label">复制</span>
          </button>

          <button
            type="button"
            className="tool-action"
            disabled={!selectedClip || busyId === selectedClip.id}
            onClick={() => {
              if (selectedClip) {
                void handlePasteAndHide(selectedClip.id);
              }
            }}
          >
            <span className="fab fab-muted">
              <svg viewBox="0 0 24 24">
                <path d="m9 7 8 5-8 5Z" />
              </svg>
            </span>
            <span className="tool-label">粘贴</span>
          </button>

          <button
            type="button"
            className="tool-action"
            disabled={!selectedClip || busyId === selectedClip.id}
            onClick={() => {
              if (selectedClip) {
                void handleToggleFavorite(selectedClip.id);
              }
            }}
          >
            <span className={`fab fab-favorite${selectedClip?.isFavorite ? ' is-active' : ''}`}>
              <svg viewBox="0 0 24 24">
                <path d="m12 4 2.5 5.2 5.7.8-4.1 4 1 5.7L12 17l-5.1 2.7 1-5.7-4.1-4 5.7-.8Z" />
              </svg>
            </span>
            <span className="tool-label">{selectedFavoriteLabel}</span>
          </button>

          <button
            type="button"
            className="tool-action"
            disabled={clearDisabled}
            onClick={() => {
              void handleClearCurrent();
            }}
          >
            <span className="fab fab-muted">
              <svg viewBox="0 0 24 24">
                <path d="M5 7h14" />
                <path d="M9 7V5.5h6V7" />
                <path d="M8 10v7" />
                <path d="M12 10v7" />
                <path d="M16 10v7" />
                <path d="M7 7l.8 11a2 2 0 0 0 2 1.8h4.4a2 2 0 0 0 2-1.8L17 7" />
              </svg>
            </span>
            <span className="tool-label">清空</span>
          </button>
        </aside>

        <div className="utility-rail" aria-hidden="true">
          <div className="utility-indicator">{loading ? '同步中' : `${clips.length} 条`}</div>
        </div>
      </main>
    </div>
  );
}

export default App;
