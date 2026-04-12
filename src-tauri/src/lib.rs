mod clipboard;
mod commands;
mod events;
mod models;
mod shell;
mod store;
mod window;

use commands::{
    clear_current_clips, copy_clip, get_clip_counts, get_clip_image_preview, get_window_state,
    list_clips_page, paste_clip_and_hide, show_clip_in_finder, toggle_favorite, toggle_pin_window,
    update_window_settings,
};
use events::{TRAY_ABOUT_MENU_ID, TRAY_QUIT_MENU_ID, TRAY_SETTINGS_MENU_ID};
use shell::{configure_shell, emit_open_settings};
use store::ClipboardState;
use tauri::{ActivationPolicy, Manager, WindowEvent};
use window::{
    close_main_window, configure_main_window_overlay, open_about_window, open_main_window,
    should_auto_hide_main_window,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .on_menu_event(|app, event| match event.id().0.as_str() {
            TRAY_ABOUT_MENU_ID => {
                let _ = open_about_window(app);
            }
            TRAY_SETTINGS_MENU_ID => {
                emit_open_settings(app);
                open_main_window(app, true);
            }
            TRAY_QUIT_MENU_ID => {
                app.exit(0);
            }
            _ => {}
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }

            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = window.minimize();
                }
                WindowEvent::Focused(false)
                    if should_auto_hide_main_window(window.app_handle()) =>
                {
                    close_main_window(window.app_handle());
                }
                _ => {}
            }
        })
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(ActivationPolicy::Accessory);

            let state = ClipboardState::load(app.handle())?;
            app.manage(state);
            configure_main_window_overlay(app.handle());
            clipboard::start_clipboard_watcher(app.handle().clone());
            clipboard::start_cleanup_scheduler(app.handle().clone());
            if let Err(error) = configure_shell(app.handle()) {
                eprintln!("[shell] failed to configure shell integrations: {error}");
            }
            if cfg!(debug_assertions) {
                app.handle().plugin(tauri_plugin_mcp_bridge::init())?;
            }
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_clips_page,
            get_clip_image_preview,
            get_clip_counts,
            toggle_favorite,
            copy_clip,
            paste_clip_and_hide,
            clear_current_clips,
            show_clip_in_finder,
            toggle_pin_window,
            get_window_state,
            update_window_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
