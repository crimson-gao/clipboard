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
#[cfg(target_os = "macos")]
use core_graphics::{
    event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode},
    event_source::{CGEventSource, CGEventSourceStateID},
};
use futures_util::StreamExt;
use html2text::from_read;
use image::{codecs::png::PngEncoder, ColorType, ImageEncoder};
#[cfg(target_os = "macos")]
use objc2_app_kit::{
    NSApplicationActivationOptions, NSMainMenuWindowLevel, NSRunningApplication, NSWindow,
    NSWindowCollectionBehavior, NSWorkspace,
};
use serde::{Deserialize, Serialize};
use tauri::{
    menu::MenuBuilder,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State, WebviewWindow,
};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub const CLIPS_CHANGED_EVENT: &str = "clips:changed";
pub const OPEN_SETTINGS_EVENT: &str = "open-settings";
const DEFAULT_PAGE_SIZE: usize = 30;
const RETENTION_HOURS: i64 = 24 * 7;
const CLEANUP_INTERVAL_SECS: u64 = 60 * 60;
const DEFAULT_SHORTCUT: &str = "CommandOrControl+Shift+S";
const LEGACY_DEFAULT_SHORTCUT: &str = "CommandOrControl+Shift+V";
const TRAY_ID: &str = "main-tray";
pub const TRAY_SETTINGS_MENU_ID: &str = "tray-settings";
pub const TRAY_QUIT_MENU_ID: &str = "tray-quit";

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
    pub shortcut_enabled: bool,
    pub show_tray_icon: bool,
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
    #[serde(default = "default_shortcut_enabled")]
    shortcut_enabled: bool,
    #[serde(default = "default_show_tray_icon")]
    show_tray_icon: bool,
}

pub struct ClipboardState {
    path: PathBuf,
    inner: Mutex<PersistedStore>,
    thumbnail_cache: Mutex<HashMap<String, Vec<u8>>>,
    last_target_app_pid: Mutex<Option<i32>>,
}

impl ClipboardState {
    pub fn load(app: &AppHandle) -> tauri::Result<Self> {
        let app_dir = app.path().app_data_dir()?;
        fs::create_dir_all(&app_dir)?;
        let path = app_dir.join("clipboard-store.json");
        let mut inner = load_store(&path);
        if inner.shortcut.is_none() || inner.shortcut.as_deref() == Some(LEGACY_DEFAULT_SHORTCUT) {
            inner.shortcut = Some(DEFAULT_SHORTCUT.to_string());
        }
        let changed = prune_expired_clips(&mut inner);
        let state = Self {
            path,
            inner: Mutex::new(inner),
            thumbnail_cache: Mutex::new(HashMap::new()),
            last_target_app_pid: Mutex::new(None),
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
            shortcut: Some(DEFAULT_SHORTCUT.to_string()),
            shortcut_enabled: true,
            show_tray_icon: true,
        })
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

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn current_window_state(store: &PersistedStore) -> WindowState {
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

pub fn emit_open_settings(app: &AppHandle) {
    let _ = app.emit(OPEN_SETTINGS_EVENT, ());
}

#[cfg(target_os = "macos")]
fn with_main_ns_window<T>(
    app: &AppHandle,
    f: impl FnOnce(&NSWindow) -> T,
) -> Result<T, String> {
    let Some(window) = app.get_webview_window("main") else {
        return Err("main window not found".to_string());
    };
    let ns_window = window.ns_window().map_err(|error| error.to_string())?;
    let ns_window = ns_window.cast::<NSWindow>();
    let ns_window = unsafe { ns_window.as_ref() }
        .ok_or_else(|| "failed to resolve NSWindow".to_string())?;
    Ok(f(ns_window))
}

#[cfg(not(target_os = "macos"))]
fn with_main_ns_window<T>(
    _app: &AppHandle,
    _f: impl FnOnce(&()) -> T,
) -> Result<T, String> {
    Err("macOS only".to_string())
}

#[cfg(target_os = "macos")]
fn apply_main_window_overlay(app: &AppHandle, hides_on_deactivate: bool) -> Result<(), String> {
    with_main_ns_window(app, |ns_window| {
        let collection_behavior =
            ns_window.collectionBehavior()
                | NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary;

        ns_window.setLevel(NSMainMenuWindowLevel + 1);
        ns_window.setCollectionBehavior(collection_behavior);
        ns_window.setHidesOnDeactivate(hides_on_deactivate);
    })?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn apply_main_window_overlay(_app: &AppHandle, _hides_on_deactivate: bool) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "macos")]
fn order_main_window_front(app: &AppHandle) -> Result<(), String> {
    with_main_ns_window(app, |ns_window| {
        ns_window.orderFrontRegardless();
    })?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn order_main_window_front(_app: &AppHandle) -> Result<(), String> {
    Ok(())
}

pub fn configure_main_window_overlay(app: &AppHandle) {
    let hides_on_deactivate = app
        .state::<ClipboardState>()
        .inner
        .lock()
        .map(|store| !store.is_pinned)
        .unwrap_or(true);

    let _ = apply_main_window_overlay(app, hides_on_deactivate);
}

pub fn should_auto_hide_main_window(app: &AppHandle) -> bool {
    app.state::<ClipboardState>()
        .inner
        .lock()
        .map(|store| !store.is_pinned)
        .unwrap_or(false)
}

pub fn close_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

pub fn open_main_window(app: &AppHandle, remember_target: bool) {
    if remember_target {
        let state = app.state::<ClipboardState>();
        let _ = remember_frontmost_application(&state);
    }
    if let Some(window) = app.get_webview_window("main") {
        let is_pinned = app
            .state::<ClipboardState>()
            .inner
            .lock()
            .map(|store| store.is_pinned)
            .unwrap_or(false);
        let _ = apply_main_window_overlay(app, !is_pinned);
        let _ = window.unminimize();
        let _ = window.show();
        if !is_pinned {
            let _ = window.set_focus();
        }
        let _ = order_main_window_front(app);
    }
}

fn toggle_main_window(app: &AppHandle, remember_target: bool) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            close_main_window(app);
        } else {
            open_main_window(app, remember_target);
        }
    }
}

pub fn ensure_tray_icon(app: &AppHandle, visible: bool) -> Result<(), String> {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_visible(visible).map_err(|error| error.to_string())?;
        return Ok(());
    }

    if !visible {
        return Ok(());
    }

    let Some(icon) = app.default_window_icon().cloned() else {
        return Ok(());
    };
    let app_handle = app.clone();
    let menu = MenuBuilder::new(app)
        .text(TRAY_SETTINGS_MENU_ID, "设置")
        .separator()
        .text(TRAY_QUIT_MENU_ID, "退出")
        .build()
        .map_err(|error| error.to_string())?;

    TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .icon(icon)
        .show_menu_on_left_click(false)
        .tooltip("Clipboard")
        .on_tray_icon_event(move |_tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                toggle_main_window(&app_handle, true);
            }
        })
        .build(app)
        .map_err(|error| error.to_string())?;

    Ok(())
}

pub fn configure_shell(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<ClipboardState>();
    let show_tray_icon = state
        .inner
        .lock()
        .map_err(|error| error.to_string())?
        .show_tray_icon;
    ensure_tray_icon(app, show_tray_icon)?;
    configure_shortcut(app)
}

#[cfg(target_os = "macos")]
fn current_process_pid() -> i32 {
    std::process::id() as i32
}

#[cfg(target_os = "macos")]
fn frontmost_application_pid() -> Option<i32> {
    let workspace = NSWorkspace::sharedWorkspace();
    workspace
        .frontmostApplication()
        .map(|application| application.processIdentifier())
}

#[cfg(target_os = "macos")]
fn remember_frontmost_application(state: &ClipboardState) -> Result<(), String> {
    let pid = frontmost_application_pid().filter(|pid| *pid != current_process_pid());
    let mut last_target_app_pid = state
        .last_target_app_pid
        .lock()
        .map_err(|error| error.to_string())?;
    *last_target_app_pid = pid;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn remember_frontmost_application(_state: &ClipboardState) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "macos")]
fn activate_application(pid: i32) -> Result<bool, String> {
    let Some(application) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) else {
        return Ok(false);
    };
    Ok(application.activateWithOptions(NSApplicationActivationOptions::empty()))
}

#[cfg(target_os = "macos")]
fn post_command_v() -> Result<(), String> {
    const COMMAND_KEY_CODE: CGKeyCode = 55;
    const V_KEY_CODE: CGKeyCode = 9;

    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
        .map_err(|_| "failed to create event source".to_string())?;

    let command_down = CGEvent::new_keyboard_event(source.clone(), COMMAND_KEY_CODE, true)
        .map_err(|_| "failed to create command down event".to_string())?;
    let v_down = CGEvent::new_keyboard_event(source.clone(), V_KEY_CODE, true)
        .map_err(|_| "failed to create v down event".to_string())?;
    let v_up = CGEvent::new_keyboard_event(source.clone(), V_KEY_CODE, false)
        .map_err(|_| "failed to create v up event".to_string())?;
    let command_up = CGEvent::new_keyboard_event(source, COMMAND_KEY_CODE, false)
        .map_err(|_| "failed to create command up event".to_string())?;

    v_down.set_flags(CGEventFlags::CGEventFlagCommand);
    v_up.set_flags(CGEventFlags::CGEventFlagCommand);

    command_down.post(CGEventTapLocation::HID);
    v_down.post(CGEventTapLocation::HID);
    v_up.post(CGEventTapLocation::HID);
    command_up.post(CGEventTapLocation::HID);
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn post_command_v() -> Result<(), String> {
    Ok(())
}

fn paste_into_previous_application(state: &ClipboardState) {
    #[cfg(target_os = "macos")]
    {
        let target_pid = state
            .last_target_app_pid
            .lock()
            .ok()
            .and_then(|guard| *guard);

        thread::spawn(move || {
            if let Some(pid) = target_pid {
                let _ = activate_application(pid);
                thread::sleep(Duration::from_millis(120));
            }
            let _ = post_command_v();
        });
    }
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

fn clip_copy_text(clip: &ClipItem) -> String {
    if clip.kind == "file" {
        clip.file_paths.join("\n")
    } else {
        clip.content_text.clone()
    }
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
    window: WebviewWindow,
    state: State<'_, ClipboardState>,
    id: i64,
) -> Result<bool, String> {
    let (text, is_pinned) = {
        let store = state.inner.lock().map_err(|error| error.to_string())?;
        let Some(clip) = store.clips.iter().find(|clip| clip.id == id) else {
            return Ok(false);
        };
        (clip_copy_text(clip), store.is_pinned)
    };

    app.clipboard().write_text(text).map_err(|error| error.to_string())?;
    if !is_pinned {
        let _ = window.hide();
    }
    Ok(true)
}

#[tauri::command]
pub fn paste_clip_and_hide(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, ClipboardState>,
    id: i64,
) -> Result<bool, String> {
    let copied = copy_clip(app, window.clone(), state.clone(), id)?;
    if copied {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        }
        paste_into_previous_application(&state);
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
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, ClipboardState>,
) -> Result<WindowState, String> {
    let mut store = state.inner.lock().map_err(|error| error.to_string())?;
    store.is_pinned = !store.is_pinned;
    let hides_on_deactivate = !store.is_pinned;
    let response = current_window_state(&store);
    state.save(&store).map_err(|error| error.to_string())?;
    drop(store);
    let _ = window.set_visible_on_all_workspaces(true);
    apply_main_window_overlay(&app, hides_on_deactivate)?;
    Ok(response)
}

#[tauri::command]
pub fn get_window_state(state: State<'_, ClipboardState>) -> Result<WindowState, String> {
    let store = state.inner.lock().map_err(|error| error.to_string())?;
    Ok(current_window_state(&store))
}

#[tauri::command]
pub fn update_window_settings(
    app: AppHandle,
    state: State<'_, ClipboardState>,
    shortcut: String,
    shortcut_enabled: bool,
    show_tray_icon: bool,
) -> Result<WindowState, String> {
    let normalized_shortcut = shortcut.trim().to_string();
    if shortcut_enabled {
        if normalized_shortcut.is_empty() {
            return Err("快捷键不能为空".to_string());
        }
        let _ = normalized_shortcut
            .parse::<Shortcut>()
            .map_err(|error| error.to_string())?;
    }

    {
        let mut store = state.inner.lock().map_err(|error| error.to_string())?;
        store.shortcut = Some(if normalized_shortcut.is_empty() {
            DEFAULT_SHORTCUT.to_string()
        } else {
            normalized_shortcut.clone()
        });
        store.shortcut_enabled = shortcut_enabled;
        store.show_tray_icon = show_tray_icon;
        state.save(&store).map_err(|error| error.to_string())?;
    }

    configure_shortcut(&app)?;
    ensure_tray_icon(&app, show_tray_icon)?;

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
        .unwrap_or_else(|| DEFAULT_SHORTCUT.to_string());
    let shortcut_enabled = store.shortcut_enabled;
    drop(store);
    app.global_shortcut().unregister_all().map_err(|error| error.to_string())?;
    if !shortcut_enabled {
        return Ok(());
    }

    let shortcut = shortcut.parse::<Shortcut>().map_err(|error| error.to_string())?;
    app.global_shortcut().on_shortcut(shortcut, move |app, _, event| {
        if event.state != ShortcutState::Pressed {
            return;
        }
        toggle_main_window(app, true);
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
