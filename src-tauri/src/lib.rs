mod commands;

use commands::{
    configure_shell, clear_current_clips, copy_clip, debug_hide_main_window, debug_show_main_window,
    emit_open_settings, get_clip_counts, get_clip_image_preview, get_overlay_debug_state, get_window_state,
    hide_main_window, init_main_window_overlay, list_clips_page, main_window_should_auto_hide, paste_clip_and_hide,
    seed_debug_data, show_clip_in_finder, show_main_window, start_cleanup_scheduler,
    start_clipboard_watcher, toggle_favorite, toggle_pin_window, update_window_settings,
    ClipboardState,
    TRAY_QUIT_MENU_ID, TRAY_SETTINGS_MENU_ID,
};
use tauri::{ActivationPolicy, Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .on_menu_event(|app, event| match event.id().0.as_str() {
            TRAY_SETTINGS_MENU_ID => {
                emit_open_settings(app);
                show_main_window(app, true);
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
                WindowEvent::Focused(false) => {
                    if main_window_should_auto_hide(&window.app_handle()) {
                        hide_main_window(&window.app_handle());
                    }
                }
                _ => {}
            }
        })
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(ActivationPolicy::Accessory);

            let state = ClipboardState::load(&app.handle())?;
            seed_debug_data(&state)?;
            app.manage(state);
            init_main_window_overlay(&app.handle());
            start_clipboard_watcher(app.handle().clone());
            start_cleanup_scheduler(app.handle().clone());
            if let Err(error) = configure_shell(&app.handle()) {
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
            get_overlay_debug_state,
            debug_show_main_window,
            debug_hide_main_window,
            update_window_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
