#![allow(clippy::needless_return)]

use eframe::egui;
use egui::{text::LayoutJob, ColorImage, RichText, TextureHandle};
use image::GenericImageView;
use rfd::FileDialog;
use std::{
    fs,
    path::{Path, PathBuf},
};

const MAX_FILE_SIZE_BYTES: u64 = 10_000_000; // 10MB
const MAX_RECENT_FILES: usize = 10;

pub enum Content {
    Text(String),
    Image(TextureHandle),
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct FileViewerApp {
    #[serde(skip)]
    content: Option<Content>,
    #[serde(skip)]
    current_path: Option<PathBuf>,
    #[serde(skip)]
    error_message: Option<String>,
    dark_mode: bool,
    recent_files: Vec<PathBuf>,
    show_line_numbers: bool,
    word_wrap: bool,
    text_zoom: f32,
    image_zoom: f32,
    #[serde(skip)]
    show_about: bool,
}

impl FileViewerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage
            && let Some(s) = storage.get_string(eframe::APP_KEY)
            && let Ok(app) = serde_json::from_str::<FileViewerApp>(&s)
        {
            return app;
        }
        if let Some(app) = Self::load_settings_from_disk() {
            return app;
        }
        Default::default()
    }

    fn apply_theme(&self, ctx: &egui::Context) {
        let mut visuals = if self.dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };

        // Accent colors
        visuals.selection.bg_fill = if self.dark_mode {
            egui::Color32::from_rgb(80, 140, 255)
        } else {
            egui::Color32::from_rgb(0, 110, 230)
        };
        visuals.hyperlink_color = visuals.selection.bg_fill;

        // Start from current style, adjust spacing, then inject our visuals
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 6.0);
        style.visuals = visuals;
        ctx.set_style(style);
    }

    fn is_supported_image(path: &Path) -> bool {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp")
    }

    fn load_text(&self, path: &Path) -> Result<String, String> {
        let bytes = fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn load_image(&self, path: &Path) -> Result<ColorImage, String> {
        let img = image::open(path).map_err(|e| format!("Failed to open image: {}", e))?;
        let (width, height) = img.dimensions();
        let rgba = img.to_rgba8();
        let pixels = rgba.into_flat_samples();
        Ok(ColorImage::from_rgba_unmultiplied([
            width as _,
            height as _,
        ], pixels.as_slice()))
    }

    pub fn load_file(&mut self, path: PathBuf, ctx: &egui::Context) {
        self.content = None;
        self.error_message = None;
        self.current_path = None;

        if let Ok(metadata) = fs::metadata(&path)
            && metadata.len() > MAX_FILE_SIZE_BYTES
        {
            self.error_message = Some(format!(
                "File is too large (> {:.1}MB)",
                MAX_FILE_SIZE_BYTES as f64 / 1_000_000.0
            ));
            return;
        }

        let loaded = if Self::is_supported_image(&path) {
            match self.load_image(&path) {
                Ok(color_image) => {
                    let texture = ctx.load_texture(
                        path.to_string_lossy(),
                        color_image,
                        egui::TextureOptions::LINEAR,
                    );
                    Ok(Content::Image(texture))
                }
                Err(e) => Err(e),
            }
        } else {
            match self.load_text(&path) {
                Ok(text) => Ok(Content::Text(text)),
                Err(e) => Err(e),
            }
        };

        match loaded {
            Ok(content) => {
                self.content = Some(content);
                self.current_path = Some(path.clone());
                // Deduplicate and push to recents
                self.recent_files.retain(|p| p != &path);
                self.recent_files.push(path);
                if self.recent_files.len() > MAX_RECENT_FILES {
                    let overflow = self.recent_files.len() - MAX_RECENT_FILES;
                    self.recent_files.drain(0..overflow);
                }
                // Persist updated recents immediately
                self.save_settings_to_disk();
            }
            Err(e) => self.error_message = Some(e),
        }
    }

    fn settings_path() -> Option<PathBuf> {
        directories::ProjectDirs::from("", "", "gemini-file-viewer")
            .map(|dirs| dirs.config_dir().join("settings.json"))
    }

    fn load_settings_from_disk() -> Option<Self> {
        let path = Self::settings_path()?;
        let data = fs::read(&path).ok()?;
        serde_json::from_slice::<Self>(&data).ok()
    }

    fn save_settings_to_disk(&self) {
        if let Some(path) = Self::settings_path() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(s) = serde_json::to_vec_pretty(self) {
                let _ = fs::write(path, s);
            }
        }
    }
}

impl Default for FileViewerApp {
    fn default() -> Self {
        Self {
            content: None,
            current_path: None,
            error_message: None,
            dark_mode: true,
            recent_files: Vec::new(),
            show_line_numbers: true,
            word_wrap: true,
            text_zoom: 1.0,
            image_zoom: 1.0,
            show_about: false,
        }
    }
}

impl eframe::App for FileViewerApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if let Ok(s) = serde_json::to_string(self) {
            storage.set_string(eframe::APP_KEY, s);
        }
        self.save_settings_to_disk();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply polished visuals and spacing (initial pass).
        self.apply_theme(ctx);

        let mut file_to_load: Option<PathBuf> = None;

        // Keyboard shortcuts and Ctrl+Wheel zoom
        let mut toggle_dark = false;
        ctx.input(|i| {
            if i.modifiers.command && i.key_pressed(egui::Key::O) {
                if let Some(path) = FileDialog::new()
                    .add_filter("All Supported", &["txt","rs","toml","md","json","js","html","css","png","jpg","jpeg","gif","bmp","webp"])
                    .add_filter("Images", &["png","jpg","jpeg","gif","bmp","webp"])
                    .add_filter("Text/Source", &["txt","rs","toml","md","json","js","html","css"])
                    .pick_file()
                {
                    file_to_load = Some(path);
                }
            }
            if i.modifiers.command && i.key_pressed(egui::Key::D) {
                toggle_dark = true;
            }
            if i.modifiers.command && i.key_pressed(egui::Key::L) {
                self.show_line_numbers = !self.show_line_numbers;
                self.save_settings_to_disk();
            }
            if i.modifiers.command && i.key_pressed(egui::Key::W) {
                self.word_wrap = !self.word_wrap;
                self.save_settings_to_disk();
            }

            // Ctrl + Mouse wheel zoom for content
            if i.modifiers.command && i.raw_scroll_delta.y != 0.0 {
                let dir = i.raw_scroll_delta.y.signum();
                match &self.content {
                    Some(Content::Text(_)) => {
                        let factor = if dir > 0.0 { 1.05 } else { 1.0 / 1.05 };
                        self.text_zoom = (self.text_zoom * factor).clamp(0.6, 3.0);
                    }
                    Some(Content::Image(_)) => {
                        let factor = if dir > 0.0 { 1.10 } else { 1.0 / 1.10 };
                        self.image_zoom = (self.image_zoom * factor).clamp(0.1, 6.0);
                    }
                    _ => {}
                }
            }
        });

        // About dialog
        if self.show_about {
            egui::Window::new("About Gemini File Viewer")
                .collapsible(false)
                .resizable(false)
                .open(&mut self.show_about)
                .show(ctx, |ui| {
                    ui.label(RichText::new("Gemini File Viewer 2.0").strong());
                    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                    ui.separator();
                    ui.label("Shortcuts:");
                    ui.monospace("Ctrl+O — Open file");
                    ui.monospace("Ctrl+D — Toggle dark mode");
                    ui.monospace("Ctrl+L — Toggle line numbers");
                    ui.monospace("Ctrl+W — Toggle word wrap");
                    ui.monospace("Ctrl+Wheel — Zoom text/image");
                });
        }
        if toggle_dark {
            self.dark_mode = !self.dark_mode;
            self.apply_theme(ctx);
            self.save_settings_to_disk();
        }

        // Top Toolbar
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button(RichText::new("📂 Open File…"))
                    .clicked()
                    && let Some(path) = FileDialog::new()
                        .add_filter("All Supported", &["txt","rs","toml","md","json","js","html","css","png","jpg","jpeg","gif","bmp","webp"])
                        .add_filter("Images", &["png","jpg","jpeg","gif","bmp","webp"])
                        .add_filter("Text/Source", &["txt","rs","toml","md","json","js","html","css"])
                        .pick_file()
                {
                    file_to_load = Some(path);
                }

                ui.menu_button(RichText::new("🕘 Recent Files"), |ui| {
                    ui.set_min_width(480.0);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    if self.recent_files.is_empty() {
                        ui.label("(empty)");
                    }
                    for file in self.recent_files.iter().rev().cloned() {
                        let display = file.to_string_lossy();
                        if ui
                            .button(egui::RichText::new(display.clone()).monospace())
                            .on_hover_text(display)
                            .clicked()
                        {
                            file_to_load = Some(file);
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    if ui.button("🧹 Clear Recent Files").clicked() {
                        self.recent_files.clear();
                        ui.close_menu();
                    }
                });

                ui.separator();
                let prev_dark = self.dark_mode;
                let prev_lines = self.show_line_numbers;
                ui.checkbox(&mut self.dark_mode, "🌙 Dark Mode");
                ui.checkbox(&mut self.show_line_numbers, "🔢 Line Numbers");
                if self.dark_mode != prev_dark {
                    // Reapply theme immediately so toggle takes effect this frame.
                    self.apply_theme(ctx);
                }
                if self.dark_mode != prev_dark || self.show_line_numbers != prev_lines {
                    self.save_settings_to_disk();
                }
                ui.separator();

                if ui.button("🧹 Clear").clicked() {
                    self.content = None;
                    self.current_path = None;
                    self.error_message = None;
                }
            });
        });

        // Status Bar
        egui::TopBottomPanel::bottom("statusbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(path) = &self.current_path {
                    ui.monospace(format!("📄 {}", path.to_string_lossy()));
                    if let Ok(metadata) = fs::metadata(path) {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("({:.1} KB)", metadata.len() as f64 / 1024.0));
                        });
                    }
                    if ui.button("Copy Path").on_hover_text("Copy path to clipboard").clicked() {
                        ui.ctx().copy_text(path.to_string_lossy().into());
                    }
                    #[cfg(target_os = "windows")]
                    if ui.button("Open Folder").clicked() {
                        if let Some(parent) = path.parent() {
                            let _ = std::process::Command::new("explorer").arg(parent).spawn();
                        }
                    }
                    if let Some(Content::Image(texture)) = &self.content {
                        let size = texture.size();
                        ui.separator();
                        ui.label(format!("{}×{} px", size[0], size[1]));
                        ui.label(format!("{:.0}%", self.image_zoom * 100.0));
                    }
                } else {
                    ui.label("No file selected.");
                }
            });
        });

        // Main Content
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(err) = &self.error_message {
                ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
            }

            if let Some(content) = &self.content {
                match content {
                    Content::Text(text) => {
                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            // Wrap preference
                            ui.style_mut().wrap_mode = Some(if self.word_wrap { egui::TextWrapMode::Wrap } else { egui::TextWrapMode::Extend });
                            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                                let text_style = egui::TextStyle::Monospace;
                                let mut font_id = text_style.resolve(ui.style());
                                font_id.size = (font_id.size * self.text_zoom).clamp(8.0, 48.0);
                                let text_color = ui.visuals().text_color();
                                if self.show_line_numbers {
                                    let mut job = LayoutJob::default();
                                    for (i, line) in text.lines().enumerate() {
                                        job.append(&format!("{:>4} ", i + 1), 0.0, egui::TextFormat { font_id: font_id.clone(), color: egui::Color32::GRAY, ..Default::default() });
                                        job.append(line, 0.0, egui::TextFormat { font_id: font_id.clone(), color: text_color, ..Default::default() });
                                        job.append("\n", 0.0, egui::TextFormat::default());
                                    }
                                    ui.label(job);
                                } else {
                                    ui.label(RichText::new(text).monospace().size(font_id.size));
                                }
                            });
                        });
                    }
                    Content::Image(texture) => {
                        egui::ScrollArea::both().show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                let size = texture.size();
                                let desired = egui::vec2(size[0] as f32 * self.image_zoom, size[1] as f32 * self.image_zoom);
                                ui.add_sized(desired, egui::Image::new(texture));
                            });
                        });
                    }
                }
            } else if self.error_message.is_none() {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.25);
                    ui.label(RichText::new("✨ Gemini File Viewer").heading());
                    ui.add_space(6.0);
                    ui.label("Open a file to get started.");
                });
            }
        });

        // Deferred file loading to avoid borrow issues
        if let Some(path) = file_to_load {
            self.load_file(path, ctx);
        }
    }
}
