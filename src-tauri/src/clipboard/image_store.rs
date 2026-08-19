use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::PathBuf,
};
use anyhow::Result;

/// Returns the directory where captured images are stored (~/.monoclip/images/).
pub fn images_dir() -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let dir = home.join(".monoclip").join("images");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Hash raw RGBA bytes for deduplication.  Samples every 64th byte to keep it fast.
pub fn hash_rgba(bytes: &[u8]) -> u64 {
    let mut h = DefaultHasher::new();
    bytes.len().hash(&mut h);
    for chunk in bytes.chunks(64) {
        chunk[0].hash(&mut h);
    }
    h.finish()
}

/// Save raw RGBA image data as a PNG file.
/// Returns the absolute path to the saved file, or None if encoding fails.
pub fn save_as_png(rgba: &[u8], width: u32, height: u32) -> Result<String> {
    let hash = hash_rgba(rgba);
    let dir = images_dir()?;
    let path = dir.join(format!("{:016x}.png", hash));

    // Don't re-encode if we already saved this exact image
    if !path.exists() {
        let img = image::RgbaImage::from_raw(width, height, rgba.to_vec())
            .ok_or_else(|| anyhow::anyhow!("Failed to create RgbaImage from clipboard data"))?;
        img.save_with_format(&path, image::ImageFormat::Png)?;
    }

    Ok(path.to_string_lossy().into_owned())
}

/// Delete an image file that belongs to a clip being hard-deleted.
pub fn delete_image_file(path: &str) {
    let p = std::path::Path::new(path);
    // Only delete files inside our managed images dir
    if let Ok(images) = images_dir() {
        if p.starts_with(&images) {
            let _ = std::fs::remove_file(p);
        }
    }
}
