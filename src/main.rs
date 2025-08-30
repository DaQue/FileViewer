
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use egui::{ColorImage, TextureHandle};
use image::GenericImageView;
use rfd::FileDialog;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_resizable(true),
        ..Default::default()
    };
    eframe::run_native(
        "Gemini File Viewer",
        options,
        Box::new(|_cc| Ok(Box::<FileViewerApp>::default())),
    )
}

#[derive(Default)]
struct FileViewerApp {
    content: Option<Result<Content, String>>,
}

enum Content {
    Text(String),
    Image { texture: TextureHandle },
}

impl eframe::App for FileViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("File Viewer");
            ui.add_space(10.0);

            if ui.button("Open File...").clicked() {
                self.content = None; // Clear previous content
                if let Some(path) = pick_file() {
                    self.content = Some(load_file(&path, ctx));
                }
            }

            ui.add_space(10.0);
            ui.separator();

            if let Some(content_result) = &self.content {
                match content_result {
                    Ok(content) => match content {
                        Content::Text(text) => {
                            egui::ScrollArea::both().show(ui, |ui| {
                                ui.label(text);
                            });
                        }
                        Content::Image { texture, .. } => {
                            // Let egui scale the image to fit the available space while maintaining aspect ratio.
                            let image_widget = egui::Image::new(texture).max_size(ui.available_size());
                            ui.add(image_widget);
                        }
                    },
                    Err(e) => {
                        ui.colored_label(egui::Color32::RED, format!("Error: {}", e));
                    }
                }
            }
        });
    }
}

fn pick_file() -> Option<PathBuf> {
    FileDialog::new()
        .add_filter("Text & Source Code", &["txt", "rs", "toml", "md", "json", "js", "html", "css"])
        .add_filter("Images", &["png", "jpg", "jpeg"])
        .pick_file()
}

fn load_file(path: &PathBuf, ctx: &egui::Context) -> Result<Content, String> {
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    match extension {
        "png" | "jpg" | "jpeg" => {
            let image_data = fs::read(path).map_err(|e| e.to_string())?;
            let image = image::load_from_memory(&image_data).map_err(|e| e.to_string())?;
            let (width, height) = image.dimensions();
            let image_buffer = image.to_rgba8();
            let pixels = image_buffer.into_flat_samples();
            let color_image = ColorImage::from_rgba_unmultiplied([width as _, height as _], pixels.as_slice());

            let texture = ctx.load_texture(
                path.to_string_lossy(),
                color_image,
                Default::default(),
            );

            Ok(Content::Image { texture })
        }
        _ => { // Default to text for all other filtered types
            fs::read_to_string(path)
                .map(Content::Text)
                .map_err(|e| e.to_string())
        }
    }
}
