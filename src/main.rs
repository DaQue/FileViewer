#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use egui::{ColorImage, TextureHandle, text::LayoutJob};
use image::GenericImageView;
use rfd::FileDialog;
use std::fs;
use std::path::PathBuf;

const MAX_FILE_SIZE_BYTES: u64 = 10_000_000; // 10MB
const MAX_RECENT_FILES: usize = 10;

// Main application entry point
fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_resizable(true)
            .with_title("Gemini File Viewer 2.0"),
        ..Default::default()
    };
    eframe::run_native(
        "Gemini File Viewer 2.0",
        options,
        Box::new(|cc| Ok(Box::new(FileViewerApp::new(cc))))
    )
}

// Enum to hold the currently displayed content
enum Content {
    Text(String),
    Image(TextureHandle),
}

// Main application state
#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
struct FileViewerApp {
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
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            if let Some(s) = storage.get_string(eframe::APP_KEY) {
                if let Ok(app) = serde_json::from_str::<FileViewerApp>(&s) {
                    return app;
                }
            }
        }
        Default::default()
    }
    // Handles loading a file (text or image)
    fn load_file(&mut self, path: PathBuf, ctx: &egui::Context) {
        // Clear previous state
        self.content = None;
        self.error_message = None;
        self.current_path = None;

        // Check file size
        if let Ok(metadata) = fs::metadata(&path) {
            if metadata.len() > MAX_FILE_SIZE_BYTES {
                self.error_message = Some(format!("File is too large (> {:.1}MB)", MAX_FILE_SIZE_BYTES as f64 / 1_000_000.0));
                return;
            }
        }

        // Load based on extension
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        let is_image = ["png", "jpg", "jpeg", "gif", "bmp", "webp"].contains(&extension.as_str());

        if is_image {
            match fs::read(&path) {
                Ok(image_data) => {
                    match image::load_from_memory(&image_data) {
                        Ok(image) => {
                            let (width, height) = image.dimensions();
                            let image_buffer = image.to_rgba8();
                            let pixels = image_buffer.into_flat_samples();
                            let color_image = ColorImage::from_rgba_unmultiplied([width as _, height as _], pixels.as_slice());
                            let texture = ctx.load_texture(path.to_string_lossy(), color_image, Default::default());
                            self.content = Some(Content::Image(texture));
                        }
                        Err(e) => self.error_message = Some(format!("Failed to decode image: {}", e)),
                    }
                }
                Err(e) => self.error_message = Some(format!("Failed to read file: {}", e)),
            }
        } else {
            match fs::read_to_string(&path) {
                Ok(text) => self.content = Some(Content::Text(text)),
                Err(e) => self.error_message = Some(format!("Failed to read text file: {}", e)),
            }
        }

        // If loading was successful, update path and recent files
        if self.error_message.is_none() {
            self.current_path = Some(path.clone());
            if !self.recent_files.contains(&path) {
                self.recent_files.push(path);
                if self.recent_files.len() > MAX_RECENT_FILES {
                    self.recent_files.remove(0);
                }
            }
        }
    }
}

// eframe::App implementation where the UI is defined
impl eframe::App for FileViewerApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if let Ok(s) = serde_json::to_string(self) {
            storage.set_string(eframe::APP_KEY, s);
        }
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(if self.dark_mode { egui::Visuals::dark() } else { egui::Visuals::light() });

        let mut file_to_load = None;

        // --- Top Toolbar --- 
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("📁 Open File").clicked() {
                    if let Some(path) = FileDialog::new().add_filter("All Supported Files", &["txt", "rs", "toml", "md", "json", "js", "html", "css", "png", "jpg", "jpeg", "gif", "bmp", "webp"]).pick_file() {
                        file_to_load = Some(path);
                    }
                }

                ui.menu_button("📋 Recent Files", |ui| {
                    if self.recent_files.is_empty() {
                        ui.label("(empty)");
                    }
                    for file in self.recent_files.iter().rev().cloned() {
                        if let Some(file_name) = file.file_name() {
                            if ui.button(file_name.to_string_lossy()).clicked() {
                                file_to_load = Some(file);
                                ui.close_menu();
                            }
                        }
                    }
                });

                ui.separator();
                ui.checkbox(&mut self.dark_mode, "🌙 Dark Mode");
                ui.checkbox(&mut self.show_line_numbers, "🔢 Line Numbers");
                ui.separator();

                if ui.button("🗑️ Clear").clicked() {
                    self.content = None;
                    self.current_path = None;
                    self.error_message = None;
                }
            });
        });

        // --- Status Bar --- 
        egui::TopBottomPanel::bottom("statusbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(path) = &self.current_path {
                    ui.label("📄");
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

        // --- Main Content Panel --- 
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(err) = &self.error_message {
                ui.colored_label(egui::Color32::RED, format!("❌ {}", err));
            }

            if let Some(content) = &self.content {
                match content {
                    Content::Text(text) => {
                        egui::ScrollArea::both().show(ui, |ui| {
                            if self.show_line_numbers {
                                // Custom layout for line numbers
                                let mut job = LayoutJob::default();
                                let text_style = egui::TextStyle::Monospace;
                                let text_color = ui.visuals().text_color();

                                for (i, line) in text.lines().enumerate() {
                                    job.append(&format!("{:>4} │ ", i + 1), 0.0, egui::TextFormat { font_id: text_style.resolve(ui.style()), color: egui::Color32::GRAY, ..Default::default() });
                                    job.append(line, 0.0, egui::TextFormat { font_id: text_style.resolve(ui.style()), color: text_color, ..Default::default() });
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

        // Load the file after the UI has been drawn to avoid borrow checker errors.
        if let Some(path) = file_to_load {
            self.load_file(path, ctx);
        }
    }
}
