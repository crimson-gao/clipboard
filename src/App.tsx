import type { FocusEvent, MouseEvent } from 'react';
import { useState } from 'react';

import { Tooltip } from './components/Tooltip';
import { buildCategories } from './features/clips/categories';
import { ClipList } from './features/clips/ClipList';
import { useClipboardApp } from './hooks/useClipboardApp';
import { clipboardApi } from './lib/clipboard-api';

type TooltipState = {
  text: string;
  x: number;
  y: number;
};

function App() {
  const [tooltip, setTooltip] = useState<TooltipState | null>(null);
  const {
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
    handleShowInFinder,
    handleToggleFavorite,
    handleTogglePin,
  } = useClipboardApp();
  const categories = buildCategories(counts);
  const selectedFavoriteLabel = selectedClip?.isFavorite ? '取消收藏' : '收藏';
  const clearDisabled =
    activeCategory === 'favorite' || clips.length === 0 || busyId !== null;
  const selectedClipBusy = !selectedClip || busyId === selectedClip.id;
  const copySelectedClip = () => {
    if (!selectedClip) {
      return;
    }

    void handleCopy(selectedClip.id);
  };
  const pasteSelectedClip = () => {
    if (!selectedClip) {
      return;
    }

    void handlePasteAndHide(selectedClip.id);
  };
  const toggleFavoriteForSelectedClip = () => {
    if (!selectedClip) {
      return;
    }

    void handleToggleFavorite(selectedClip.id);
  };
  const writePathToClipboard = (path: string) => {
    void navigator.clipboard.writeText(path);
  };
  const openSettingsWindow = () => {
    void clipboardApi.openSettingsWindow();
  };
  const togglePinnedWindow = () => {
    void handleTogglePin();
  };
  const clearCurrentClips = () => {
    void handleClearCurrent();
  };

  const showTooltip = (text: string, x: number, y: number) => {
    setTooltip({ text, x, y });
  };

  const hideTooltip = () => {
    setTooltip(null);
  };

  const bindTooltip = (text: string) => ({
    onMouseEnter: (event: MouseEvent<HTMLElement>) => {
      showTooltip(text, event.clientX, event.clientY);
    },
    onMouseMove: (event: MouseEvent<HTMLElement>) => {
      showTooltip(text, event.clientX, event.clientY);
    },
    onMouseLeave: () => {
      hideTooltip();
    },
    onFocus: (event: FocusEvent<HTMLElement>) => {
      const rect = event.currentTarget.getBoundingClientRect();
      showTooltip(text, rect.left + rect.width / 2, rect.top);
    },
    onBlur: () => {
      hideTooltip();
    },
  });

  return (
    <div className="app-shell">
      <div className="top-chrome">
        <header className="topbar">
          <div className="topbar-title-wrap">
            <p className="topbar-title">剪切板</p>
          </div>

          <label className="search-box">
            <input
              type="text"
              value={searchText}
              className={searchText ? 'has-value' : ''}
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
              }}
            />
          </label>

          <div className="topbar-actions">
            <div className="topbar-action-item">
              <button
                type="button"
                className="ghost-icon"
                aria-label="设置"
                onClick={openSettingsWindow}
              >
                <svg viewBox="0 0 24 24">
                  <path d="M12 8.2a3.8 3.8 0 1 0 0 7.6 3.8 3.8 0 0 0 0-7.6Z" />
                  <path d="m4.8 13.4 1.3.2a6.7 6.7 0 0 0 .6 1.4l-.8 1.1 1.8 1.8 1.1-.8c.5.3.9.5 1.4.6l.2 1.3h2.6l.2-1.3c.5-.1 1-.3 1.4-.6l1.1.8 1.8-1.8-.8-1.1c.3-.5.5-.9.6-1.4l1.3-.2v-2.6l-1.3-.2a6.7 6.7 0 0 0-.6-1.4l.8-1.1-1.8-1.8-1.1.8a6.7 6.7 0 0 0-1.4-.6L13.4 4h-2.6l-.2 1.3c-.5.1-1 .3-1.4.6l-1.1-.8-1.8 1.8.8 1.1c-.3.5-.5.9-.6 1.4l-1.3.2Z" />
                </svg>
              </button>
              <span className="topbar-action-label">设置</span>
            </div>
            <div className="topbar-action-item">
              <button
                type="button"
                className={`ghost-icon${windowState?.isPinned ? ' is-active' : ''}`}
                aria-label="固定窗口"
                onClick={togglePinnedWindow}
              >
                <svg viewBox="0 0 24 24">
                  <path d="m9 5 6 0 0 4 2.8 2.7-4.3.8-.2 6.5H10.7l-.2-6.5-4.3-.8L9 9Z" />
                </svg>
              </button>
              <span className="topbar-action-label">固定</span>
            </div>
          </div>
        </header>

        <nav className="category-strip" aria-label="内容分类">
          {categories.map((item) => (
            <button
              key={item.key}
              type="button"
              className={`category-item${
                activeCategory === item.key ? ' is-active' : ''
              }`}
              onClick={() => {
                setActiveCategory(item.key);
              }}
            >
              <span className="category-icon">
                {item.icon(activeCategory === item.key)}
              </span>
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
          <ClipList
            clips={clips}
            selectedClipId={selectedClipId}
            expandedClipIds={expandedClipIds}
            hasMore={hasMore}
            loadingMore={loadingMore}
            bindTooltip={bindTooltip}
            onSelect={setSelectedClipId}
            onPaste={(id) => {
              void handlePasteAndHide(id);
            }}
            onToggleExpanded={toggleExpanded}
            onShowInFinder={handleShowInFinder}
            onCopyPath={writePathToClipboard}
            setRowRef={setRowRef}
            preloadRef={preloadRef}
          />
        </section>

        <aside className="right-rail" aria-label="右侧工具栏">
          <div className="floating-toolbar">
            <button
              type="button"
              className="tool-action"
              disabled={selectedClipBusy}
              onClick={copySelectedClip}
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
              disabled={selectedClipBusy}
              onClick={pasteSelectedClip}
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
              disabled={selectedClipBusy}
              onClick={toggleFavoriteForSelectedClip}
            >
              <span
                className={`fab fab-favorite${
                  selectedClip?.isFavorite ? ' is-active' : ''
                }`}
              >
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
              onClick={clearCurrentClips}
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
          </div>

          <div className="utility-rail" aria-hidden="true">
            <div className="utility-indicator">
              {loading ? '同步中' : `${clips.length} 条`}
            </div>
          </div>
        </aside>
      </main>

      {tooltip ? <Tooltip {...tooltip} /> : null}
    </div>
  );
}

export default App;
