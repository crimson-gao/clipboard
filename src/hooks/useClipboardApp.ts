import { useEffect, useRef, useState } from 'react';

import { clipboardApi } from '../lib/clipboard-api';
import type { ClipCounts, ClipFilter, ClipItem, WindowState } from '../types';
import {
  buildTabStateKey,
  CATEGORY_ORDER,
  EMPTY_TAB_SNAPSHOT,
  PAGE_SIZE,
  type CategoryKey,
  type TabSnapshot,
} from '../features/clips/clip-utils';

function isEditableTarget(target: EventTarget | null): boolean {
  const element = target as HTMLElement | null;
  return Boolean(
    element &&
      (element.tagName === 'INPUT' ||
        element.tagName === 'TEXTAREA' ||
        element.isContentEditable),
  );
}

export function useClipboardApp() {
  const queryRef = useRef('');
  const filterRef = useRef<ClipFilter>('all');
  const observerRef = useRef<IntersectionObserver | null>(null);
  const rowRefs = useRef<Record<number, HTMLElement | null>>({});
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
  const filter: ClipFilter = activeCategory;
  const tabStateKey = buildTabStateKey(query, filter);
  const selectedClip = clips.find((clip) => clip.id === selectedClipId) ?? null;

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
    const page = await clipboardApi.listClipsPage(
      nextQuery,
      nextFilter,
      nextOffset,
      pageSize,
    );
    setClips((current) => (append ? [...current, ...page.items] : page.items));
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
        setSelectedClipId(cached.selectedClipId);
        setExpandedClipIds(cached.expandedClipIds);
        void loadClips(
          query,
          filter,
          0,
          false,
          Math.max(cached.offset, PAGE_SIZE),
        );
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
  }, [counts.dataVersion, filter, query, tabStateKey]);

  useEffect(() => {
    if (clips.length === 0) {
      setSelectedClipId(null);
      return;
    }

    if (!clips.some((clip) => clip.id === selectedClipId)) {
      setSelectedClipId(clips[0]?.id ?? null);
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
  }, [
    clips,
    counts.dataVersion,
    expandedClipIds,
    hasMore,
    offset,
    selectedClipId,
    tabStateKey,
  ]);

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
    if (
      activeCategory === 'favorite' ||
      clips.length === 0 ||
      busyId !== null
    ) {
      return;
    }

    await runWithBusyId(-1, () =>
      clipboardApi.clearCurrentClips(clips.map((clip) => clip.id)),
    );
  };

  const handleTogglePin = async () => {
    const next = await clipboardApi.togglePinWindow();
    setWindowState(next);
  };

  const toggleExpanded = (id: number) => {
    setExpandedClipIds((current) =>
      current.includes(id)
        ? current.filter((value) => value !== id)
        : [...current, id],
    );
  };

  const setRowRef = (
    id: number,
    preloadRefCallback?: (node: HTMLElement | null) => void,
  ) => {
    return (node: HTMLElement | null) => {
      rowRefs.current[id] = node;
      preloadRefCallback?.(node);
    };
  };

  useEffect(() => {
    return () => {
      observerRef.current?.disconnect();
    };
  }, []);

  useEffect(() => {
    const selectedNode = selectedClipId
      ? rowRefs.current[selectedClipId]
      : null;
    selectedNode?.scrollIntoView({
      block: 'nearest',
      inline: 'nearest',
    });
  }, [selectedClipId]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (isEditableTarget(event.target)) {
        return;
      }

      if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
        event.preventDefault();
        const currentIndex = CATEGORY_ORDER.indexOf(activeCategory);
        if (currentIndex === -1) {
          return;
        }

        const delta = event.key === 'ArrowRight' ? 1 : -1;
        const nextIndex =
          (currentIndex + delta + CATEGORY_ORDER.length) %
          CATEGORY_ORDER.length;
        setActiveCategory(CATEGORY_ORDER[nextIndex] ?? activeCategory);
        return;
      }

      if (!selectedClip) {
        return;
      }

      if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
        event.preventDefault();
        const currentIndex = clips.findIndex((clip) => clip.id === selectedClip.id);
        if (currentIndex === -1) {
          return;
        }

        const nextIndex =
          event.key === 'ArrowDown'
            ? Math.min(clips.length - 1, currentIndex + 1)
            : Math.max(0, currentIndex - 1);
        setSelectedClipId(clips[nextIndex]?.id ?? selectedClip.id);
        return;
      }

      if (event.key === 'Enter') {
        event.preventDefault();
        void handlePasteAndHide(selectedClip.id);
        return;
      }

      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'c') {
        event.preventDefault();
        void handleCopy(selectedClip.id);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [activeCategory, clips, selectedClip]);

  return {
    activeCategory,
    busyId,
    clips,
    counts,
    expandedClipIds,
    hasMore,
    loading,
    loadingMore,
    preloadRef,
    searchText,
    selectedClip,
    selectedClipId,
    setActiveCategory,
    setIsComposing,
    setRowRef,
    setSearchText,
    setSelectedClipId,
    toggleExpanded,
    windowState,
    handleClearCurrent,
    handleCopy,
    handlePasteAndHide,
    handleToggleFavorite,
    handleTogglePin,
    handleShowInFinder: (id: number) => {
      void clipboardApi.showClipInFinder(id);
    },
  };
}
