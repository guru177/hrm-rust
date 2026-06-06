use std::path::{Path, PathBuf};

const MAX_PHOTO_BYTES: usize = 2 * 1024 * 1024;

pub fn storage_root() -> PathBuf {
    PathBuf::from(
        std::env::var("STORAGE_PATH").unwrap_or_else(|_| "../frontend/public/storage".to_string()),
    )
}

fn extension_from_mime(mime: Option<&str>) -> &'static str {
    match mime {
        Some("image/png") => "png",
        Some("image/gif") => "gif",
        Some("image/webp") => "webp",
        _ => "jpg",
    }
}

fn extension_from_filename(name: Option<&str>) -> &'static str {
    name.and_then(|n| Path::new(n).extension())
        .and_then(|e| e.to_str())
        .map(|e| match e.to_ascii_lowercase().as_str() {
            "png" => "png",
            "gif" => "gif",
            "webp" => "webp",
            "jpeg" | "jpg" => "jpg",
            _ => "jpg",
        })
        .unwrap_or("jpg")
}

/// Save user profile photo; returns DB path like `users/<uuid>.jpg`.
pub fn save_user_photo(
    data: &[u8],
    content_type: Option<&str>,
    filename: Option<&str>,
) -> Result<String, String> {
    if data.is_empty() {
        return Err("Empty file".into());
    }
    if data.len() > MAX_PHOTO_BYTES {
        return Err("Photo must be less than 2MB".into());
    }

    let ext = if content_type.is_some() {
        extension_from_mime(content_type)
    } else {
        extension_from_filename(filename)
    };

    let relative = format!("users/{}.{}", uuid::Uuid::new_v4(), ext);
    let full = storage_root().join(&relative);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&full, data).map_err(|e| e.to_string())?;
    Ok(relative)
}

pub fn delete_photo_path(relative: &str) {
    if relative.is_empty() || relative.contains("..") {
        return;
    }
    let full = storage_root().join(relative);
    let _ = std::fs::remove_file(full);
}
