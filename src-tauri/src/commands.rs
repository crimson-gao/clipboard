use tauri::{AppHandle, State, WebviewWindow};

use crate::{
    clipboard,
    models::{ClipCounts, ClipImagePreview, ClipItem, PaginatedClips, WindowState},
    shell,
    store::{
        build_counts, current_window_state, matches_filter, matches_query, ClipboardState,
        DEFAULT_PAGE_SIZE,
    },
};

#[tauri::command]
pub fn list_clips_page(
    state: State<'_, ClipboardState>,
    query: Option<String>,
    filter: Option<String>,
    offset: Option<usize>,
    limit: Option<usize>,
) -> Result<PaginatedClips, String> {
    let mut clips = state.with_store(|store| {
        Ok(store
            .clips
            .iter()
            .filter(|clip| matches_query(clip, query.as_deref()))
            .filter(|clip| matches_filter(clip, filter.as_deref()))
            .cloned()
            .collect::<Vec<_>>())
    })?;

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
    clipboard::get_clip_image_preview(&state, id)
}

#[tauri::command]
pub fn get_clip_counts(state: State<'_, ClipboardState>) -> Result<ClipCounts, String> {
    state.with_store(|store| Ok(build_counts(store)))
}

#[tauri::command]
pub fn toggle_favorite(
    app: AppHandle,
    state: State<'_, ClipboardState>,
    id: i64,
) -> Result<Option<ClipItem>, String> {
    let response = state.with_store_mut(|store| {
        let response = if let Some(clip) = store.clips.iter_mut().find(|clip| clip.id == id) {
            clip.is_favorite = !clip.is_favorite;
            Some(clip.clone())
        } else {
            None
        };
        if response.is_some() {
            store.data_version += 1;
        }
        state.save(store).map_err(|error| error.to_string())?;
        Ok(response)
    })?;

    if response.is_some() {
        crate::store::emit_clips_changed(&app);
    }

    Ok(response)
}

#[tauri::command]
pub fn copy_clip(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, ClipboardState>,
    id: i64,
) -> Result<bool, String> {
    let clip =
        state.with_store(|store| Ok(store.clips.iter().find(|clip| clip.id == id).cloned()))?;
    let Some(clip) = clip else {
        return Ok(false);
    };

    shell::copy_selected_clip(&app, &window, &state, &clip)
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
        shell::paste_into_previous_application(&state);
    }
    Ok(copied)
}

#[tauri::command]
pub fn clear_current_clips(
    app: AppHandle,
    state: State<'_, ClipboardState>,
    ids: Vec<i64>,
) -> Result<bool, String> {
    let changed = shell::clear_current_clips_with_confirmation(&app, &state, ids)?;
    if changed {
        crate::store::emit_clips_changed(&app);
    }
    Ok(changed)
}

#[tauri::command]
pub fn show_clip_in_finder(state: State<'_, ClipboardState>, id: i64) -> Result<bool, String> {
    let clip =
        state.with_store(|store| Ok(store.clips.iter().find(|clip| clip.id == id).cloned()))?;
    let Some(clip) = clip else {
        return Ok(false);
    };

    shell::show_clip_in_finder(&clip)
}

#[tauri::command]
pub fn toggle_pin_window(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, ClipboardState>,
) -> Result<WindowState, String> {
    shell::toggle_pin_window(&app, &window, &state)
}

#[tauri::command]
pub fn get_window_state(state: State<'_, ClipboardState>) -> Result<WindowState, String> {
    state.with_store(|store| Ok(current_window_state(store)))
}

#[tauri::command]
pub fn update_window_settings(
    app: AppHandle,
    state: State<'_, ClipboardState>,
    shortcut: String,
    shortcut_enabled: bool,
    show_tray_icon: bool,
) -> Result<WindowState, String> {
    shell::update_window_settings(&app, &state, shortcut, shortcut_enabled, show_tray_icon)
}
