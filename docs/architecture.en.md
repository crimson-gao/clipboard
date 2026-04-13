# Clipboard Architecture

## 1. Purpose

This document describes the current implementation path, module boundaries, key data flows, and the design constraints that future changes should continue to respect.

The only supported desktop implementation path is: `Tauri 2 + React + TypeScript`.

---

## 2. Architecture Overview

The app is split into two layers:

- **Rust / Tauri backend**: system integration, clipboard watching, persistence, window control, tray integration, and global shortcuts.
- **React frontend**: UI rendering, interaction state, list pagination/filter/search, and Tauri command calls.

Core principles:

1. **Keep system capabilities in Rust**: clipboard, window behavior, tray, shortcuts, and Finder integration stay out of the frontend.
2. **Keep page state in React**: filters, pagination, selection, expanded rows, and settings drafts should not move into Rust.
3. **Keep commands thin**: `commands.rs` should adapt inputs and compose capabilities, not absorb complex business logic.
4. **Prefer a single desktop path**: Electron and other shell alternatives are no longer maintained.

---

## 3. Current Code Structure

### 3.1 Frontend

- `src/App.tsx`
  - Page composition
  - Top toolbar, category switching, settings entry, pin-window entry
- `src/hooks/useClipboardApp.ts`
  - Main UI state hub
  - Data loading, pagination, search, list selection, settings modal state
  - Subscribes to `clips:changed` and `open-settings`
- `src/lib/clipboard-api.ts`
  - Single frontend Tauri access layer
  - Wraps `invoke` calls and event subscriptions
- `src/features/clips/`
  - `ClipList.tsx`: list and item interaction
  - `ClipImagePreview.tsx`: image preview
  - `clip-utils.ts`: categories, summaries, display formatting, pagination constants
- `src/features/settings/`
  - `SettingsModal.tsx`: settings panel
  - `ShortcutRecorder.tsx`: shortcut recorder
- `src/components/Tooltip.tsx`
  - Shared tooltip UI
- `src/about.tsx`
  - About window page

### 3.2 Backend

- `src-tauri/src/lib.rs`
  - Tauri application composition entry
  - Registers plugins, tray menu, window events, and command handlers
- `src-tauri/src/commands.rs`
  - Frontend-facing command entry points
  - List queries, favorite toggle, copy, paste-back, clear current list, window settings, etc.
- `src-tauri/src/clipboard.rs`
  - System clipboard watcher
  - Text / HTML / file / image parsing and normalization
  - Periodic cleanup scheduler
- `src-tauri/src/store.rs`
  - JSON persistence
  - Model writes, deduplicating upsert, filtering, search, counts, expiration cleanup
- `src-tauri/src/shell.rs`
  - Tray, global shortcuts, confirmation dialogs, clipboard write-back, paste-back integration
- `src-tauri/src/window.rs`
  - macOS floating window behavior, auto-hide, app activation switching, About window
- `src-tauri/src/events.rs`
  - Shared frontend/backend event names and tray menu identifiers
- `src-tauri/src/models.rs`
  - Shared command response models

---

## 4. Key Runtime Flows

### 4.1 App Startup

1. `lib.rs` initializes the Tauri builder.
2. Plugins are registered:
   - `tauri-plugin-clipboard-manager`
   - `tauri-plugin-global-shortcut`
   - `tauri-plugin-dialog`
3. `ClipboardState::load()`:
   - loads `clipboard-store.json` from the app data directory
   - performs migrations and expired-data cleanup
4. Main-window overlay behavior is configured.
5. Clipboard watcher and cleanup scheduler are started.
6. Tray and global shortcuts are configured.

### 4.2 Clipboard Capture

1. `clipboard.rs` listens for system clipboard changes.
2. Payload priority is:
   - file list
   - plain text
   - HTML converted to plain text
   - PNG image
3. The payload is converted into `UpsertClipInput`.
4. `store.rs` runs `upsert_clip_item()`:
   - deduplicates by type + content + path data
   - updates timestamps if the item already exists
   - inserts a new item otherwise
5. A `clips:changed` event is emitted after data changes.

### 4.3 Frontend Refresh

1. `useClipboardApp()` subscribes to `clips:changed`.
2. When triggered, counts are refreshed.
3. If the current query/filter tab cache is stale, paginated data is reloaded.
4. Each tab preserves:
   - loaded items
   - offset
   - hasMore
   - selectedClipId
   - expandedClipIds

### 4.4 Opening the Main Window

The main window can be opened from:

- the global shortcut
- tray icon click / tray menu actions

When paste-back behavior is needed, the app records the current foreground application PID before showing the main window.

### 4.5 Copy and Paste-back

#### Copy a history item

1. The frontend calls `copy_clip`.
2. Rust writes the selected content back to the system clipboard.
3. If the window is not pinned, it is hidden automatically.

#### Paste back into the previous app

1. The frontend calls `paste_clip_and_hide`.
2. Rust first runs `copy_clip`.
3. The current window is hidden.
4. The previously remembered foreground app is activated.
5. A native macOS `Command + V` event is posted.

> In the current implementation, file entries are copied back as path text, not as native Finder file-url / file-promise payloads.

---

## 5. Data Design

### 5.1 Storage

Persistence currently uses local JSON:

- main store file: `clipboard-store.json`
- image files: stored under `blobs/` in the app data directory

Why JSON right now:

- simple implementation
- easy to debug
- fast enough for current product validation

SQLite should only be considered when:

- history volume makes list queries noticeably slow
- search or pagination becomes a real bottleneck
- stronger consistency or indexing is required

### 5.2 Record Model

Each clipboard item contains at least:

- `id`
- `kind`: `text | image | file`
- `content_text`
- `content_path`
- `file_paths`
- `is_favorite`
- `created_at`
- `updated_at`
- `image_width` / `image_height`

### 5.3 Lifecycle Rules

- Non-favorite records are retained for the most recent **7 days** by default
- Favorite records are not deleted by expiration cleanup
- Repeated identical content updates timestamps instead of inserting duplicates
- `data_version` drives frontend cache invalidation

---

## 6. Frontend / Backend Boundary

### 6.1 Tauri Commands

The frontend currently uses `src/lib/clipboard-api.ts` for:

- `list_clips_page`
- `get_clip_image_preview`
- `get_clip_counts`
- `toggle_favorite`
- `copy_clip`
- `paste_clip_and_hide`
- `clear_current_clips`
- `show_clip_in_finder`
- `toggle_pin_window`
- `get_window_state`
- `update_window_settings`

Constraints:

- Do not scatter raw `invoke()` calls throughout components
- Add new commands to `clipboard-api.ts` first
- Reuse shared response shapes from `models.rs` / `src/types.ts`

### 6.2 Events

Only a small set of cross-layer events should exist:

- `clips:changed`
- `open-settings`

Constraints:

- Events should notify, not transport complex business objects
- Complex data should always be pulled through commands to avoid state drift

---

## 7. UI Design Constraints

The current main experience is a floating panel centered around search, categories, history items, and item-level actions.

It should continue to preserve:

- **Fast summon**: visible and focusable quickly after the shortcut
- **Few steps**: copy / favorite / paste-back / reveal in Finder should stay one-step actions
- **High information density**: text, image, and file items should remain readable in one list
- **Predictable window behavior**: pinned vs. non-pinned behavior must stay consistent

Avoid introducing:

- heavy global state frameworks
- large UI component libraries
- duplicated cross-layer state that needs constant syncing

---

## 8. Current Trade-offs

### 8.1 Accepted Trade-offs

1. **JSON over database**
   - fits the current stage
   - later query flexibility is limited

2. **Frontend tab-level cache for paginated results**
   - reduces repeated loads when switching categories and searches
   - depends on `dataVersion` for invalidation

3. **File entries are copied back as path text first**
   - keeps them visible, searchable, and revealable
   - native Finder-semantic paste is not implemented yet

4. **Window behavior stays close to native macOS**
   - floating and hide behavior use `NSWindow` capabilities
   - platform-specific logic stays concentrated in `window.rs`

### 8.2 Risks to Keep Contained

- `commands.rs` growing too large again
- `useClipboardApp.ts` becoming overloaded with orchestration logic
- naming drift between commands and frontend types
- documentation falling behind implementation

---

## 9. Recommended Next Steps

### 9.1 High Priority

1. Keep the command layer thin
2. Add more rule-focused tests in the store layer
3. Make the difference between “copy path text” and “native file paste” explicit
4. Keep tightening settings, list, and detail interaction boundaries

### 9.2 Only If Needed

- SQLite persistence
- stronger thumbnail caching
- richer image / file metadata
- more windows or quick-action panels

---

## 10. Architecture Summary

- **Desktop runtime**: `Tauri 2` only
- **State boundary**: system state in Rust, page state in React
- **Persistence**: JSON + blobs for now
- **Window model**: floating main window + optional pin + About child window
- **Data flow**: watcher writes store, frontend reacts to events and fetches data
- **Evolution principle**: prefer simple, reliable, and debuggable before expanding scope
