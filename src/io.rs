use eframe::egui::ColorImage;
use image::GenericImageView;
use std::fs;
use std::path::Path;

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

