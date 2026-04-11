mod commands;

use commands::{
    clear_current_clips, configure_shortcut, copy_clip, get_clip_counts, get_clip_image_preview,
    get_window_state, list_clips_page, paste_clip_and_hide, seed_debug_data, show_clip_in_finder,
    start_cleanup_scheduler, start_clipboard_watcher, toggle_favorite, toggle_pin_window,
    ClipboardState,
};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let state = ClipboardState::load(&app.handle())?;
            seed_debug_data(&state)?;
            app.manage(state);
            start_clipboard_watcher(app.handle().clone());
            start_cleanup_scheduler(app.handle().clone());
            if let Err(error) = configure_shortcut(&app.handle()) {
                eprintln!("[shortcut] failed to configure shortcut: {error}");
            }
            if cfg!(debug_assertions) {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    window.open_devtools();
                }
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
