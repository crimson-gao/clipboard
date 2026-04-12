use std::{collections::HashSet, process::Command, thread, time::Duration};

#[cfg(target_os = "macos")]
use core_graphics::{
    event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode},
    event_source::{CGEventSource, CGEventSourceStateID},
};
use tauri::{
    menu::MenuBuilder,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::{
    events::{OPEN_SETTINGS_EVENT, TRAY_ABOUT_MENU_ID, TRAY_QUIT_MENU_ID, TRAY_SETTINGS_MENU_ID},
    models::{ClipItem, WindowState},
    store::{clip_copy_text, current_window_state, ClipboardState, DEFAULT_SHORTCUT},
    window::{apply_main_window_overlay, toggle_main_window},
};

const TRAY_ID: &str = "main-tray";

pub fn emit_open_settings(app: &AppHandle) {
    let _ = app.emit(OPEN_SETTINGS_EVENT, ());
}

pub fn ensure_tray_icon(app: &AppHandle, visible: bool) -> Result<(), String> {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_visible(visible)
            .map_err(|error| error.to_string())?;
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
        .text(TRAY_ABOUT_MENU_ID, "关于 Clipboard")
        .separator()
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
    let show_tray_icon = app
        .state::<ClipboardState>()
        .with_store(|store| Ok(store.show_tray_icon))?;
    ensure_tray_icon(app, show_tray_icon)?;
    configure_shortcut(app)
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

pub fn paste_into_previous_application(state: &ClipboardState) {
    #[cfg(target_os = "macos")]
    {
        let target_pid = state
            .last_target_app_pid
            .lock()
            .ok()
            .and_then(|guard| *guard);

        thread::spawn(move || {
            if let Some(pid) = target_pid {
                let _ = crate::window::activate_application(pid);
                thread::sleep(Duration::from_millis(120));
            }
            let _ = post_command_v();
        });
    }
}

pub fn copy_selected_clip(
    app: &AppHandle,
    window: &tauri::WebviewWindow,
    state: &ClipboardState,
    clip: &ClipItem,
) -> Result<bool, String> {
    let text = clip_copy_text(clip);
    let is_pinned = state.with_store(|store| Ok(store.is_pinned))?;

    app.clipboard()
        .write_text(text)
        .map_err(|error| error.to_string())?;
    if !is_pinned {
        let _ = window.hide();
    }
    Ok(true)
}

pub fn clear_current_clips_with_confirmation(
    app: &AppHandle,
    state: &ClipboardState,
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
    state.with_store_mut(|store| {
        let original_len = store.clips.len();
        store.clips.retain(|clip| !id_set.contains(&clip.id));
        let changed = store.clips.len() != original_len;
        if changed {
            store.data_version += 1;
            state.save(store).map_err(|error| error.to_string())?;
        }
        Ok(changed)
    })
}

pub fn show_clip_in_finder(clip: &ClipItem) -> Result<bool, String> {
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

pub fn toggle_pin_window(
    app: &AppHandle,
    window: &tauri::WebviewWindow,
    state: &ClipboardState,
) -> Result<WindowState, String> {
    let response = state.with_store_mut(|store| {
        store.is_pinned = !store.is_pinned;
        let response = current_window_state(store);
        state.save(store).map_err(|error| error.to_string())?;
        Ok(response)
    })?;
    let _ = window.set_visible_on_all_workspaces(true);
    apply_main_window_overlay(app, !response.is_pinned)?;
    Ok(response)
}

fn resolve_shortcut(shortcut: &str) -> String {
    let normalized = shortcut.trim();
    if normalized.is_empty() {
        DEFAULT_SHORTCUT.to_string()
    } else {
        normalized.to_string()
    }
}

pub fn update_window_settings(
    app: &AppHandle,
    state: &ClipboardState,
    shortcut: String,
    shortcut_enabled: bool,
    show_tray_icon: bool,
) -> Result<WindowState, String> {
    let normalized_shortcut = resolve_shortcut(&shortcut);
    if shortcut_enabled {
        normalized_shortcut
            .parse::<Shortcut>()
            .map_err(|error| error.to_string())?;
    }

    state.with_store_mut(|store| {
        store.shortcut = Some(normalized_shortcut.clone());
        store.shortcut_enabled = shortcut_enabled;
        store.show_tray_icon = show_tray_icon;
        state.save(store).map_err(|error| error.to_string())?;
        Ok(())
    })?;

    configure_shortcut(app)?;
    ensure_tray_icon(app, show_tray_icon)?;
    state.with_store(|store| Ok(current_window_state(store)))
}

pub fn configure_shortcut(app: &AppHandle) -> Result<(), String> {
    let (shortcut, shortcut_enabled) = app.state::<ClipboardState>().with_store(|store| {
        Ok((
            store
                .shortcut
                .clone()
                .unwrap_or_else(|| DEFAULT_SHORTCUT.to_string()),
            store.shortcut_enabled,
        ))
    })?;

    app.global_shortcut()
        .unregister_all()
        .map_err(|error| error.to_string())?;
    if !shortcut_enabled {
        return Ok(());
    }

    let shortcut = shortcut
        .parse::<Shortcut>()
        .map_err(|error| error.to_string())?;
    app.global_shortcut()
        .on_shortcut(shortcut, move |app, _, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }
            toggle_main_window(app, true);
        })
        .map_err(|error| error.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::resolve_shortcut;
    use crate::store::DEFAULT_SHORTCUT;

    #[test]
    fn resolve_shortcut_falls_back_to_default_for_blank_input() {
        assert_eq!(resolve_shortcut(""), DEFAULT_SHORTCUT);
        assert_eq!(resolve_shortcut("   "), DEFAULT_SHORTCUT);
    }

    #[test]
    fn resolve_shortcut_trims_non_empty_input() {
        assert_eq!(
            resolve_shortcut("  CommandOrControl+Shift+K  "),
            "CommandOrControl+Shift+K"
        );
    }
}
