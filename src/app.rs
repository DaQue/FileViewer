use eframe::egui;
use crate::highlight;
use crate::search;
use egui::{text::LayoutJob, RichText, TextureHandle};
use rfd::FileDialog;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_FILE_SIZE_BYTES: u64 = 10_000_000; // 10MB
const MAX_RECENT_FILES: usize = 10;
const BIG_TEXT_CHAR_THRESHOLD: usize = 500_000; // Disable heavy features beyond this
pub(crate) const HIGHLIGHT_CHAR_THRESHOLD: usize = 200_000; // Disable syntax/mark highlights beyond this

pub enum Content {
    Text(String),
    Image(TextureHandle),
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
    SolarizedLight,
    SolarizedDark,
    Dracula,
    GruvboxDark,
    Sepia,
    Allison,
}

impl Default for Theme { fn default() -> Self { Theme::Dark } }

impl Theme {
    pub fn is_dark(self) -> bool {
        matches!(self, Theme::Dark | Theme::SolarizedDark | Theme::Dracula | Theme::GruvboxDark | Theme::Allison)
    }
    pub fn name(self) -> &'static str {
        match self {
            Theme::Light => "Light",
            Theme::Dark => "Dark",
            Theme::SolarizedLight => "Solarized Light",
            Theme::SolarizedDark => "Solarized Dark",
            Theme::Dracula => "Dracula",
            Theme::GruvboxDark => "Gruvbox Dark",
            Theme::Sepia => "Sepia",
            Theme::Allison => "Allison",
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct FileViewerApp {
    #[serde(skip)]
    pub(crate) content: Option<Content>,
    #[serde(skip)]
    pub(crate) current_path: Option<PathBuf>,
    #[serde(skip)]
    pub(crate) error_message: Option<String>,
    pub(crate) dark_mode: bool,
    pub(crate) theme: Theme,
    #[serde(default = "default_follow_system_true")]
    pub(crate) follow_system_theme: bool,
    pub(crate) recent_files: Vec<PathBuf>,
    pub(crate) show_line_numbers: bool,
    pub(crate) word_wrap: bool,
    pub(crate) text_zoom: f32,
    pub(crate) image_zoom: f32,
    #[serde(skip)]
    pub(crate) show_about: bool,
    pub(crate) image_fit: bool,
    pub(crate) accent_rgb: [u8; 3],
    #[serde(default = "default_spacing_scale")]
    pub(crate) spacing_scale: f32,
    #[serde(default = "default_rounding")]
    pub(crate) theme_rounding: u8,
    #[serde(skip)]
    pub(crate) show_theme_editor: bool,
    // Derived/runtime-only state for text rendering
    #[serde(skip)]
    pub(crate) text_is_big: bool,
    #[serde(skip)]
    pub(crate) text_line_count: usize,
    #[serde(skip)]
    pub(crate) text_is_lossy: bool,
    // Simple find state
    #[serde(skip)]
    pub(crate) search_query: String,
    #[serde(skip)]
    pub(crate) search_active: bool,
    #[serde(skip)]
    pub(crate) search_count: usize,
    #[serde(skip)]
    pub(crate) search_current: usize,
}

impl FileViewerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load custom fonts if present (from Allison fork)
        load_custom_fonts(&cc.egui_ctx);
        if let Some(storage) = cc.storage
            && let Some(s) = storage.get_string(eframe::APP_KEY)
            && let Ok(mut app) = serde_json::from_str::<FileViewerApp>(&s)
        {
            app.text_is_big = false;
            app.text_line_count = 0;
            app.text_is_lossy = false;
            app.search_query = String::new();
            app.search_active = false;
            app.search_count = 0;
            if app.dark_mode != app.theme.is_dark() {
                app.theme = if app.dark_mode { Theme::Dark } else { Theme::Light };
            }
            if app.spacing_scale <= 0.0 { app.spacing_scale = default_spacing_scale(); }
            if app.theme_rounding == 0 { app.theme_rounding = default_rounding(); }
            return app;
        }
        if let Some(mut app) = crate::settings::load_settings_from_disk() {
            app.text_is_big = false;
            app.text_line_count = 0;
            app.text_is_lossy = false;
            app.search_query = String::new();
            app.search_active = false;
            app.search_count = 0;
            if app.dark_mode != app.theme.is_dark() {
                app.theme = if app.dark_mode { Theme::Dark } else { Theme::Light };
            }
            if app.spacing_scale <= 0.0 { app.spacing_scale = default_spacing_scale(); }
            if app.theme_rounding == 0 { app.theme_rounding = default_rounding(); }
            return app;
        }
        Default::default()
    }

    pub(crate) fn apply_theme(&self, ctx: &egui::Context) {
        let mut visuals = if self.theme.is_dark() { egui::Visuals::dark() } else { egui::Visuals::light() };

        // Accent color override
        let accent = egui::Color32::from_rgb(self.accent_rgb[0], self.accent_rgb[1], self.accent_rgb[2]);
        visuals.selection.bg_fill = accent;
        visuals.hyperlink_color = accent;

        // Panel fills by theme
        visuals.panel_fill = match self.theme {
            Theme::Light => egui::Color32::from_rgb(247, 247, 249),
            Theme::Dark => egui::Color32::from_rgb(22, 22, 24),
            Theme::SolarizedLight => egui::Color32::from_rgb(253, 246, 227),
            Theme::SolarizedDark => egui::Color32::from_rgb(0, 43, 54),
            Theme::Dracula => egui::Color32::from_rgb(30, 31, 41),
            Theme::GruvboxDark => egui::Color32::from_rgb(40, 40, 40),
            Theme::Sepia => egui::Color32::from_rgb(247, 242, 231),
            Theme::Allison => egui::Color32::from_rgb(24, 26, 30),
        };

        let mut style = (*ctx.style()).clone();
        let s = self.spacing_scale.max(0.5).min(2.0);
        style.spacing.item_spacing = egui::vec2(8.0 * s, 6.0 * s);
        style.spacing.button_padding = egui::vec2(10.0 * s, 6.0 * s);
        style.spacing.interact_size = egui::vec2(36.0 * s, 28.0 * s);
        let wm_x: i8 = (12.0 * s).round() as i8;
        let wm_y: i8 = (8.0 * s).round() as i8;
        style.spacing.window_margin = egui::Margin::symmetric(wm_x, wm_y);
        // Global rounding is more limited in egui 0.31; skip if not available
        style.visuals = visuals;
        ctx.set_style(style);
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

        let loaded = if crate::io::is_supported_image(&path) {
            match crate::io::load_image(&path) {
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
            match crate::io::load_text(&path) {
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
                self.recent_files.retain(|p| p != &path);
                self.recent_files.push(path);
                if self.recent_files.len() > MAX_RECENT_FILES {
                    let overflow = self.recent_files.len() - MAX_RECENT_FILES;
                    self.recent_files.drain(0..overflow);
                }
                crate::settings::save_settings_to_disk(self);
            }
            Err(e) => self.error_message = Some(e),
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
            theme: Theme::Dark,
            follow_system_theme: true,
            recent_files: Vec::new(),
            show_line_numbers: true,
            word_wrap: true,
            text_zoom: 1.0,
            image_zoom: 1.0,
            show_about: false,
            image_fit: false,
            accent_rgb: [93, 156, 255],
            spacing_scale: 1.0,
            theme_rounding: 6,
            show_theme_editor: false,
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
        crate::settings::save_settings_to_disk(self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Follow system theme if enabled
        if self.follow_system_theme {
            let sys_dark = matches!(dark_light::detect(), Ok(dark_light::Mode::Dark));
            if sys_dark != self.dark_mode {
                self.dark_mode = sys_dark;
                self.theme = if self.dark_mode { Theme::Dark } else { Theme::Light };
            }
        }
        // Apply visuals each frame
        self.apply_theme(ctx);

        let mut file_to_load: Option<PathBuf> = None;

        // Drag & Drop: preview and open files
        let hovered = ctx.input(|i| i.raw.hovered_files.clone());
        if !hovered.is_empty() {
            egui::Area::new("drop_hint".into())
                .fixed_pos(egui::pos2(20.0, 20.0))
                .order(egui::Order::Tooltip)
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style())
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Drop to open file").strong());
                            if let Some(path) = hovered[0].path.as_ref() {
                                ui.monospace(path.to_string_lossy());
                            }
                        });
                });
        }
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if let Some(df) = dropped.first() {
            if let Some(path) = df.path.clone() {
                file_to_load = Some(path);
            }
        }

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
                crate::settings::save_settings_to_disk(self);
            }
            if i.modifiers.command && i.key_pressed(egui::Key::W) {
                self.word_wrap = !self.word_wrap;
                crate::settings::save_settings_to_disk(self);
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

            // Navigation with arrow keys for current content type
            if i.key_pressed(egui::Key::ArrowRight) {
                if let Some(cur) = self.current_path.clone() {
                    match self.content {
                        Some(Content::Image(_)) => {
                            if let Some(next) = crate::io::neighbor_image(&cur, true) { file_to_load = Some(next); }
                        }
                        Some(Content::Text(_)) => {
                            if let Some(next) = crate::io::neighbor_text(&cur, true) { file_to_load = Some(next); }
                        }
                        _ => {}
                    }
                }
            }
            if i.key_pressed(egui::Key::ArrowLeft) {
                if let Some(cur) = self.current_path.clone() {
                    match self.content {
                        Some(Content::Image(_)) => {
                            if let Some(prev) = crate::io::neighbor_image(&cur, false) { file_to_load = Some(prev); }
                        }
                        Some(Content::Text(_)) => {
                            if let Some(prev) = crate::io::neighbor_text(&cur, false) { file_to_load = Some(prev); }
                        }
                        _ => {}
                    }
                }
            }
            // Support '<' and '>' typed keys for both images and text
            for ev in &i.events {
                if let egui::Event::Text(t) = ev {
                    if t == ">" {
                        if let Some(cur) = self.current_path.clone() {
                            match self.content {
                                Some(Content::Image(_)) => { if let Some(next) = crate::io::neighbor_image(&cur, true) { file_to_load = Some(next); } }
                                Some(Content::Text(_)) => { if let Some(next) = crate::io::neighbor_text(&cur, true) { file_to_load = Some(next); } }
                                _ => {}
                            }
                        }
                    } else if t == "<" {
                        if let Some(cur) = self.current_path.clone() {
                            match self.content {
                                Some(Content::Image(_)) => { if let Some(prev) = crate::io::neighbor_image(&cur, false) { file_to_load = Some(prev); } }
                                Some(Content::Text(_)) => { if let Some(prev) = crate::io::neighbor_text(&cur, false) { file_to_load = Some(prev); } }
                                _ => {}
                            }
                        }
                    }
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
                    ui.label(RichText::new("Gemini File Viewer 2.1").strong());
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
            self.theme = if self.dark_mode { Theme::Dark } else { Theme::Light };
            self.follow_system_theme = false; // manual override
            self.apply_theme(ctx);
            crate::settings::save_settings_to_disk(self);
        }

        // Top Toolbar
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                crate::ui::toolbar(ui, self, ctx, &mut file_to_load);
            });
        });

        // Search Bar (only when viewing text)
        if matches!(self.content, Some(Content::Text(_))) {
            egui::TopBottomPanel::top("searchbar").show(ctx, |ui| {
                crate::ui::search_bar(ui, self);
            });
        }

        // Status Bar
        egui::TopBottomPanel::bottom("statusbar").show(ctx, |ui| {
            crate::ui::status_bar(ui, self);
        });

        // Extra status information
        egui::TopBottomPanel::bottom("status-extra").show(ctx, |ui| {
            crate::ui::status_extra(ui, self);
        });

        // Theme Editor window
        if self.show_theme_editor {
            let mut open = self.show_theme_editor;
            egui::Window::new("Theme Editor").open(&mut open).resizable(false).show(ctx, |ui| {
                ui.heading("Theme & Layout");
                ui.separator();
                ui.checkbox(&mut self.follow_system_theme, "Follow system light/dark");
                ui.horizontal(|ui| {
                    ui.label("Accent color:");
                    let mut srgba = egui::Color32::from_rgb(self.accent_rgb[0], self.accent_rgb[1], self.accent_rgb[2]);
                    if ui.color_edit_button_srgba(&mut srgba).changed() {
                        self.accent_rgb = [srgba.r(), srgba.g(), srgba.b()];
                    }
                });
                ui.add(egui::Slider::new(&mut self.spacing_scale, 0.6..=1.6).text("Spacing scale"));
                ui.add(egui::Slider::new(&mut self.theme_rounding, 0..=12).text("Corner radius"));
                ui.label("Close this window using the × in the title bar.");
            });
            self.show_theme_editor = open;
        }

        // Main Content
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(err) = &self.error_message {
                ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
            }

            if let Some(content) = &self.content {
                match content {
                    Content::Text(text) => {
                        let mut frame = egui::Frame::group(ui.style());
                        frame.fill = if self.dark_mode { egui::Color32::from_rgb(28, 28, 30) } else { egui::Color32::from_rgb(255, 255, 255) };
                        frame.inner_margin = egui::Margin::symmetric(12, 10);
                        frame = frame.corner_radius(egui::CornerRadius::same(8));
                        frame.show(ui, |ui| {
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
                                    let mut bracket_depth: i32 = 0;
                                    let mut in_block_comment = false;
                                    let ext = self
                                        .current_path
                                        .as_ref()
                                        .and_then(|p| p.extension().and_then(|s| s.to_str()))
                                        .unwrap_or("")
                                        .to_lowercase();
                                    // Determine target line for current match
                                    let target_line = if !self.search_query.is_empty() && self.search_count > 0 {
                                        search::find_target_line(text, &self.search_query, self.search_current)
                                    } else { None };
                                    // Render per line and capture rect
                                    let mut counter: usize = 0;
                                    let mut target_rect: Option<egui::Rect> = None;
                                    for (i, line) in text.lines().enumerate() {
                                        let mut line_job = LayoutJob::default();
                                        if do_line_numbers {
                                            line_job.append(&format!("{:>4} ", i + 1), 0.0, egui::TextFormat { font_id: font_id.clone(), color: egui::Color32::GRAY, ..Default::default() });
                                        }
                                        highlight::append_highlighted(&mut line_job, line, &ext, &self.search_query, font_id.clone(), text_color, do_highlight, &mut bracket_depth, self.search_current, &mut counter, &mut in_block_comment);
                                        let resp = ui.label(line_job);
                                        if target_line == Some(i) { target_rect = Some(resp.rect); }
                                    }
                                    if let Some(rect) = target_rect { ui.scroll_to_rect(rect, Some(egui::Align::Center)); }
                                } else {
                                    ui.label(RichText::new(text).monospace().size(font_id.size));
                                }
                            });
                        });
                    }
                    Content::Image(texture) => {
                        let viewport = ui.available_size();
                        // Checkerboard background
                        let rect = ui.max_rect();
                        let painter = ui.painter_at(rect);
                        let size_cell = 12.0;
                        let c1 = if ui.visuals().dark_mode { egui::Color32::from_gray(48) } else { egui::Color32::from_gray(220) };
                        let c2 = if ui.visuals().dark_mode { egui::Color32::from_gray(60) } else { egui::Color32::from_gray(235) };
                        let mut y = rect.top();
                        let mut row = 0;
                        while y < rect.bottom() {
                            let mut x = rect.left();
                            let mut col = 0;
                            while x < rect.right() {
                                let r = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(size_cell, size_cell));
                                let color = if (row + col) % 2 == 0 { c1 } else { c2 };
                                painter.rect_filled(r, 0.0, color);
                                x += size_cell;
                                col += 1;
                            }
                            y += size_cell;
                            row += 1;
                        }
                        egui::ScrollArea::both().show(ui, |ui| {
                            ui.centered_and_justified(|ui| {
                                let size = texture.size();
                                let mut effective_zoom = self.image_zoom;
                                if self.image_fit {
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
                    use egui::RichText as RT;
                    ui.add_space(ui.available_height() * 0.20);
                    ui.label(RT::new("🪐").size(48.0));
                    ui.add_space(8.0);
                    ui.label(RT::new("Gemini File Viewer").heading());
                    ui.add_space(4.0);
                    ui.label("Open a file to get started.");
                    ui.add_space(12.0);
                    if ui.add(egui::Button::new("📂 Open a file (Ctrl+O)").min_size(egui::vec2(220.0, 36.0))).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("All Supported", &["txt","rs","py","toml","md","json","js","html","css","png","jpg","jpeg","gif","bmp","webp"])
                            .add_filter("Images", &["png","jpg","jpeg","gif","bmp","webp"])
                            .add_filter("Text/Source", &["txt","rs","py","toml","md","json","js","html","css"])
                            .pick_file()
                        {
                            file_to_load = Some(path);
                        }
                    }
                });
            }
        });

        // Deferred file loading to avoid borrow issues
        if let Some(path) = file_to_load {
            self.load_file(path, ctx);
        }
    }
}

fn try_read(path: &Path) -> Option<Vec<u8>> { std::fs::read(path).ok() }

fn load_custom_fonts(ctx: &egui::Context) {
    use egui::{FontData, FontDefinitions, FontFamily};
    let mut fonts = FontDefinitions::default();

    // Candidate roots: CWD and executable dir
    let mut roots: Vec<std::path::PathBuf> = vec![std::path::PathBuf::from(".")];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() { roots.push(dir.to_path_buf()); }
    }

    let font_paths = [
        ("Inter-Regular", "assets/fonts/Inter-Regular.ttf"),
        ("Inter-Medium", "assets/fonts/Inter-Medium.ttf"),
        ("Inter-SemiBold", "assets/fonts/Inter-SemiBold.ttf"),
        ("JetBrainsMono-Regular", "assets/fonts/JetBrainsMono-Regular.ttf"),
        ("JetBrainsMono-Bold", "assets/fonts/JetBrainsMono-Bold.ttf"),
    ];

    for (key, rel) in font_paths {
        let mut loaded: Option<Vec<u8>> = None;
        for root in &roots {
            let p = root.join(rel);
            if let Some(bytes) = try_read(&p) {
                loaded = Some(bytes);
                break;
            }
        }
        if let Some(bytes) = loaded {
            fonts.font_data.insert(key.to_string(), FontData::from_owned(bytes).into());
        }
    }

    // Prefer Inter for proportional text if present
    if fonts.font_data.contains_key("Inter-Medium") || fonts.font_data.contains_key("Inter-Regular") {
        let family = fonts.families.entry(FontFamily::Proportional).or_default();
        if fonts.font_data.contains_key("Inter-SemiBold") { family.insert(0, "Inter-SemiBold".to_owned()); }
        if fonts.font_data.contains_key("Inter-Medium") { family.insert(0, "Inter-Medium".to_owned()); }
        if fonts.font_data.contains_key("Inter-Regular") { family.insert(0, "Inter-Regular".to_owned()); }
    }

    // Prefer JetBrains Mono for monospace if present
    if fonts.font_data.contains_key("JetBrainsMono-Regular") {
        let family = fonts.families.entry(FontFamily::Monospace).or_default();
        if fonts.font_data.contains_key("JetBrainsMono-Bold") { family.insert(0, "JetBrainsMono-Bold".to_owned()); }
        family.insert(0, "JetBrainsMono-Regular".to_owned());
    }

    ctx.set_fonts(fonts);
}

fn default_follow_system_true() -> bool { true }
fn default_spacing_scale() -> f32 { 1.0 }
fn default_rounding() -> u8 { 6 }
