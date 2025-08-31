use std::path::PathBuf;
use eframe::egui;

pub(crate) fn toolbar(ui: &mut egui::Ui, app: &mut crate::app::FileViewerApp, ctx: &egui::Context, file_to_load: &mut Option<PathBuf>) {
    use crate::io;
    use rfd::FileDialog;
    use egui::RichText;

    if ui
        .button(RichText::new("Open File"))
        .clicked()
        && let Some(path) = FileDialog::new()
            .add_filter("All Supported", &["txt","rs","py","toml","md","json","js","html","css","png","jpg","jpeg","gif","bmp","webp"])
            .add_filter("Images", &["png","jpg","jpeg","gif","bmp","webp"])
            .add_filter("Text/Source", &["txt","rs","py","toml","md","json","js","html","css"])
            .pick_file()
    {
        *file_to_load = Some(path);
    }

    ui.menu_button(RichText::new("Recent Files"), |ui| {
        ui.set_min_width(480.0);
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        if app.recent_files.is_empty() {
            ui.label("(empty)");
        }
        for file in app.recent_files.clone().into_iter().rev() {
            let display = file.to_string_lossy();
            if ui
                .button(egui::RichText::new(display.clone()).monospace())
                .on_hover_text(display)
                .clicked()
            {
                *file_to_load = Some(file);
                ui.close_menu();
            }
        }
        ui.separator();
        if ui.button("Clear Recent Files").clicked() {
            app.recent_files.clear();
            ui.close_menu();
        }
    });

    ui.separator();
    let prev_dark = app.dark_mode;
    let prev_lines = app.show_line_numbers;
    ui.checkbox(&mut app.dark_mode, "Dark Mode");
    ui.checkbox(&mut app.show_line_numbers, "Line Numbers");
    if app.dark_mode != prev_dark {
        app.apply_theme(ctx);
    }
    if app.dark_mode != prev_dark || app.show_line_numbers != prev_lines {
        crate::settings::save_settings_to_disk(app);
    }
    ui.separator();

    if ui.button("Clear").clicked() {
        app.content = None;
        app.current_path = None;
        app.error_message = None;
    }

    if matches!(app.content, Some(crate::app::Content::Image(_))) {
        ui.separator();
        let prev_fit = app.image_fit;
        ui.checkbox(&mut app.image_fit, "Fit to Window");
        if app.image_fit != prev_fit { crate::settings::save_settings_to_disk(app); }
        if ui.button("Zoom -").clicked() { app.image_fit = false; app.image_zoom = (app.image_zoom / 1.10).clamp(0.1, 6.0); }
        if ui.button("Zoom +").clicked() { app.image_fit = false; app.image_zoom = (app.image_zoom * 1.10).clamp(0.1, 6.0); }
        if ui.button("100%").clicked() { app.image_fit = false; app.image_zoom = 1.0; }
    }
}

pub(crate) fn search_bar(ui: &mut egui::Ui, app: &mut crate::app::FileViewerApp) {
    ui.horizontal_wrapped(|ui| {
        ui.label("Find:");
        let prev = app.search_query.clone();
        let resp = ui.text_edit_singleline(&mut app.search_query);
        if app.search_active {
            resp.request_focus();
            app.search_active = false;
        }
        // Enter / Shift+Enter navigate matches
        let (enter, shift) = ui.input(|i| (i.key_pressed(egui::Key::Enter), i.modifiers.shift));
        if enter && app.search_count > 0 {
            if shift {
                if app.search_current == 0 { app.search_current = app.search_count.saturating_sub(1); } else { app.search_current -= 1; }
            } else {
                app.search_current = (app.search_current + 1) % app.search_count;
            }
        }

        if resp.changed() || (prev.is_empty() && !app.search_query.is_empty()) {
            app.search_count = 0;
            app.search_current = 0;
            if let Some(crate::app::Content::Text(ref text)) = app.content {
                if !app.search_query.is_empty() && text.len() <= crate::app::HIGHLIGHT_CHAR_THRESHOLD {
                    app.search_count = crate::search::recompute_count(&app.search_query, text);
                }
            }
        }
        if !app.search_query.is_empty() {
            ui.label(format!("{} match(es)", app.search_count));
            ui.add_space(8.0);
            if ui.button("Prev").clicked() && app.search_count > 0 {
                if app.search_current == 0 { app.search_current = app.search_count.saturating_sub(1); } else { app.search_current -= 1; }
            }
            if ui.button("Next").clicked() && app.search_count > 0 {
                app.search_current = (app.search_current + 1) % app.search_count;
            }
            if app.search_count > 0 {
                ui.label(format!("{}/{}", app.search_current + 1, app.search_count));
            }
        }
    });
}

pub(crate) fn status_bar(ui: &mut egui::Ui, app: &mut crate::app::FileViewerApp) {
    use std::fs;
    ui.horizontal(|ui| {
        if let Some(path) = &app.current_path {
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
                { let _ = std::process::Command::new("explorer").arg(path).spawn(); }
                #[cfg(target_os = "macos")]
                { let _ = std::process::Command::new("open").arg("-R").arg(path).spawn(); }
                #[cfg(all(unix, not(target_os = "macos")))]
                { if let Some(parent) = path.parent() { let _ = std::process::Command::new("xdg-open").arg(parent).spawn(); } }
            }
        } else {
            ui.label("No file selected.");
        }
    });
}

pub(crate) fn status_extra(ui: &mut egui::Ui, app: &mut crate::app::FileViewerApp) {
    ui.horizontal(|ui| {
        match &app.content {
            Some(crate::app::Content::Image(texture)) => {
                let size = texture.size();
                ui.label(format!("Image: {}x{} px", size[0], size[1]));
                let eff = if app.image_fit { None } else { Some(app.image_zoom) };
                if let Some(z) = eff { ui.label(format!("Zoom: {:.0}%", z * 100.0)); }
                let est = (size[0] as usize).saturating_mul(size[1] as usize).saturating_mul(4);
                ui.label(format!("Texture ~{:.1} MB", est as f64 / (1024.0 * 1024.0)));
                if app.image_fit { ui.label("Fit: on"); }
            }
            Some(crate::app::Content::Text(_)) => {
                ui.label(format!("Lines: {}", app.text_line_count));
                ui.label(format!("Zoom: {:.0}%", app.text_zoom * 100.0));
                if app.text_is_big { ui.label("Large file: reduced features"); }
                if app.text_is_lossy { ui.label("UTF-8 (lossy)"); }
            }
            _ => {}
        }
    });
}

