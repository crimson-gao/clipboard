# Clipboard Product Goals

## 1. Product Positioning

Clipboard is a local clipboard manager built specifically for `macOS Apple Silicon`.

Its value is not “the most features possible,” but a focused desktop tool for frequent copy / paste-back workflows that is:

- fast to summon
- fast to search
- fast to paste back
- behaviorally stable
- local-first

The only supported implementation path is: `Tauri 2 + React + TypeScript`.

---

## 2. Target Users and Core Scenarios

### 2.1 Target Users

- developers, designers, and operators who copy text frequently
- macOS users who constantly move content across apps
- heavy clipboard-history users who need fast recall and re-copy flows

### 2.2 Core Scenarios

1. **Recover something copied moments ago**
2. **Search recent history by keywords**
3. **Re-copy an item and paste it back into the previous foreground app**
4. **Favorite important snippets so cleanup does not remove them**
5. **Inspect image or file entries and reveal files in Finder**

---

## 3. Must-Haves for Phase 1

### 3.1 Functional Goals

- background resident app behavior
- listen for system clipboard changes
- record text, image, and file history
- local persistence for history
- global shortcut to open the main panel
- tray icon and basic tray menu actions
- pinned and non-pinned panel modes
- auto-hide on blur when not pinned
- search, pagination, and favorites
- re-copy history items
- paste selected content back into the previous app
- reveal file items in Finder
- basic settings: shortcut, shortcut toggle, tray visibility toggle

### 3.2 Experience Goals

- window summon behavior should feel responsive
- recent history should refresh quickly
- search results should be stable and predictable
- favorite and cleanup rules should stay simple and obvious
- failures should degrade individual actions instead of destabilizing the whole app

---

## 4. Explicitly Out of Scope for Phase 1

- iCloud or cloud sync
- multi-device sync
- Windows / Linux compatibility
- browser extension support
- account system
- OCR, translation, or AI summaries
- complex workflow automation
- team collaboration features

These are not part of the current product focus.

---

## 5. Technical Goals

### 5.1 Principles to Keep

- one desktop stack only
- keep system integrations in Rust where possible
- let the frontend focus on UI and lightweight state
- minimize duplicated cross-layer state
- prefer debuggability and maintainability over premature completeness

### 5.2 Current Technical Decisions

- `Tauri 2` as the only desktop shell
- `React + TypeScript + Vite` for the frontend
- local JSON as the current persistence layer
- binary image payloads written into the app data directory
- native macOS window capabilities for floating and blur-hide behavior

---

## 6. Current Design Boundaries

### 6.1 Capabilities Already Present

From the current codebase, the project already supports and is structured around:

- clipboard watching and history writes
- 7-day cleanup for non-favorite entries
- paginated history loading and category counts
- text / image / file categorized display
- favorite toggling and clearing the current list
- tray menu and global shortcut entry points
- pinned-window and auto-hide behavior switching
- an About child window
- baseline tests and local validation scripts

### 6.2 Current Accepted Limitations

- file entries are currently closer to “path recording” than full Finder-semantic native paste
- persistence is still JSON and may need to evolve as history volume grows
- main frontend state remains concentrated in `useClipboardApp.ts`
- the command layer is cleaner than before but still needs to be kept from growing too large

---

## 7. Success Criteria

Phase 1 is successful if all of the following are true:

1. users can reliably recover recently copied text, images, and files
2. users can quickly summon the panel and complete re-copy / paste-back flows
3. favorite content is not lost easily
4. pinned and non-pinned window behavior remains consistent
5. local data remains simple, readable, and easy to debug
6. the code structure supports incremental evolution instead of collapsing back into large catch-all files

---

## 8. Conditions for a Later Phase

Only after Phase 1 is stable should the project consider:

- SQLite, once data volume and query cost become a real bottleneck
- native object-level paste semantics for files, once there is a concrete need
- dedicated performance work for image preview or search, once those become real issues
- finer frontend state decomposition, once interaction complexity actually demands it

Principle: **stabilize the local single-machine clipboard experience first, then expand scope.**
