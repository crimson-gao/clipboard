# Clipboard Architecture

## Overview

Clipboard keeps a single desktop implementation path: `Tauri 2 + React + TypeScript`.

- Rust handles clipboard access, persistence, tray/shortcut integration, and macOS window behavior.
- React handles rendering, user interaction, lightweight view state, and Tauri command invocation.

## Frontend

- `src/App.tsx`: page composition only
- `src/hooks/useClipboardApp.ts`: clipboard list state, pagination, keyboard shortcuts, settings autosave
- `src/features/clips/`: list rendering, image preview, clip formatting helpers
- `src/features/settings/`: settings modal
- `src/components/`: cross-feature UI pieces such as tooltips

## Backend

- `src-tauri/src/commands.rs`: thin Tauri command bridge
- `src-tauri/src/store.rs`: persisted state, filtering, counts, cleanup, migrations
- `src-tauri/src/clipboard.rs`: clipboard watcher, text/html/file/image handling
- `src-tauri/src/window.rs`: window focus, hide/show, pin, overlay behavior
- `src-tauri/src/shell.rs`: tray icon, global shortcut, paste-back integration
- `src-tauri/src/models.rs`: shared response models

## Testing

- `tests/*.test.ts(x)`: frontend unit and component tests with Vitest
- `tests/e2e/*.spec.ts`: browser-level smoke tests with Playwright
- `src-tauri` unit tests: store rules and clipboard text normalization

## Validation Path

Recommended local validation:

```bash
npm run check
npm run test:e2e
cd src-tauri && cargo test && cargo clippy -- -D warnings
```
