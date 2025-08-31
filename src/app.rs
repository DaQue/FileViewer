#![allow(clippy::needless_return)]

use eframe::egui;
use egui::{text::LayoutJob, ColorImage, TextureHandle};
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

#[derive(Default, serde::Deserialize, serde::Serialize)]
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
}

impl FileViewerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            if let Some(s) = storage.get_string(eframe::APP_KEY) {
                if let Ok(app) = serde_json::from_str::<FileViewerApp>(&s) {
                    return app;
                }
            }
        }
        Default::default()
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

        if let Ok(metadata) = fs::metadata(&path) {
            if metadata.len() > MAX_FILE_SIZE_BYTES {
                self.error_message = Some(format!(
                    "File is too large (> {:.1}MB)",
                    MAX_FILE_SIZE_BYTES as f64 / 1_000_000.0
                ));
                return;
            }
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
            }
            Err(e) => self.error_message = Some(e),
        }
    }
}

impl eframe::App for FileViewerApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if let Ok(s) = serde_json::to_string(self) {
            storage.set_string(eframe::APP_KEY, s);
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(if self.dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        });

        let mut file_to_load: Option<PathBuf> = None;

        // Top Toolbar
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("Open File…").clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("All Supported", &["txt","rs","toml","md","json","js","html","css","png","jpg","jpeg","gif","bmp","webp"])
                        .add_filter("Images", &["png","jpg","jpeg","gif","bmp","webp"])
                        .add_filter("Text/Source", &["txt","rs","toml","md","json","js","html","css"])
                        .pick_file()
                    {
                        file_to_load = Some(path);
                    }
                }

                ui.menu_button("Recent Files", |ui| {
                    ui.set_min_width(480.0);
                    ui.style_mut().wrap = Some(false);
                    if self.recent_files.is_empty() {
                        ui.label("(empty)");
                    }
                    for file in self.recent_files.iter().rev().cloned() {
                        if let Some(file_name) = file.file_name() {
                            let display = file.to_string_lossy();
                            if ui.button(egui::RichText::new(display.clone()).monospace()).on_hover_text(file.to_string_lossy()).clicked() {
                                file_to_load = Some(file);
                                ui.close_menu();
                            }
                        }
                    }
                    ui.separator();
                    if ui.button("Clear Recent Files").clicked() {
                        self.recent_files.clear();
                        ui.close_menu();
                    }
                });

                ui.separator();
                ui.checkbox(&mut self.dark_mode, "Dark Mode");
                ui.checkbox(&mut self.show_line_numbers, "Line Numbers");
                ui.separator();

                if ui.button("Clear").clicked() {
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
                    ui.monospace(path.to_string_lossy());
                    if let Ok(metadata) = fs::metadata(path) {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("({:.1} KB)", metadata.len() as f64 / 1024.0));
                        });
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
                        egui::ScrollArea::both().show(ui, |ui| {
                            if self.show_line_numbers {
                                let mut job = LayoutJob::default();
                                let text_style = egui::TextStyle::Monospace;
                                let text_color = ui.visuals().text_color();
                                for (i, line) in text.lines().enumerate() {
                                    job.append(
                                        &format!("{:>4} ", i + 1),
                                        0.0,
                                        egui::TextFormat {
                                            font_id: text_style.resolve(ui.style()),
                                            color: egui::Color32::GRAY,
                                            ..Default::default()
                                        },
                                    );
                                    job.append(
                                        line,
                                        0.0,
                                        egui::TextFormat {
                                            font_id: text_style.resolve(ui.style()),
                                            color: text_color,
                                            ..Default::default()
                                        },
                                    );
                                    job.append("\n", 0.0, egui::TextFormat::default());
                                }
                                ui.label(job);
                            } else {
                                ui.label(text);
                            }
                        });
                    }
                    Content::Image(texture) => {
                        let image_widget = egui::Image::new(texture).max_size(ui.available_size());
                        ui.add(image_widget);
                    }
                }
            }
        });

        // Deferred file loading to avoid borrow issues
        if let Some(path) = file_to_load {
            self.load_file(path, ctx);
        }
    }
}
