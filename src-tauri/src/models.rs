use serde::{Deserialize, Serialize};

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
