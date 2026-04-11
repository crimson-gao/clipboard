use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
    thread,
    time::Duration,
};

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use clipboard_rs::{Clipboard, ClipboardContext};
use clipboard_watcher::{Body, ClipboardEventListener};
use futures_util::StreamExt;
use html2text::from_read;
use image::{codecs::png::PngEncoder, ColorType, ImageEncoder};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

pub const CLIPS_CHANGED_EVENT: &str = "clips:changed";
const DEFAULT_PAGE_SIZE: usize = 30;
const RETENTION_HOURS: i64 = 24 * 7;
const CLEANUP_INTERVAL_SECS: u64 = 60 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipItem {
    pub id: i64,
    #[serde(rename = "type")]
    pub kind: String,
    pub content_text: String,
    pub is_favorite: bool,
    pub created_at: String,
    pub updated_at: String,
    pub content_path: Option<String>,
    pub file_paths: Vec<String>,
    pub image_width: Option<i32>,
    pub image_height: Option<i32>,
    pub preview_data_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowState {
    pub is_pinned: bool,
    pub shortcut: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipCounts {
    pub text: usize,
    pub image: usize,
    pub file: usize,
    pub favorite: usize,
    pub data_version: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedClips {
    pub items: Vec<ClipItem>,
    pub total: usize,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipImagePreview {
    pub bytes: Vec<u8>,
    pub mime_type: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedStore {
    clips: Vec<ClipItem>,
    next_id: i64,
    #[serde(default = "default_data_version")]
    data_version: i64,
    is_pinned: bool,
    shortcut: Option<String>,
}

pub struct ClipboardState {
    path: PathBuf,
    inner: Mutex<PersistedStore>,
    thumbnail_cache: Mutex<HashMap<String, Vec<u8>>>,
    suppressed_keys: Mutex<HashSet<String>>,
}

impl ClipboardState {
    pub fn load(app: &AppHandle) -> tauri::Result<Self> {
        let app_dir = app.path().app_data_dir()?;
        fs::create_dir_all(&app_dir)?;
        let path = app_dir.join("clipboard-store.json");
        let mut inner = load_store(&path);
        let changed = prune_expired_clips(&mut inner);
        let state = Self {
            path,
            inner: Mutex::new(inner),
            thumbnail_cache: Mutex::new(HashMap::new()),
            suppressed_keys: Mutex::new(HashSet::new()),
        };
        if changed {
            let store = state.inner.lock().map_err(|_| tauri::Error::AssetNotFound("lock failed".into()))?;
            state.save(&store)?;
        }
        Ok(state)
    }

    fn save(&self, store: &PersistedStore) -> tauri::Result<()> {
        let payload = serde_json::to_vec_pretty(store)?;
        fs::write(&self.path, payload)?;
        Ok(())
    }

    fn blobs_dir(&self) -> tauri::Result<PathBuf> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| tauri::Error::AssetNotFound("missing app data dir".into()))?;
        let blobs_dir = parent.join("blobs");
        fs::create_dir_all(&blobs_dir)?;
        Ok(blobs_dir)
    }
}

fn load_store(path: &Path) -> PersistedStore {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<PersistedStore>(&bytes).ok())
        .unwrap_or_else(|| PersistedStore {
            clips: Vec::new(),
            next_id: 1,
            data_version: 1,
            is_pinned: false,
            shortcut: Some("CommandOrControl+Shift+V".to_string()),
        })
}

fn default_data_version() -> i64 {
    1
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn current_window_state(store: &PersistedStore) -> WindowState {
    WindowState {
        is_pinned: store.is_pinned,
        shortcut: store
            .shortcut
            .clone()
            .unwrap_or_else(|| "CommandOrControl+Shift+V".to_string()),
    }
}

fn matches_filter(clip: &ClipItem, filter: Option<&str>) -> bool {
    match filter.unwrap_or("all") {
        "all" => true,
        "favorite" => clip.is_favorite,
        "text" | "image" | "file" => clip.kind == filter.unwrap(),
        _ => true,
    }
}

fn matches_query(clip: &ClipItem, query: Option<&str>) -> bool {
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
        clip.content_path
            .as_deref()
            .unwrap_or("")
            .to_lowercase(),
        joined_file_paths.to_lowercase(),
    ];

    terms.iter().all(|term| {
        haystacks
            .iter()
            .any(|value| value.contains(term.as_str()))
    })
}

fn emit_clips_changed(app: &AppHandle) {
    let _ = app.emit(CLIPS_CHANGED_EVENT, ());
}

fn retention_cutoff() -> DateTime<Utc> {
    Utc::now() - ChronoDuration::hours(RETENTION_HOURS)
}

fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|datetime| datetime.with_timezone(&Utc))
}

fn prune_expired_clips(store: &mut PersistedStore) -> bool {
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

fn cleanup_expired_clips(state: &ClipboardState) -> Result<bool, String> {
    let mut store = state.inner.lock().map_err(|error| error.to_string())?;
    let changed = prune_expired_clips(&mut store);
    if changed {
        store.data_version += 1;
        persist_store(state, &store)?;
    }
    Ok(changed)
}

fn build_counts(store: &PersistedStore) -> ClipCounts {
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

fn suppression_key(kind: &str, content_text: &str, file_paths: &[String]) -> String {
    match kind {
        "file" => format!("file:{}", file_paths.join("\n")),
        "image" => format!("image:{content_text}"),
        _ => format!("text:{content_text}"),
    }
}

fn clip_copy_text(clip: &ClipItem) -> String {
    if clip.kind == "file" {
        clip.file_paths.join("\n")
    } else {
        clip.content_text.clone()
    }
}

fn insert_suppression_key(state: &ClipboardState, key: String) -> Result<(), String> {
    let mut suppressed = state.suppressed_keys.lock().map_err(|error| error.to_string())?;
    suppressed.insert(key);
    Ok(())
}

fn should_suppress(state: &ClipboardState, key: &str) -> Result<bool, String> {
    let mut suppressed = state.suppressed_keys.lock().map_err(|error| error.to_string())?;
    Ok(suppressed.remove(key))
}

fn upsert_clip_item(
    store: &mut PersistedStore,
    kind: &str,
    content_text: String,
    content_path: Option<String>,
    file_paths: Vec<String>,
    image_width: Option<i32>,
    image_height: Option<i32>,
    preview_data_url: Option<String>,
) -> bool {
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
        kind: kind.to_string(),
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

fn persist_store(state: &ClipboardState, store: &PersistedStore) -> Result<(), String> {
    state.save(store).map_err(|error| error.to_string())
}

fn normalize_line_text(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn preserve_plain_text(input: &str) -> String {
    input.replace("\r\n", "\n").replace('\r', "\n")
}

fn is_table_rule(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty()
        && trimmed
            .chars()
            .all(|ch| matches!(ch, '─' | '│' | '┼' | '┬' | '┴' | '├' | '┤' | '┌' | '┐' | '└' | '┘' | ' ' | '\t' | '-'))
}

fn clean_html2text_output(input: &str) -> String {
    let mut lines = Vec::new();

    for raw_line in input.lines() {
        let mut line = raw_line.trim().to_string();
        if line.is_empty() || is_table_rule(&line) {
            continue;
        }

        if line.contains('│') {
            let columns = line
                .split('│')
                .map(normalize_line_text)
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>();
            line = columns.join("\t");
        } else {
            line = normalize_line_text(&line);
        }

        while let Some(stripped) = line.strip_prefix('#') {
            line = stripped.trim_start().to_string();
        }
        if let Some(stripped) = line.strip_prefix('>') {
            line = stripped.trim_start().to_string();
        }
        if line.starts_with('*') && line.ends_with('*') && line.len() > 1 {
            line = line.trim_matches('*').trim().to_string();
        }

        if !line.is_empty() {
            lines.push(line);
        }
    }

    let mut output = String::new();
    let mut previous_was_heading_like = false;
    for line in lines {
        let is_heading_like = line.ends_with(':')
            || line.starts_with('✨')
            || line.starts_with('🚀')
            || line.starts_with("Disclaimer");

        if !output.is_empty() {
            output.push('\n');
            if previous_was_heading_like {
                output.push('\n');
            }
        }
        output.push_str(&line);
        previous_was_heading_like = is_heading_like;
    }

    output
}

fn html_to_plain_text(html: &str) -> String {
    match from_read(html.as_bytes(), usize::MAX) {
        Ok(text) => clean_html2text_output(&text),
        Err(_) => preserve_plain_text(html),
    }
}

fn preferred_clipboard_payload() -> Result<Option<Body>, String> {
    let ctx = ClipboardContext::new().map_err(|error| error.to_string())?;

    if let Ok(files) = ctx.get_files() {
        let file_paths = files
            .into_iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        if !file_paths.is_empty() {
            return Ok(Some(Body::FileList(file_paths)));
        }
    }

    if let Ok(text) = ctx.get_text() {
        let preserved = preserve_plain_text(&text);
        if !preserved.is_empty() {
            return Ok(Some(Body::PlainText(preserved)));
        }
    }

    if let Ok(html) = ctx.get_html() {
        let plain = html_to_plain_text(&html);
        if !plain.is_empty() {
            return Ok(Some(Body::PlainText(plain)));
        }
    }

    Ok(None)
}

fn handle_plain_text(state: &ClipboardState, text: String) -> Result<bool, String> {
    let preserved = preserve_plain_text(&text);
    if preserved.is_empty() {
        return Ok(false);
    }
    let key = suppression_key("text", &preserved, &[]);
    if should_suppress(state, &key)? {
        return Ok(false);
    }

    let mut store = state.inner.lock().map_err(|error| error.to_string())?;
    let changed = upsert_clip_item(
        &mut store,
        "text",
        preserved,
        None,
        Vec::new(),
        None,
        None,
        None,
    );
    if changed {
        persist_store(state, &store)?;
    }
    Ok(changed)
}

fn handle_file_list(state: &ClipboardState, files: Vec<PathBuf>) -> Result<bool, String> {
    let file_paths = files
        .into_iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    if file_paths.is_empty() {
        return Ok(false);
    }

    let summary = file_paths.join("\n");
    let key = suppression_key("file", &summary, &file_paths);
    if should_suppress(state, &key)? {
        return Ok(false);
    }

    let mut store = state.inner.lock().map_err(|error| error.to_string())?;
    let changed = upsert_clip_item(
        &mut store,
        "file",
        summary,
        None,
        file_paths,
        None,
        None,
        None,
    );
    if changed {
        persist_store(state, &store)?;
    }
    Ok(changed)
}

fn save_png_preview(state: &ClipboardState, file_name: &str, bytes: &[u8]) -> Result<String, String> {
    let blobs_dir = state.blobs_dir().map_err(|error| error.to_string())?;
    let image_path = blobs_dir.join(file_name);
    fs::write(&image_path, bytes).map_err(|error| error.to_string())?;
    Ok(image_path.to_string_lossy().to_string())
}

fn should_use_thumbnail(width: i32, height: i32) -> bool {
    width > 900 || height > 900 || width.saturating_mul(height) > 1_200_000
}

fn generate_preview_png_bytes(state: &ClipboardState, content_path: &str) -> Result<Vec<u8>, String> {
    {
        let cache = state.thumbnail_cache.lock().map_err(|error| error.to_string())?;
        if let Some(cached) = cache.get(content_path) {
            return Ok(cached.clone());
        }
    }

    let image = image::open(content_path).map_err(|error| error.to_string())?;
    let preview = if should_use_thumbnail(image.width() as i32, image.height() as i32) {
        image.thumbnail(360, 220)
    } else {
        image
    }
    .to_rgba8();
    let (width, height) = preview.dimensions();
    let mut png_bytes = Vec::new();
    PngEncoder::new(&mut png_bytes)
        .write_image(preview.as_raw(), width, height, ColorType::Rgba8.into())
        .map_err(|error| error.to_string())?;

    let mut cache = state.thumbnail_cache.lock().map_err(|error| error.to_string())?;
    cache.insert(content_path.to_string(), png_bytes.clone());
    Ok(png_bytes)
}

fn handle_png_image(
    state: &ClipboardState,
    path: Option<PathBuf>,
    bytes: &[u8],
) -> Result<bool, String> {
    let content_path = if let Some(path) = path {
        path.to_string_lossy().to_string()
    } else {
        let file_name = format!("clipboard-image-{}.png", Utc::now().timestamp_millis());
        save_png_preview(state, &file_name, bytes)?
    };
    let dimensions = image::load_from_memory(bytes)
        .ok()
        .map(|image| (image.width() as i32, image.height() as i32));
    let (image_width, image_height) = dimensions.unwrap_or((0, 0));
    let summary = format!("{image_width} x {image_height}");
    let key = suppression_key("image", &summary, &[]);
    if should_suppress(state, &key)? {
        return Ok(false);
    }

    let mut store = state.inner.lock().map_err(|error| error.to_string())?;
    let changed = upsert_clip_item(
        &mut store,
        "image",
        summary,
        Some(content_path),
        Vec::new(),
        Some(image_width),
        Some(image_height),
        None,
    );
    if changed {
        persist_store(state, &store)?;
    }
    Ok(changed)
}

fn handle_raw_image(
    state: &ClipboardState,
    width: u32,
    height: u32,
    bytes: &[u8],
    path: Option<PathBuf>,
) -> Result<bool, String> {
    let mut png_bytes = Vec::new();
    PngEncoder::new(&mut png_bytes)
        .write_image(bytes, width, height, ColorType::Rgb8.into())
        .map_err(|error| error.to_string())?;
    handle_png_image(state, path, &png_bytes)
}

pub fn start_clipboard_watcher(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut listener = match ClipboardEventListener::builder().spawn() {
            Ok(listener) => listener,
            Err(error) => {
                eprintln!("[clipboard] failed to start watcher: {error}");
                return;
            }
        };

        let mut stream = listener.new_stream(32);
        while let Some(result) = stream.next().await {
            let Ok(content) = result else {
                continue;
            };

            let state = app.state::<ClipboardState>();
            let preferred = preferred_clipboard_payload().ok().flatten();
            let active_body = preferred.as_ref().unwrap_or(content.as_ref());

            let changed = match active_body {
                Body::PlainText(text) => handle_plain_text(&state, text.clone()),
                Body::FileList(files) => handle_file_list(&state, files.clone()),
                Body::PngImage { path, bytes } => handle_png_image(&state, path.clone(), bytes),
                Body::RawImage(image) => {
                    handle_raw_image(&state, image.width, image.height, &image.bytes, image.path.clone())
                }
                Body::Html(html) => handle_plain_text(&state, html_to_plain_text(html)),
                Body::Custom { .. } => Ok(false),
            };

            match changed {
                Ok(true) => emit_clips_changed(&app),
                Ok(false) => {}
                Err(error) => eprintln!("[clipboard] failed to process event: {error}"),
            }
        }
    });
}

pub fn start_cleanup_scheduler(app: AppHandle) {
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(CLEANUP_INTERVAL_SECS));
            let state = app.state::<ClipboardState>();
            match cleanup_expired_clips(&state) {
                Ok(true) => emit_clips_changed(&app),
                Ok(false) => {}
                Err(error) => eprintln!("[cleanup] failed to prune expired clips: {error}"),
            }
        }
    });
}

#[tauri::command]
pub fn list_clips_page(
    state: State<'_, ClipboardState>,
    query: Option<String>,
    filter: Option<String>,
    offset: Option<usize>,
    limit: Option<usize>,
) -> Result<PaginatedClips, String> {
    let store = state.inner.lock().map_err(|error| error.to_string())?;
    let mut clips = store
        .clips
        .iter()
        .filter(|clip| matches_query(clip, query.as_deref()))
        .filter(|clip| matches_filter(clip, filter.as_deref()))
        .cloned()
        .collect::<Vec<_>>();
    drop(store);

    clips.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    let total = clips.len();
    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(DEFAULT_PAGE_SIZE);
    let end = offset.saturating_add(limit).min(total);
    let has_more = end < total;
    let mut paged_items = if offset >= total {
        Vec::new()
    } else {
        clips[offset..end].to_vec()
    };

    for clip in &mut paged_items {
        if clip.kind == "image" {
            clip.content_path = None;
            clip.preview_data_url = None;
        }
    }

    Ok(PaginatedClips {
        items: paged_items,
        total,
        has_more,
    })
}

#[tauri::command]
pub fn get_clip_image_preview(
    state: State<'_, ClipboardState>,
    id: i64,
) -> Result<Option<ClipImagePreview>, String> {
    let content_path = {
        let store = state.inner.lock().map_err(|error| error.to_string())?;
        let Some(clip) = store.clips.iter().find(|clip| clip.id == id && clip.kind == "image") else {
            return Ok(None);
        };
        clip.content_path.clone()
    };

    let Some(content_path) = content_path else {
        return Ok(None);
    };

    let bytes = generate_preview_png_bytes(&state, &content_path)?;
    Ok(Some(ClipImagePreview {
        bytes,
        mime_type: "image/png".to_string(),
    }))
}

#[tauri::command]
pub fn get_clip_counts(state: State<'_, ClipboardState>) -> Result<ClipCounts, String> {
    let store = state.inner.lock().map_err(|error| error.to_string())?;
    Ok(build_counts(&store))
}

#[tauri::command]
pub fn toggle_favorite(
    app: AppHandle,
    state: State<'_, ClipboardState>,
    id: i64,
) -> Result<Option<ClipItem>, String> {
    let mut store = state.inner.lock().map_err(|error| error.to_string())?;
    let response = if let Some(clip) = store.clips.iter_mut().find(|clip| clip.id == id) {
        clip.is_favorite = !clip.is_favorite;
        Some(clip.clone())
    } else {
        None
    };
    if response.is_some() {
        store.data_version += 1;
    }
    state.save(&store).map_err(|error| error.to_string())?;
    drop(store);
    emit_clips_changed(&app);
    Ok(response)
}

#[tauri::command]
pub fn copy_clip(
    app: AppHandle,
    state: State<'_, ClipboardState>,
    id: i64,
) -> Result<bool, String> {
    let store = state.inner.lock().map_err(|error| error.to_string())?;
    let Some(clip) = store.clips.iter().find(|clip| clip.id == id) else {
        return Ok(false);
    };

    let text = clip_copy_text(clip);
    let key = suppression_key(&clip.kind, &text, &clip.file_paths);
    insert_suppression_key(&state, key)?;

    app.clipboard().write_text(text).map_err(|error| error.to_string())?;
    Ok(true)
}

#[tauri::command]
pub fn paste_clip_and_hide(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, ClipboardState>,
    id: i64,
) -> Result<bool, String> {
    let copied = copy_clip(app, state, id)?;
    if copied {
        let _ = window.hide();
    }
    Ok(copied)
}

#[tauri::command]
pub fn clear_current_clips(
    app: AppHandle,
    state: State<'_, ClipboardState>,
    ids: Vec<i64>,
) -> Result<bool, String> {
    if ids.is_empty() {
        return Ok(false);
    }

    let confirmed = app
        .dialog()
        .message(format!("确认清空当前列表中的 {} 条记录？", ids.len()))
        .title("清空当前列表")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "清空".into(),
            "取消".into(),
        ))
        .blocking_show();

    if !confirmed {
        return Ok(false);
    }

    let id_set = ids.into_iter().collect::<HashSet<_>>();
    let mut store = state.inner.lock().map_err(|error| error.to_string())?;
    let original_len = store.clips.len();
    store.clips.retain(|clip| !id_set.contains(&clip.id));
    let changed = store.clips.len() != original_len;
    if changed {
        store.data_version += 1;
        state.save(&store).map_err(|error| error.to_string())?;
        drop(store);
        emit_clips_changed(&app);
    }
    Ok(changed)
}

#[tauri::command]
pub fn show_clip_in_finder(
    state: State<'_, ClipboardState>,
    id: i64,
) -> Result<bool, String> {
    let store = state.inner.lock().map_err(|error| error.to_string())?;
    let Some(clip) = store.clips.iter().find(|clip| clip.id == id) else {
        return Ok(false);
    };

    let target_path = if clip.kind == "file" {
        clip.file_paths.first().cloned()
    } else {
        clip.content_path.clone()
    };

    let Some(target_path) = target_path else {
        return Ok(false);
    };

    #[cfg(target_os = "macos")]
    {
        let status = Command::new("open")
            .args(["-R", &target_path])
            .status()
            .map_err(|error| error.to_string())?;
        return Ok(status.success());
    }

    #[allow(unreachable_code)]
    Ok(false)
}

#[tauri::command]
pub fn toggle_pin_window(
    window: WebviewWindow,
    state: State<'_, ClipboardState>,
) -> Result<WindowState, String> {
    let mut store = state.inner.lock().map_err(|error| error.to_string())?;
    store.is_pinned = !store.is_pinned;
    window
        .set_always_on_top(store.is_pinned)
        .map_err(|error| error.to_string())?;
    let response = current_window_state(&store);
    state.save(&store).map_err(|error| error.to_string())?;
    Ok(response)
}

#[tauri::command]
pub fn get_window_state(state: State<'_, ClipboardState>) -> Result<WindowState, String> {
    let store = state.inner.lock().map_err(|error| error.to_string())?;
    Ok(current_window_state(&store))
}

pub fn configure_shortcut(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<ClipboardState>();
    let store = state
        .inner
        .lock()
        .map_err(|error| error.to_string())?;
    let shortcut = store
        .shortcut
        .clone()
        .unwrap_or_else(|| "CommandOrControl+Shift+V".to_string());
    drop(store);
    let shortcut = shortcut.parse::<Shortcut>().map_err(|error| error.to_string())?;

    app.global_shortcut().on_shortcut(shortcut, move |app, _, _| {
        if let Some(window) = app.get_webview_window("main") {
            if window.is_visible().unwrap_or(false) {
                let _ = window.hide();
            } else {
                let _ = window.show();
            }
        }
    }).map_err(|error| error.to_string())?;

    Ok(())
}

pub fn seed_debug_data(state: &ClipboardState) -> tauri::Result<()> {
    let mut store = state
        .inner
        .lock()
        .map_err(|_| tauri::Error::AssetNotFound("lock failed".into()))?;

    if !store.clips.is_empty() {
        return Ok(());
    }

    let created_at = now_iso();
    store.clips = vec![
        ClipItem {
            id: 1,
            kind: "text".into(),
            content_text: "Tauri migration in progress".into(),
            is_favorite: false,
            created_at: created_at.clone(),
            updated_at: created_at.clone(),
            content_path: None,
            file_paths: Vec::new(),
            image_width: None,
            image_height: None,
            preview_data_url: None,
        },
        ClipItem {
            id: 2,
            kind: "file".into(),
            content_text: "/Applications/Xcode.app".into(),
            is_favorite: true,
            created_at: created_at.clone(),
            updated_at: created_at.clone(),
            content_path: None,
            file_paths: vec!["/Applications/Xcode.app".into()],
            image_width: None,
            image_height: None,
            preview_data_url: None,
        },
    ];
    store.next_id = 3;
    state.save(&store)
}
