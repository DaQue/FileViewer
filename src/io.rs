use eframe::egui::ColorImage;
use image::GenericImageView;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_IMAGE_TEXTURE_BYTES: usize = 128 * 1024 * 1024; // ~128 MB RGBA texture limit

pub(crate) fn is_supported_image(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp")
}

pub(crate) fn load_text(path: &Path) -> Result<(String, bool, usize), String> {
    let bytes = fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let lossy = text.contains('\u{FFFD}');
    let lines = text.lines().count();
    Ok((text, lossy, lines))
}

pub(crate) fn load_image(path: &Path) -> Result<ColorImage, String> {
    // Pre-check dimensions to estimate texture memory before decoding
    if let Ok((w, h)) = image::image_dimensions(path) {
        let est_bytes: usize = (w as usize)
            .saturating_mul(h as usize)
            .saturating_mul(4);
        if est_bytes > MAX_IMAGE_TEXTURE_BYTES {
            return Err(format!(
                "Image too large: {}x{} (~{:.1} MB RGBA). Limit ~{:.0} MB",
                w,
                h,
                est_bytes as f64 / (1024.0 * 1024.0),
                MAX_IMAGE_TEXTURE_BYTES as f64 / (1024.0 * 1024.0)
            ));
        }
    }

    let img = image::open(path).map_err(|e| format!("Failed to open image: {}", e))?;
    let (width, height) = img.dimensions();
    let rgba = img.to_rgba8();
    let pixels = rgba.into_flat_samples();
    Ok(ColorImage::from_rgba_unmultiplied([
        width as _,
        height as _,
    ], pixels.as_slice()))
}

pub(crate) fn is_supported_text(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(
        ext.as_str(),
        "txt" | "rs" | "py" | "toml" | "md" | "json" | "js" | "html" | "css"
    )
}

pub(crate) fn neighbor_image(path: &Path, forward: bool) -> Option<PathBuf> {
    let parent = path.parent()?;
    let mut images: Vec<PathBuf> = std::fs::read_dir(parent).ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && is_supported_image(p))
        .collect();
    if images.is_empty() { return None; }
    images.sort();
    let current_name = path.file_name()?;
    let idx = images.iter().position(|p| p.file_name() == Some(current_name))?;
    if images.len() <= 1 { return None; }
    let next_idx = if forward {
        (idx + 1) % images.len()
    } else {
        (idx + images.len() - 1) % images.len()
    };
    images.get(next_idx).cloned()
}

pub(crate) fn neighbor_text(path: &Path, forward: bool) -> Option<PathBuf> {
    let parent = path.parent()?;
    let mut texts: Vec<PathBuf> = std::fs::read_dir(parent).ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && is_supported_text(p))
        .collect();
    if texts.is_empty() { return None; }
    texts.sort();
    let current_name = path.file_name()?;
    let idx = texts.iter().position(|p| p.file_name() == Some(current_name))?;
    if texts.len() <= 1 { return None; }
    let next_idx = if forward {
        (idx + 1) % texts.len()
    } else {
        (idx + texts.len() - 1) % texts.len()
    };
    texts.get(next_idx).cloned()
}

