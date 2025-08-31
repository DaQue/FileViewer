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
const BIG_TEXT_CHAR_THRESHOLD: usize = 500_000; // Disable heavy features beyond this
const HIGHLIGHT_CHAR_THRESHOLD: usize = 200_000; // Disable syntax/mark highlights beyond this
const MAX_IMAGE_TEXTURE_BYTES: usize = 128 * 1024 * 1024; // ~128 MB RGBA texture limit

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
    image_fit: bool,
    // Derived/runtime-only state for text rendering
    #[serde(skip)]
    text_is_big: bool,
    #[serde(skip)]
    text_line_count: usize,
    #[serde(skip)]
    text_is_lossy: bool,
    // Simple find state
    #[serde(skip)]
    search_query: String,
    #[serde(skip)]
    search_active: bool,
    #[serde(skip)]
    search_count: usize,
    #[serde(skip)]
    search_current: usize,
}

impl FileViewerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage
            && let Some(s) = storage.get_string(eframe::APP_KEY)
            && let Ok(mut app) = serde_json::from_str::<FileViewerApp>(&s)
        {
            // ensure runtime-only fields are initialized
            app.text_is_big = false;
            app.text_line_count = 0;
            app.text_is_lossy = false;
            app.search_query = String::new();
            app.search_active = false;
            app.search_count = 0;
            return app;
        }
        if let Some(mut app) = Self::load_settings_from_disk() {
            app.text_is_big = false;
            app.text_line_count = 0;
            app.text_is_lossy = false;
            app.search_query = String::new();
            app.search_active = false;
            app.search_count = 0;
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

    fn load_text(&self, path: &Path) -> Result<(String, bool, usize), String> {
        let bytes = fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let lossy = text.contains('\u{FFFD}');
        let lines = text.lines().count();
        Ok((text, lossy, lines))
    }

    fn load_image(&self, path: &Path) -> Result<ColorImage, String> {
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
                Ok((text, lossy, lines)) => {
                    self.text_is_big = text.len() >= BIG_TEXT_CHAR_THRESHOLD || lines >= 50_000;
                    self.text_line_count = lines;
                    self.text_is_lossy = lossy;
                    Ok(Content::Text(text))
                }
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
            image_fit: false,
            text_is_big: false,
            text_line_count: 0,
            text_is_lossy: false,
            search_query: String::new(),
            search_active: false,
            search_count: 0,
            search_current: 0,
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
        // Apply visuals each frame
        self.apply_theme(ctx);

        let mut file_to_load: Option<PathBuf> = None;

        // Keyboard shortcuts
        let mut toggle_dark = false;
        ctx.input(|i| {
            if i.modifiers.command && i.key_pressed(egui::Key::O) {
                if let Some(path) = FileDialog::new()
                    .add_filter("All Supported", &["txt","rs","py","toml","md","json","js","html","css","png","jpg","jpeg","gif","bmp","webp"])
                    .add_filter("Images", &["png","jpg","jpeg","gif","bmp","webp"])
                    .add_filter("Text/Source", &["txt","rs","py","toml","md","json","js","html","css"])
                    .pick_file()
                {
                    file_to_load = Some(path);
                }
            }
            if i.modifiers.command && i.key_pressed(egui::Key::D) {
                toggle_dark = true;
            }
            if i.modifiers.command && i.key_pressed(egui::Key::F) {
                self.search_active = true;
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
                        self.image_fit = false;
                        let factor = if dir > 0.0 { 1.10 } else { 1.0 / 1.10 };
                        self.image_zoom = (self.image_zoom * factor).clamp(0.1, 6.0);
                    }
                    _ => {}
                }
            }

            // Reset and keyboard zoom shortcuts
            if i.modifiers.command && i.key_pressed(egui::Key::Num0) {
                match &self.content {
                    Some(Content::Text(_)) => self.text_zoom = 1.0,
                    Some(Content::Image(_)) => { self.image_fit = false; self.image_zoom = 1.0; },
                    _ => {}
                }
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Equals) {
                match &self.content {
                    Some(Content::Text(_)) => self.text_zoom = (self.text_zoom * 1.05).clamp(0.6, 3.0),
                    Some(Content::Image(_)) => { self.image_fit = false; self.image_zoom = (self.image_zoom * 1.10).clamp(0.1, 6.0); },
                    _ => {}
                }
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Minus) {
                match &self.content {
                    Some(Content::Text(_)) => self.text_zoom = (self.text_zoom / 1.05).clamp(0.6, 3.0),
                    Some(Content::Image(_)) => { self.image_fit = false; self.image_zoom = (self.image_zoom / 1.10).clamp(0.1, 6.0); },
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
                    ui.monospace("Ctrl+= / Ctrl+- — Zoom in/out");
                    ui.monospace("Ctrl+0 — Reset zoom");
                    ui.monospace("Ctrl+F — Find in text");
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
                    .button(RichText::new("Open File"))
                    .clicked()
                    && let Some(path) = FileDialog::new()
                        .add_filter("All Supported", &["txt","rs","py","toml","md","json","js","html","css","png","jpg","jpeg","gif","bmp","webp"])
                        .add_filter("Images", &["png","jpg","jpeg","gif","bmp","webp"])
                        .add_filter("Text/Source", &["txt","rs","py","toml","md","json","js","html","css"])
                        .pick_file()
                {
                    file_to_load = Some(path);
                }

                ui.menu_button(RichText::new("Recent Files"), |ui| {
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
                    if ui.button("Clear Recent Files").clicked() {
                        self.recent_files.clear();
                        ui.close_menu();
                    }
                });

                ui.separator();
                let prev_dark = self.dark_mode;
                let prev_lines = self.show_line_numbers;
                ui.checkbox(&mut self.dark_mode, "Dark Mode");
                ui.checkbox(&mut self.show_line_numbers, "Line Numbers");
                if self.dark_mode != prev_dark {
                    self.apply_theme(ctx);
                }
                if self.dark_mode != prev_dark || self.show_line_numbers != prev_lines {
                    self.save_settings_to_disk();
                }
                ui.separator();

                if ui.button("Clear").clicked() {
                    self.content = None;
                    self.current_path = None;
                    self.error_message = None;
                }

                // Image tools
                if matches!(self.content, Some(Content::Image(_))) {
                    ui.separator();
                    ui.checkbox(&mut self.image_fit, "Fit to Window");
                    if ui.button("Zoom -").clicked() { self.image_fit = false; self.image_zoom = (self.image_zoom / 1.10).clamp(0.1, 6.0); }
                    if ui.button("Zoom +").clicked() { self.image_fit = false; self.image_zoom = (self.image_zoom * 1.10).clamp(0.1, 6.0); }
                    if ui.button("100%").clicked() {
                        self.image_fit = false;
                        self.image_zoom = 1.0;
                    }
                }
            });
        });

        // Search Bar (only when viewing text)
        if matches!(self.content, Some(Content::Text(_))) {
            egui::TopBottomPanel::top("searchbar").show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Find:");
                    let prev = self.search_query.clone();
                    let resp = ui.text_edit_singleline(&mut self.search_query);
                    if self.search_active {
                        resp.request_focus();
                        self.search_active = false;
                    }
                    if resp.changed() || (prev.is_empty() && !self.search_query.is_empty()) {
                        self.search_count = 0;
                        self.search_current = 0;
                        if let Some(Content::Text(ref text)) = self.content {
                            if !self.search_query.is_empty() && text.len() <= HIGHLIGHT_CHAR_THRESHOLD {
                                let needle = self.search_query.to_ascii_lowercase();
                                self.search_count = text.to_ascii_lowercase().matches(&needle).count();
                            }
                        }
                    }
                    if !self.search_query.is_empty() {
                        ui.label(format!("{} match(es)", self.search_count));
                        ui.add_space(8.0);
                        if ui.button("Prev").clicked() && self.search_count > 0 {
                            if self.search_current == 0 { self.search_current = self.search_count.saturating_sub(1); } else { self.search_current -= 1; }
                        }
                        if ui.button("Next").clicked() && self.search_count > 0 {
                            self.search_current = (self.search_current + 1) % self.search_count;
                        }
                        if self.search_count > 0 {
                            ui.label(format!("{}/{}", self.search_current + 1, self.search_count));
                        }
                    }
                });
            });
        }

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
                    if ui.button("Copy Path").on_hover_text("Copy path to clipboard").clicked() {
                        ui.ctx().copy_text(path.to_string_lossy().into());
                    }
                    if ui.button("Open Folder").clicked() {
                        #[cfg(target_os = "windows")]
                        {
                            let _ = std::process::Command::new("explorer").arg(path).spawn();
                        }
                        #[cfg(target_os = "macos")]
                        {
                            let _ = std::process::Command::new("open").arg("-R").arg(path).spawn();
                        }
                        #[cfg(all(unix, not(target_os = "macos")))]
                        {
                            if let Some(parent) = path.parent() {
                                let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
                            }
                        }
                    }
                } else {
                    ui.label("No file selected.");
                }
            });
        });

        // Extra status information
        egui::TopBottomPanel::bottom("status-extra").show(ctx, |ui| {
            ui.horizontal(|ui| {
                match &self.content {
                    Some(Content::Image(texture)) => {
                        let size = texture.size();
                        ui.label(format!("Image: {}x{} px", size[0], size[1]));
                        let eff = if self.image_fit {
                            // best-effort: approximate using window size if available later
                            None
                        } else { Some(self.image_zoom) };
                        if let Some(z) = eff { ui.label(format!("Zoom: {:.0}%", z * 100.0)); }
                        let est = (size[0] as usize).saturating_mul(size[1] as usize).saturating_mul(4);
                        ui.label(format!("Texture ~{:.1} MB", est as f64 / (1024.0 * 1024.0)));
                        if self.image_fit { ui.label("Fit: on"); }
                    }
                    Some(Content::Text(_)) => {
                        ui.label(format!("Lines: {}", self.text_line_count));
                        ui.label(format!("Zoom: {:.0}%", self.text_zoom * 100.0));
                        if self.text_is_big {
                            ui.label("Large file: reduced features");
                        }
                        if self.text_is_lossy {
                            ui.label("UTF-8 (lossy)");
                        }
                    }
                    _ => {}
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

                                let do_line_numbers = self.show_line_numbers && !self.text_is_big;
                                let do_highlight = !self.text_is_big && text.len() <= HIGHLIGHT_CHAR_THRESHOLD;
                                if do_line_numbers || do_highlight || !self.search_query.is_empty() {
                                    let mut job = LayoutJob::default();
                                    let mut bracket_depth: i32 = 0;
                                    let ext = self
                                        .current_path
                                        .as_ref()
                                        .and_then(|p| p.extension().and_then(|s| s.to_str()))
                                        .unwrap_or("")
                                        .to_lowercase();
                                    for (i, line) in text.lines().enumerate() {
                                        if do_line_numbers {
                                            job.append(&format!("{:>4} ", i + 1), 0.0, egui::TextFormat { font_id: font_id.clone(), color: egui::Color32::GRAY, ..Default::default() });
                                        }
                                        append_highlighted(&mut job, line, &ext, &self.search_query, font_id.clone(), text_color, do_highlight, &mut bracket_depth);
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
                        let viewport = ui.available_size();
                        egui::ScrollArea::both().show(ui, |ui| {
                            ui.centered_and_justified(|ui| {
                                let size = texture.size();
                                let mut effective_zoom = self.image_zoom;
                                if self.image_fit {
                                    // Use the outer viewport size captured before the ScrollArea
                                    let sx = if size[0] > 0 { viewport.x / size[0] as f32 } else { 1.0 };
                                    let sy = if size[1] > 0 { viewport.y / size[1] as f32 } else { 1.0 };
                                    let fit = sx.min(sy);
                                    if fit.is_finite() && fit > 0.0 {
                                        effective_zoom = fit.clamp(0.1, 6.0);
                                    }
                                }
                                let desired = egui::vec2(size[0] as f32 * effective_zoom, size[1] as f32 * effective_zoom);
                                let image = egui::Image::new(texture).fit_to_exact_size(desired);
                                let resp = ui.add(image);
                                if resp.hovered() {
                                    let scroll = ui.input(|i| i.raw_scroll_delta.y);
                                    if scroll != 0.0 {
                                        self.image_fit = false;
                                        let factor = if scroll > 0.0 { 1.10 } else { 1.0 / 1.10 };
                                        self.image_zoom = (self.image_zoom * factor).clamp(0.1, 6.0);
                                    }
                                }
                            });
                        });
                    }
                }
            } else if self.error_message.is_none() {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.25);
                    ui.label(RichText::new("Gemini File Viewer").heading());
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

fn append_with_search(job: &mut LayoutJob, text: &str, font_id: egui::FontId, color: egui::Color32, query: &str, current_idx: usize, counter: &mut usize) {
    if query.is_empty() {
        job.append(text, 0.0, egui::TextFormat { font_id, color, ..Default::default() });
        return;
    }
    let lc_query = query.to_ascii_lowercase();
    let mut rest = text;
    loop {
        let lc_rest = rest.to_ascii_lowercase();
        if let Some(found_rel) = lc_rest.find(&lc_query) {
            let prefix = &rest[..found_rel];
            if !prefix.is_empty() {
                job.append(prefix, 0.0, egui::TextFormat { font_id: font_id.clone(), color, ..Default::default() });
            }
            let matched = &rest[found_rel..found_rel + lc_query.len()];
            let mut fmt = egui::TextFormat { font_id: font_id.clone(), color, ..Default::default() };
            if *counter == current_idx {
                fmt.background = egui::Color32::from_rgba_premultiplied(224, 108, 117, 96);
            } else {
                fmt.background = egui::Color32::from_rgba_premultiplied(255, 255, 0, 64);
            }
            job.append(matched, 0.0, fmt);
            *counter += 1;
            rest = &rest[found_rel + lc_query.len()..];
            if rest.is_empty() { break; }
        } else {
            if !rest.is_empty() {
                job.append(rest, 0.0, egui::TextFormat { font_id, color, ..Default::default() });
            }
            break;
        }
    }
}

fn token_highlight(
    job: &mut LayoutJob,
    text: &str,
    ext: &str,
    font_id: egui::FontId,
    base_color: egui::Color32,
    query: &str,
    do_syntax: bool,
    depth: &mut i32,
) {
    if !do_syntax {
        append_with_search(job, text, font_id, base_color, query, usize::MAX, &mut 0);
        return;
    }
    let kw_color = egui::Color32::from_rgb(97, 175, 239); // blue-ish
    let num_color = egui::Color32::from_rgb(209, 154, 102); // orange-ish
    let bool_color = egui::Color32::from_rgb(198, 120, 221); // purple-ish
    let bracket_colors = [
        egui::Color32::from_rgb(152, 195, 121), // green
        egui::Color32::from_rgb(224, 108, 117), // red
        egui::Color32::from_rgb(97, 175, 239),  // blue
        egui::Color32::from_rgb(229, 192, 123), // yellow
        egui::Color32::from_rgb(86, 182, 194),  // cyan
    ];

    let keywords_rs: &[&str] = &[
        // Stable Rust keywords
        "as","async","await","break","const","continue","crate","dyn","else","enum","extern","false","fn","for","if","impl","in","let","loop","match","mod","move","mut","pub","ref","return","self","Self","static","struct","super","trait","true","type","unsafe","use","where","while",
        // Common future/reserved
        "union","box","try","yield","macro","macro_rules"
    ];
    let keywords_py: &[&str] = &[
        // Python 3.11+ keywords
        "False","None","True","and","as","assert","async","await","break","class","continue","def","del","elif","else","except","finally","for","from","global","if","import","in","is","lambda","nonlocal","not","or","pass","raise","return","try","while","with","yield","match","case"
    ];

    // Simple word tokenizer
    let mut buf = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            buf.push(ch);
        } else {
            if !buf.is_empty() {
                let lc = buf.to_ascii_lowercase();
                let (color, _) = if ext == "rs" && keywords_rs.contains(&buf.as_str()) {
                    (kw_color, true)
                } else if ext == "py" && keywords_py.contains(&buf.as_str()) {
                    (kw_color, true)
                } else if lc == "true" || lc == "false" || lc == "null" || lc == "none" { // json/python null/booleans
                    (bool_color, true)
                } else if buf.chars().all(|c| c.is_ascii_digit()) {
                    (num_color, true)
                } else {
                    (base_color, false)
                };
                append_with_search(job, &buf, font_id.clone(), color, query, usize::MAX, &mut 0);
                buf.clear();
            }
            let color = match ch {
                '(' | '[' | '{' => {
                    let idx = ((*depth).max(0) as usize) % bracket_colors.len();
                    *depth = depth.saturating_add(1);
                    Some(bracket_colors[idx])
                }
                ')' | ']' | '}' => {
                    *depth = depth.saturating_sub(1);
                    let idx = ((*depth).max(0) as usize) % bracket_colors.len();
                    Some(bracket_colors[idx])
                }
                _ => None,
            };
            let delim = ch.to_string();
            append_with_search(job, &delim, font_id.clone(), color.unwrap_or(base_color), query, usize::MAX, &mut 0);
        }
    }
    if !buf.is_empty() {
        let lc = buf.to_ascii_lowercase();
        let (color, _) = if ext == "rs" && keywords_rs.contains(&buf.as_str()) {
            (kw_color, true)
        } else if ext == "py" && keywords_py.contains(&buf.as_str()) {
            (kw_color, true)
        } else if lc == "true" || lc == "false" || lc == "null" || lc == "none" {
            (bool_color, true)
        } else if buf.chars().all(|c| c.is_ascii_digit()) {
            (num_color, true)
        } else {
            (base_color, false)
        };
        append_with_search(job, &buf, font_id, color, query, usize::MAX, &mut 0);
    }
}

fn append_highlighted(
    job: &mut LayoutJob,
    line: &str,
    ext: &str,
    query: &str,
    font_id: egui::FontId,
    base_color: egui::Color32,
    do_syntax: bool,
    depth: &mut i32,
) {
    // Very lightweight tokenization: comments, strings; plus search highlight
    // Handle comment split first to avoid borrow issues with inner closure.
    if do_syntax {
        let comment_prefix = if ext == "rs" { "//" } else if ext == "toml" { "#" } else { "" };
        let comment_prefix = if ext == "py" { "#" } else { comment_prefix };
        if !comment_prefix.is_empty() {
            if let Some(pos) = line.find(comment_prefix) {
                append_highlighted(job, &line[..pos], "", query, font_id.clone(), base_color, do_syntax, depth);
                let fmt = egui::TextFormat { font_id: font_id.clone(), color: egui::Color32::GRAY, ..Default::default() };
                job.append(&line[pos..], 0.0, fmt);
                return;
            }
        }
    }

    let mut buf = String::new();

    // String literal coloring
    if do_syntax {
        let mut chars = line.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '"' {
                if !buf.is_empty() { token_highlight(job, &buf, ext, font_id.clone(), base_color, query, do_syntax, depth); buf.clear(); }
                buf.clear();
                let mut s = String::from('"');
                while let Some(c2) = chars.next() {
                    s.push(c2);
                    if c2 == '"' { break; }
                }
                append_with_search(job, &s, font_id.clone(), egui::Color32::from_rgb(152, 195, 121), query, usize::MAX, &mut 0);
            } else {
                buf.push(ch);
            }
        }
    } else {
        buf.push_str(line);
    }

    // Flush any remaining non-string content with token highlight
    if !buf.is_empty() {
        token_highlight(job, &buf, ext, font_id, base_color, query, do_syntax, depth);
    }
}
