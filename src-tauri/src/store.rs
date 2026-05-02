use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    events::CLIPS_CHANGED_EVENT,
    models::{ClipCounts, ClipItem, WindowState},
};

pub const DEFAULT_PAGE_SIZE: usize = 30;
const RETENTION_HOURS: i64 = 24 * 7;
pub const DEFAULT_SHORTCUT: &str = "CommandOrControl+Shift+S";

pub struct UpsertClipInput {
    pub kind: String,
    pub content_text: String,
    pub content_path: Option<String>,
    pub file_paths: Vec<String>,
    pub image_width: Option<i32>,
    pub image_height: Option<i32>,
    pub preview_data_url: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedStore {
    pub clips: Vec<ClipItem>,
    pub next_id: i64,
    #[serde(default = "default_data_version")]
    pub data_version: i64,
    pub is_pinned: bool,
    pub shortcut: Option<String>,
    #[serde(default = "default_shortcut_enabled")]
    pub shortcut_enabled: bool,
    #[serde(default = "default_show_tray_icon")]
    pub show_tray_icon: bool,
}

pub struct ClipboardState {
    path: PathBuf,
    inner: Mutex<PersistedStore>,
    pub thumbnail_cache: Mutex<HashMap<String, Vec<u8>>>,
    pub last_target_app_pid: Arc<Mutex<Option<i32>>>,
}

impl ClipboardState {
    pub fn load(app: &AppHandle) -> tauri::Result<Self> {
        let app_dir = app.path().app_data_dir()?;
        fs::create_dir_all(&app_dir)?;
        let path = app_dir.join("clipboard-store.json");
        let mut inner = load_store(&path);
        migrate_store(&mut inner);
        let changed = prune_expired_clips(&mut inner);

        let state = Self {
            path,
            inner: Mutex::new(inner),
            thumbnail_cache: Mutex::new(HashMap::new()),
            last_target_app_pid: Arc::new(Mutex::new(None)),
        };

        if changed {
            let store = state
                .inner
                .lock()
                .map_err(|_| tauri::Error::AssetNotFound("lock failed".into()))?;
            state.save(&store)?;
        }

        Ok(state)
    }

    pub fn save(&self, store: &PersistedStore) -> tauri::Result<()> {
        let payload = serde_json::to_vec_pretty(store)?;
        fs::write(&self.path, payload)?;
        Ok(())
    }

    pub fn blobs_dir(&self) -> tauri::Result<PathBuf> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| tauri::Error::AssetNotFound("missing app data dir".into()))?;
        let blobs_dir = parent.join("blobs");
        fs::create_dir_all(&blobs_dir)?;
        Ok(blobs_dir)
    }

    pub fn with_store<T>(
        &self,
        f: impl FnOnce(&PersistedStore) -> Result<T, String>,
    ) -> Result<T, String> {
        let store = self.inner.lock().map_err(|error| error.to_string())?;
        f(&store)
    }

    pub fn with_store_mut<T>(
        &self,
        f: impl FnOnce(&mut PersistedStore) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut store = self.inner.lock().map_err(|error| error.to_string())?;
        f(&mut store)
    }
}

fn default_store() -> PersistedStore {
    PersistedStore {
        clips: Vec::new(),
        next_id: 1,
        data_version: default_data_version(),
        is_pinned: false,
        shortcut: Some(DEFAULT_SHORTCUT.to_string()),
        shortcut_enabled: default_shortcut_enabled(),
        show_tray_icon: default_show_tray_icon(),
    }
}

pub fn load_store(path: &Path) -> PersistedStore {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<PersistedStore>(&bytes).ok())
        .unwrap_or_else(default_store)
}

fn migrate_store(store: &mut PersistedStore) {
    store
        .shortcut
        .get_or_insert_with(|| DEFAULT_SHORTCUT.to_string());
    if store.next_id <= 0 {
        store.next_id = store.clips.iter().map(|clip| clip.id).max().unwrap_or(0) + 1;
    }
}

fn default_data_version() -> i64 {
    1
}

fn default_shortcut_enabled() -> bool {
    true
}

fn default_show_tray_icon() -> bool {
    true
}

pub fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

pub fn current_window_state(store: &PersistedStore) -> WindowState {
    WindowState {
        is_pinned: store.is_pinned,
        shortcut: store
            .shortcut
            .clone()
            .unwrap_or_else(|| DEFAULT_SHORTCUT.to_string()),
        shortcut_enabled: store.shortcut_enabled,
        show_tray_icon: store.show_tray_icon,
    }
}

pub fn matches_filter(clip: &ClipItem, filter: Option<&str>) -> bool {
    match filter.unwrap_or("all") {
        "all" => true,
        "favorite" => clip.is_favorite,
        "text" | "image" | "file" => clip.kind == filter.unwrap_or("all"),
        _ => true,
    }
}

pub fn matches_query(clip: &ClipItem, query: Option<&str>) -> bool {
    let terms = query
        .unwrap_or("")
        .split_whitespace()
        .map(|term| term.to_lowercase())
        .collect::<Vec<_>>();

    if terms.is_empty() {
        return true;
    }

    let joined_file_paths = clip.file_paths.join("\n");
    let haystacks = [
        clip.content_text.to_lowercase(),
        clip.content_path.as_deref().unwrap_or("").to_lowercase(),
        joined_file_paths.to_lowercase(),
    ];

    terms
        .iter()
        .all(|term| haystacks.iter().any(|value| value.contains(term.as_str())))
}

pub fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|datetime| datetime.with_timezone(&Utc))
}

pub fn retention_cutoff() -> DateTime<Utc> {
    Utc::now() - ChronoDuration::hours(RETENTION_HOURS)
}

pub fn prune_expired_clips(store: &mut PersistedStore) -> bool {
    let cutoff = retention_cutoff();
    let original_len = store.clips.len();
    store.clips.retain(|clip| {
        if clip.is_favorite {
            return true;
        }

        parse_timestamp(&clip.updated_at)
            .map(|updated_at| updated_at >= cutoff)
            .unwrap_or(true)
    });
    store.clips.len() != original_len
}

pub fn cleanup_expired_clips(state: &ClipboardState) -> Result<bool, String> {
    state.with_store_mut(|store| {
        let changed = prune_expired_clips(store);
        if changed {
            store.data_version += 1;
            state.save(store).map_err(|error| error.to_string())?;
        }
        Ok(changed)
    })
}

pub fn build_counts(store: &PersistedStore) -> ClipCounts {
    let mut counts = ClipCounts {
        text: 0,
        image: 0,
        file: 0,
        favorite: 0,
        data_version: store.data_version,
    };

    for clip in &store.clips {
        match clip.kind.as_str() {
            "text" => counts.text += 1,
            "image" => counts.image += 1,
            "file" => counts.file += 1,
            _ => {}
        }
        if clip.is_favorite {
            counts.favorite += 1;
        }
    }

    counts
}

pub fn clip_copy_text(clip: &ClipItem) -> String {
    if clip.kind == "file" {
        clip.file_paths.join("\n")
    } else {
        clip.content_text.clone()
    }
}

pub fn upsert_clip_item(store: &mut PersistedStore, input: UpsertClipInput) -> bool {
    let UpsertClipInput {
        kind,
        content_text,
        content_path,
        file_paths,
        image_width,
        image_height,
        preview_data_url,
    } = input;
    let normalized = content_text.replace("\r\n", "\n").replace('\r', "\n");
    if normalized.is_empty() {
        return false;
    }

    let now = now_iso();
    let existing = store.clips.iter_mut().find(|clip| {
        clip.kind == kind
            && clip.content_text == normalized
            && clip.content_path == content_path
            && clip.file_paths == file_paths
    });

    if let Some(clip) = existing {
        clip.updated_at = now;
        clip.image_width = image_width;
        clip.image_height = image_height;
        if preview_data_url.is_some() {
            clip.preview_data_url = preview_data_url;
        }
        store.data_version += 1;
        return true;
    }

    let id = store.next_id;
    store.next_id += 1;
    store.clips.push(ClipItem {
        id,
        kind,
        content_text: normalized,
        is_favorite: false,
        created_at: now.clone(),
        updated_at: now,
        content_path,
        file_paths,
        image_width,
        image_height,
        preview_data_url,
    });
    store.data_version += 1;
    true
}

pub fn emit_clips_changed(app: &AppHandle) {
    let _ = app.emit(CLIPS_CHANGED_EVENT, ());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_clip(id: i64, kind: &str, text: &str, updated_at: &str) -> ClipItem {
        ClipItem {
            id,
            kind: kind.to_string(),
            content_text: text.to_string(),
            is_favorite: false,
            created_at: updated_at.to_string(),
            updated_at: updated_at.to_string(),
            content_path: None,
            file_paths: Vec::new(),
            image_width: None,
            image_height: None,
            preview_data_url: None,
        }
    }

    #[test]
    fn matches_query_checks_text_and_paths() {
        let mut clip = build_clip(1, "file", "notes", "2026-04-01T00:00:00Z");
        clip.content_path = Some("/tmp/clipboard.png".into());
        clip.file_paths = vec!["/Users/demo/notes.txt".into()];

        assert!(matches_query(&clip, Some("notes clipboard")));
        assert!(!matches_query(&clip, Some("missing term")));
    }

    #[test]
    fn prune_expired_keeps_favorites() {
        let mut expired = build_clip(1, "text", "old", "2020-01-01T00:00:00Z");
        expired.is_favorite = true;
        let fresh = build_clip(2, "text", "fresh", &now_iso());
        let stale = build_clip(3, "text", "stale", "2020-01-01T00:00:00Z");
        let mut store = PersistedStore {
            clips: vec![expired, fresh, stale],
            next_id: 4,
            data_version: 1,
            is_pinned: false,
            shortcut: Some(DEFAULT_SHORTCUT.to_string()),
            shortcut_enabled: true,
            show_tray_icon: true,
        };

        let changed = prune_expired_clips(&mut store);

        assert!(changed);
        assert_eq!(store.clips.len(), 2);
        assert!(store.clips.iter().any(|clip| clip.is_favorite));
    }

    #[test]
    fn upsert_clip_item_updates_existing_record() {
        let created_at = now_iso();
        let mut store = PersistedStore {
            clips: vec![build_clip(1, "text", "hello", &created_at)],
            next_id: 2,
            data_version: 1,
            is_pinned: false,
            shortcut: Some(DEFAULT_SHORTCUT.to_string()),
            shortcut_enabled: true,
            show_tray_icon: true,
        };

        let changed = upsert_clip_item(
            &mut store,
            UpsertClipInput {
                kind: "text".into(),
                content_text: "hello".into(),
                content_path: None,
                file_paths: Vec::new(),
                image_width: None,
                image_height: None,
                preview_data_url: None,
            },
        );

        assert!(changed);
        assert_eq!(store.clips.len(), 1);
        assert_eq!(store.next_id, 2);
        assert_eq!(store.data_version, 2);
    }
}
