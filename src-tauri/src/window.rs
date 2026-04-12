use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

#[cfg(target_os = "macos")]
use objc2_app_kit::{
    NSApplicationActivationOptions, NSMainMenuWindowLevel, NSRunningApplication, NSWindow,
    NSWindowCollectionBehavior, NSWorkspace,
};

use crate::store::ClipboardState;

#[cfg(target_os = "macos")]
fn with_main_ns_window<T>(app: &AppHandle, f: impl FnOnce(&NSWindow) -> T) -> Result<T, String> {
    let Some(window) = app.get_webview_window("main") else {
        return Err("main window not found".to_string());
    };
    let ns_window = window.ns_window().map_err(|error| error.to_string())?;
    let ns_window = ns_window.cast::<NSWindow>();
    let ns_window =
        unsafe { ns_window.as_ref() }.ok_or_else(|| "failed to resolve NSWindow".to_string())?;
    Ok(f(ns_window))
}

#[cfg(not(target_os = "macos"))]
fn with_main_ns_window<T>(_app: &AppHandle, _f: impl FnOnce(&()) -> T) -> Result<T, String> {
    Err("macOS only".to_string())
}

#[cfg(target_os = "macos")]
pub fn apply_main_window_overlay(app: &AppHandle, hides_on_deactivate: bool) -> Result<(), String> {
    with_main_ns_window(app, |ns_window| {
        let collection_behavior = ns_window.collectionBehavior()
            | NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary;

        ns_window.setLevel(NSMainMenuWindowLevel + 1);
        ns_window.setCollectionBehavior(collection_behavior);
        ns_window.setAcceptsMouseMovedEvents(true);
        ns_window.setHidesOnDeactivate(hides_on_deactivate);
    })?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn apply_main_window_overlay(
    _app: &AppHandle,
    _hides_on_deactivate: bool,
) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "macos")]
fn order_main_window_front(app: &AppHandle) -> Result<(), String> {
    with_main_ns_window(app, |ns_window| {
        ns_window.makeKeyAndOrderFront(None);
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
        .with_store(|store| Ok(!store.is_pinned))
        .unwrap_or(true);

    let _ = apply_main_window_overlay(app, hides_on_deactivate);
}

pub fn should_auto_hide_main_window(app: &AppHandle) -> bool {
    app.state::<ClipboardState>()
        .with_store(|store| Ok(!store.is_pinned))
        .unwrap_or(false)
}

pub fn close_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
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
pub fn remember_frontmost_application(state: &ClipboardState) -> Result<(), String> {
    let pid = frontmost_application_pid().filter(|pid| *pid != current_process_pid());
    let mut last_target_app_pid = state
        .last_target_app_pid
        .lock()
        .map_err(|error| error.to_string())?;
    *last_target_app_pid = pid;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn remember_frontmost_application(_state: &ClipboardState) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn activate_application(pid: i32) -> Result<bool, String> {
    let Some(application) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
    else {
        return Ok(false);
    };
    Ok(application.activateWithOptions(NSApplicationActivationOptions::empty()))
}

#[cfg(not(target_os = "macos"))]
pub fn activate_application(_pid: i32) -> Result<bool, String> {
    Ok(false)
}

pub fn open_main_window(app: &AppHandle, remember_target: bool) {
    if remember_target {
        let state = app.state::<ClipboardState>();
        let _ = remember_frontmost_application(&state);
    }
    if let Some(window) = app.get_webview_window("main") {
        let is_pinned = app
            .state::<ClipboardState>()
            .with_store(|store| Ok(store.is_pinned))
            .unwrap_or(false);
        let _ = apply_main_window_overlay(app, !is_pinned);
        let _ = window.unminimize();
        let _ = window.show();
        #[cfg(target_os = "macos")]
        let _ = activate_application(current_process_pid());
        if !is_pinned {
            let _ = window.set_focus();
        }
        let _ = order_main_window_front(app);
    }
}

pub fn toggle_main_window(app: &AppHandle, remember_target: bool) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            close_main_window(app);
        } else {
            open_main_window(app, remember_target);
        }
    }
}

pub fn open_about_window(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("about") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    WebviewWindowBuilder::new(app, "about", WebviewUrl::App("about.html".into()))
        .title("About Clipboard")
        .inner_size(620.0, 520.0)
        .min_inner_size(560.0, 460.0)
        .resizable(true)
        .maximizable(false)
        .visible(true)
        .center()
        .build()
        .map(|_| ())
        .map_err(|error| error.to_string())
}
