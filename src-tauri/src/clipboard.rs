use std::{fs, path::PathBuf, thread, time::Duration};

use clipboard_rs::{Clipboard, ClipboardContext};
use clipboard_watcher::{Body, ClipboardEventListener};
use futures_util::StreamExt;
use html2text::from_read;
use image::{codecs::png::PngEncoder, ColorType, ImageEncoder};
use tauri::{AppHandle, Manager};

use crate::{
    models::ClipImagePreview,
    store::{
        cleanup_expired_clips, emit_clips_changed, upsert_clip_item, ClipboardState,
        UpsertClipInput,
    },
};

const CLEANUP_INTERVAL_SECS: u64 = 60 * 60;

fn normalize_line_text(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn preserve_plain_text(input: &str) -> String {
    input.replace("\r\n", "\n").replace('\r', "\n")
}

fn is_table_rule(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty()
        && trimmed.chars().all(|ch| {
            matches!(
                ch,
                '─' | '│'
                    | '┼'
                    | '┬'
                    | '┴'
                    | '├'
                    | '┤'
                    | '┌'
                    | '┐'
                    | '└'
                    | '┘'
                    | ' '
                    | '\t'
                    | '-'
            )
        })
}

pub fn clean_html2text_output(input: &str) -> String {
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
        let file_paths = files.into_iter().map(PathBuf::from).collect::<Vec<_>>();
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

    state.with_store_mut(|store| {
        let changed = upsert_clip_item(
            store,
            UpsertClipInput {
                kind: "text".into(),
                content_text: preserved,
                content_path: None,
                file_paths: Vec::new(),
                image_width: None,
                image_height: None,
                preview_data_url: None,
            },
        );
        if changed {
            state.save(store).map_err(|error| error.to_string())?;
        }
        Ok(changed)
    })
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

    state.with_store_mut(|store| {
        let changed = upsert_clip_item(
            store,
            UpsertClipInput {
                kind: "file".into(),
                content_text: summary,
                content_path: None,
                file_paths,
                image_width: None,
                image_height: None,
                preview_data_url: None,
            },
        );
        if changed {
            state.save(store).map_err(|error| error.to_string())?;
        }
        Ok(changed)
    })
}

fn save_png_preview(
    state: &ClipboardState,
    file_name: &str,
    bytes: &[u8],
) -> Result<String, String> {
    let blobs_dir = state.blobs_dir().map_err(|error| error.to_string())?;
    let image_path = blobs_dir.join(file_name);
    fs::write(&image_path, bytes).map_err(|error| error.to_string())?;
    Ok(image_path.to_string_lossy().to_string())
}

fn should_use_thumbnail(width: i32, height: i32) -> bool {
    width > 900 || height > 900 || width.saturating_mul(height) > 1_200_000
}

fn generate_preview_png_bytes(
    state: &ClipboardState,
    content_path: &str,
) -> Result<Vec<u8>, String> {
    {
        let cache = state
            .thumbnail_cache
            .lock()
            .map_err(|error| error.to_string())?;
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

    let mut cache = state
        .thumbnail_cache
        .lock()
        .map_err(|error| error.to_string())?;
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
        let file_name = format!(
            "clipboard-image-{}.png",
            chrono::Utc::now().timestamp_millis()
        );
        save_png_preview(state, &file_name, bytes)?
    };
    let dimensions = image::load_from_memory(bytes)
        .ok()
        .map(|image| (image.width() as i32, image.height() as i32));
    let (image_width, image_height) = dimensions.unwrap_or((0, 0));
    let summary = format!("{image_width} x {image_height}");

    state.with_store_mut(|store| {
        let changed = upsert_clip_item(
            store,
            UpsertClipInput {
                kind: "image".into(),
                content_text: summary,
                content_path: Some(content_path),
                file_paths: Vec::new(),
                image_width: Some(image_width),
                image_height: Some(image_height),
                preview_data_url: None,
            },
        );
        if changed {
            state.save(store).map_err(|error| error.to_string())?;
        }
        Ok(changed)
    })
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

pub fn get_clip_image_preview(
    state: &ClipboardState,
    id: i64,
) -> Result<Option<ClipImagePreview>, String> {
    let content_path = state.with_store(|store| {
        let Some(clip) = store
            .clips
            .iter()
            .find(|clip| clip.id == id && clip.kind == "image")
        else {
            return Ok(None);
        };
        Ok(clip.content_path.clone())
    })?;

    let Some(content_path) = content_path else {
        return Ok(None);
    };

    let bytes = generate_preview_png_bytes(state, &content_path)?;
    Ok(Some(ClipImagePreview {
        bytes,
        mime_type: "image/png".to_string(),
    }))
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
                Body::RawImage(image) => handle_raw_image(
                    &state,
                    image.width,
                    image.height,
                    &image.bytes,
                    image.path.clone(),
                ),
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
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(CLEANUP_INTERVAL_SECS));
        let state = app.state::<ClipboardState>();
        match cleanup_expired_clips(&state) {
            Ok(true) => emit_clips_changed(&app),
            Ok(false) => {}
            Err(error) => eprintln!("[cleanup] failed to prune expired clips: {error}"),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::clean_html2text_output;

    #[test]
    fn clean_html_output_removes_table_rules_and_markup() {
        let input = r#"
            ## Header
            │ col-a │ col-b │
            ├───────┼───────┤
            │ a     │ b     │
            > quote
        "#;

        let output = clean_html2text_output(input);

        assert!(output.contains("Header"));
        assert!(output.contains("col-a\tcol-b"));
        assert!(output.contains("a\tb"));
        assert!(output.contains("quote"));
        assert!(!output.contains("────"));
    }
}
