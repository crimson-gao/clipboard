use std::sync::{Arc, Mutex};

use tauri::{
    AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSApplication, NSApplicationActivationOptions, NSEvent, NSMainMenuWindowLevel,
    NSRunningApplication, NSScreen, NSWindow, NSWindowCollectionBehavior, NSWorkspace,
};
use objc2_foundation::{NSPoint, NSRect};

use crate::store::ClipboardState;

fn main_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())
}

fn with_main_ns_window<T>(app: &AppHandle, f: impl FnOnce(&NSWindow) -> T) -> Result<T, String> {
    let window = main_window(app)?;
    let ns_window = window.ns_window().map_err(|error| error.to_string())?;
    let ns_window = ns_window.cast::<NSWindow>();
    let ns_window =
        unsafe { ns_window.as_ref() }.ok_or_else(|| "failed to resolve NSWindow".to_string())?;
    Ok(f(ns_window))
}

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

fn order_main_window_front(app: &AppHandle) -> Result<(), String> {
    with_main_ns_window(app, |ns_window| {
        ns_window.makeKeyAndOrderFront(None);
        ns_window.orderFrontRegardless();
    })?;
    Ok(())
}

pub fn configure_main_window_overlay(app: &AppHandle) {
    let _ = apply_main_window_overlay(app, should_auto_hide_main_window(app));
}

pub fn should_auto_hide_main_window(app: &AppHandle) -> bool {
    app.state::<ClipboardState>()
        .with_store(|store| Ok(!store.is_pinned))
        .unwrap_or(false)
}

pub fn close_main_window(app: &AppHandle) {
    if let Ok(window) = main_window(app) {
        let _ = window.hide();
    }
}

fn current_process_pid() -> i32 {
    std::process::id() as i32
}

fn frontmost_application_pid() -> Option<i32> {
    NSWorkspace::sharedWorkspace()
        .frontmostApplication()
        .map(|app| app.processIdentifier())
}

pub fn remember_frontmost_application(state: &ClipboardState) {
    let pid = frontmost_application_pid().filter(|pid| *pid != current_process_pid());
    if let Some(pid) = pid {
        if let Ok(mut guard) = state.last_target_app_pid.lock() {
            *guard = Some(pid);
        }
    }
}

pub fn start_frontmost_app_tracker(app: AppHandle) {
    std::thread::spawn(move || loop {
        let state = app.state::<ClipboardState>();
        remember_frontmost_application(&state);
        std::thread::sleep(std::time::Duration::from_millis(300));
    });
}

pub fn yield_activation_to(app: &AppHandle, pid: i32) {
    let app_handle = app.clone();
    let _ = app_handle.run_on_main_thread(move || {
        let Some(target) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
        else {
            return;
        };
        if let Some(mtm) = MainThreadMarker::new() {
            let ns_app = NSApplication::sharedApplication(mtm);
            ns_app.yieldActivationToApplication(&target);
        }
    });
}

pub fn activate_application(pid: i32) -> Result<bool, String> {
    let Some(target) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) else {
        return Ok(false);
    };

    #[allow(deprecated)]
    let options = NSApplicationActivationOptions::ActivateIgnoringOtherApps;
    Ok(target.activateWithOptions(options))
}

pub fn open_main_window(app: &AppHandle, _remember_target: bool) {
    if let Ok(window) = main_window(app) {
        let auto_hide = should_auto_hide_main_window(app);
        let _ = apply_main_window_overlay(app, auto_hide);
        let _ = window.unminimize();
        let _ = window.show();
        let _ = activate_application(current_process_pid());
        if auto_hide {
            let _ = window.set_focus();
        }
        let _ = order_main_window_front(app);
    }
}

fn rect_contains_point(rect: NSRect, point: NSPoint) -> bool {
    point.x >= rect.origin.x
        && point.x <= rect.origin.x + rect.size.width
        && point.y >= rect.origin.y
        && point.y <= rect.origin.y + rect.size.height
}

fn move_main_window_to_active_screen_native(app: &AppHandle) -> Result<(), String> {
    let result = Arc::new(Mutex::new(None::<Result<(), String>>));
    let result_slot = Arc::clone(&result);
    let app_handle = app.clone();

    app.run_on_main_thread(move || {
        let outcome = (|| -> Result<(), String> {
            let mtm = MainThreadMarker::new()
                .ok_or_else(|| "failed to access AppKit main thread".to_string())?;
            let mouse_location = NSEvent::mouseLocation();
            let screens = NSScreen::screens(mtm);
            let target_screen = screens
                .iter()
                .find(|screen| rect_contains_point(screen.frame(), mouse_location))
                .or_else(|| NSScreen::mainScreen(mtm))
                .ok_or_else(|| "failed to resolve active screen".to_string())?;

            with_main_ns_window(&app_handle, |ns_window| {
                let window_frame = ns_window.frame();
                let visible_frame = target_screen.visibleFrame();

                let centered_x = visible_frame.origin.x
                    + ((visible_frame.size.width - window_frame.size.width).max(0.0) / 2.0);
                let centered_y = visible_frame.origin.y
                    + ((visible_frame.size.height - window_frame.size.height).max(0.0) / 2.0);

                ns_window.setFrameTopLeftPoint(NSPoint::new(
                    centered_x,
                    centered_y + window_frame.size.height,
                ));
            })?;

            Ok(())
        })();

        if let Ok(mut slot) = result_slot.lock() {
            *slot = Some(outcome);
        }
    })
    .map_err(|error| error.to_string())?;

    let outcome = result
        .lock()
        .map_err(|error| error.to_string())?
        .take()
        .unwrap_or_else(|| Err("failed to apply native window positioning".to_string()));
    outcome
}

pub fn move_main_window_to_point(
    app: &AppHandle,
    _point: PhysicalPosition<f64>,
) -> Result<(), String> {
    move_main_window_to_active_screen_native(app)
}

pub fn open_main_window_at_point(
    app: &AppHandle,
    remember_target: bool,
    point: Option<PhysicalPosition<f64>>,
) {
    open_main_window(app, remember_target);
    if let Some(point) = point {
        let _ = move_main_window_to_point(app, point);
        let _ = order_main_window_front(app);
    }
}

pub fn toggle_main_window_at_point(
    app: &AppHandle,
    remember_target: bool,
    point: Option<PhysicalPosition<f64>>,
) {
    if let Ok(window) = main_window(app) {
        if window.is_visible().unwrap_or(false) {
            close_main_window(app);
            return;
        }
    }
    open_main_window_at_point(app, remember_target, point);
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

pub fn open_settings_window(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
        .title("Settings")
        .inner_size(720.0, 680.0)
        .min_inner_size(640.0, 520.0)
        .resizable(true)
        .visible(true)
        .center()
        .build()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub fn request_accessibility_if_needed() {
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;
    }

    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::string::CFString;

    let key = CFString::new("AXTrustedCheckOptionPrompt");
    let value = CFBoolean::true_value();
    let options = CFDictionary::from_CFType_pairs(&[(key.clone(), value)]);

    unsafe {
        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef() as *const _);
    }
}
