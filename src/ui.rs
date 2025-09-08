use std::path::PathBuf;
use eframe::egui;
use eframe::egui::Stroke;

pub(crate) fn toolbar(ui: &mut egui::Ui, app: &mut crate::app::FileViewerApp, ctx: &egui::Context, file_to_load: &mut Option<PathBuf>) {
    use rfd::FileDialog;
    use egui::RichText;

    // Rainbow helpers (active only for Allison theme)
    let is_allison = matches!(app.theme, crate::app::Theme::Allison);
    let mut rainbow_idx: usize = 0;
    let mut next_color = |idx: &mut usize| {
        let palette = [
            egui::Color32::from_rgb(239, 83, 80),   // red
            egui::Color32::from_rgb(255, 167, 38),  // orange
            egui::Color32::from_rgb(255, 238, 88),  // yellow
            egui::Color32::from_rgb(102, 187, 106), // green
            egui::Color32::from_rgb(66, 165, 245),  // blue
            egui::Color32::from_rgb(126, 87, 194),  // indigo
            egui::Color32::from_rgb(171, 71, 188),  // violet
        ];
        let c = palette[*idx % palette.len()];
        *idx += 1;
        c
    };
    let mut rainbow_button = |ui: &mut egui::Ui, label: &str, idx: &mut usize| {
        let bg = next_color(idx);
        let text_color = if bg == egui::Color32::from_rgb(255, 238, 88) { egui::Color32::BLACK } else { egui::Color32::WHITE };
        ui.add(egui::Button::new(RichText::new(label).strong().color(text_color)).fill(bg).stroke(Stroke::new(1.0, bg.gamma_multiply(0.5))))
    };

    if (if is_allison { rainbow_button(ui, "📂 Open", &mut rainbow_idx) } else { ui.button(RichText::new("📂 Open").strong()) })
        .on_hover_text("Open a file (Ctrl+O)")
        .clicked()
        && let Some(path) = FileDialog::new()
            .add_filter("All Supported", &["txt","rs","py","toml","md","json","js","html","css","png","jpg","jpeg","gif","bmp","webp"])
            .add_filter("Images", &["png","jpg","jpeg","gif","bmp","webp"])
            .add_filter("Text/Source", &["txt","rs","py","toml","md","json","js","html","css"])
            .pick_file()
    {
        *file_to_load = Some(path);
    }

    if is_allison {
        let bg = next_color(&mut rainbow_idx);
        let text_color = if bg == egui::Color32::from_rgb(255, 238, 88) { egui::Color32::BLACK } else { egui::Color32::WHITE };
        let id = ui.make_persistent_id("recent_menu");
        let resp = ui.add(egui::Button::new(egui::RichText::new("🕘 Recent").strong().color(text_color)).fill(bg).stroke(Stroke::new(1.0, bg.gamma_multiply(0.5))));
        if resp.clicked() { ui.memory_mut(|m| m.toggle_popup(id)); }
        egui::popup::popup_below_widget(
            ui,
            id,
            &resp,
            egui::popup::PopupCloseBehavior::CloseOnClickOutside,
            |ui: &mut egui::Ui| {
            ui.set_min_width(480.0);
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            if app.recent_files.is_empty() { ui.label("(empty)"); }
            for file in app.recent_files.clone().into_iter().rev() {
                let name = file.file_name().and_then(|s| s.to_str()).unwrap_or("(unknown)");
                let parent = file.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                let btn = egui::RichText::new(name).strong();
                if ui.button(btn).on_hover_text(parent.clone()).clicked() { *file_to_load = Some(file); ui.memory_mut(|m| m.close_popup()); }
                if !parent.is_empty() { ui.label(egui::RichText::new(parent).weak().small()); }
            }
            ui.separator();
            if ui.button("🧹 Clear Recent").clicked() { app.recent_files.clear(); ui.memory_mut(|m| m.close_popup()); }
            }
        );
    } else {
        ui.menu_button(egui::RichText::new("🕘 Recent").strong(), |ui| {
            ui.set_min_width(480.0);
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            if app.recent_files.is_empty() { ui.label("(empty)"); }
            for file in app.recent_files.clone().into_iter().rev() {
                let name = file.file_name().and_then(|s| s.to_str()).unwrap_or("(unknown)");
                let parent = file.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                let btn = egui::RichText::new(name).strong();
                if ui.button(btn).on_hover_text(parent.clone()).clicked() { *file_to_load = Some(file); ui.close_menu(); }
                if !parent.is_empty() { ui.label(egui::RichText::new(parent).weak().small()); }
            }
            ui.separator();
            if ui.button("🧹 Clear Recent").clicked() { app.recent_files.clear(); ui.close_menu(); }
        });
    }

    ui.separator();
    let prev_dark = app.dark_mode;
    let prev_lines = app.show_line_numbers;
    // Theme selector (colored background in Allison)
    if is_allison {
        let bg = next_color(&mut rainbow_idx);
        let text_color = if bg == egui::Color32::from_rgb(255, 238, 88) { egui::Color32::BLACK } else { egui::Color32::WHITE };
        ui.scope(|ui| {
            let mut style = (*ctx.style()).clone();
            let mut visuals = style.visuals.clone();
            visuals.widgets.inactive.bg_fill = bg;
            visuals.widgets.hovered.bg_fill = bg.gamma_multiply(1.05);
            visuals.widgets.active.bg_fill = bg.gamma_multiply(0.95);
            visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, bg.gamma_multiply(0.5));
            visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, bg.gamma_multiply(0.6));
            visuals.widgets.active.fg_stroke = Stroke::new(1.0, bg.gamma_multiply(0.6));
            style.visuals = visuals;
            ui.set_style(style);
            let mut theme_changed = false;
            egui::ComboBox::from_id_source("theme_combo")
                .selected_text(egui::RichText::new(format!("🎨 {}", app.theme.name())).color(text_color))
                .show_ui(ui, |ui| {
                    use crate::app::Theme;
                    theme_changed |= ui.selectable_value(&mut app.theme, Theme::Light, "Light").changed();
                    theme_changed |= ui.selectable_value(&mut app.theme, Theme::Dark, "Dark").changed();
                    theme_changed |= ui.selectable_value(&mut app.theme, Theme::Allison, "Allison").changed();
                    theme_changed |= ui.selectable_value(&mut app.theme, Theme::SolarizedLight, "Solarized Light").changed();
                    theme_changed |= ui.selectable_value(&mut app.theme, Theme::SolarizedDark, "Solarized Dark").changed();
                    theme_changed |= ui.selectable_value(&mut app.theme, Theme::Dracula, "Dracula").changed();
                    theme_changed |= ui.selectable_value(&mut app.theme, Theme::GruvboxDark, "Gruvbox Dark").changed();
                    theme_changed |= ui.selectable_value(&mut app.theme, Theme::Sepia, "Sepia").changed();
                });
            if theme_changed { app.follow_system_theme = false; }
            ui.add_space(6.0);
            ui.checkbox(&mut app.follow_system_theme, egui::RichText::new("Follow system").color(text_color));
            ui.add_space(6.0);
            if ui.add(egui::Button::new(egui::RichText::new("🎛 Theme").strong().color(text_color)).fill(bg).stroke(Stroke::new(1.0, bg.gamma_multiply(0.5)))).on_hover_text("Open Theme Editor").clicked() {
                app.show_theme_editor = true;
            }
        });
    } else {
        let mut theme_changed = false;
        egui::ComboBox::from_id_source("theme_combo")
            .selected_text(format!("🎨 {}", app.theme.name()))
            .show_ui(ui, |ui| {
                use crate::app::Theme;
                theme_changed |= ui.selectable_value(&mut app.theme, Theme::Light, "Light").changed();
                theme_changed |= ui.selectable_value(&mut app.theme, Theme::Dark, "Dark").changed();
                theme_changed |= ui.selectable_value(&mut app.theme, Theme::Allison, "Allison").changed();
                theme_changed |= ui.selectable_value(&mut app.theme, Theme::SolarizedLight, "Solarized Light").changed();
                theme_changed |= ui.selectable_value(&mut app.theme, Theme::SolarizedDark, "Solarized Dark").changed();
                theme_changed |= ui.selectable_value(&mut app.theme, Theme::Dracula, "Dracula").changed();
                theme_changed |= ui.selectable_value(&mut app.theme, Theme::GruvboxDark, "Gruvbox Dark").changed();
                theme_changed |= ui.selectable_value(&mut app.theme, Theme::Sepia, "Sepia").changed();
            });
        if theme_changed { app.follow_system_theme = false; }
        ui.checkbox(&mut app.follow_system_theme, "Follow system");
        if ui.button("🎛 Theme").on_hover_text("Open Theme Editor").clicked() { app.show_theme_editor = true; }
    }
    // Accent picker removed per request

    // Always hide Dark Mode (since Light/Dark themes exist)
    // Line Numbers with its own background color in Allison
    if is_allison {
        let bg = next_color(&mut rainbow_idx);
        let text_color = if bg == egui::Color32::from_rgb(255, 238, 88) { egui::Color32::BLACK } else { egui::Color32::WHITE };
        egui::Frame::default().fill(bg).stroke(Stroke::new(1.0, bg.gamma_multiply(0.5))).show(ui, |ui| {
            let before = app.show_line_numbers;
            if ui.checkbox(&mut app.show_line_numbers, "").on_hover_text("Toggle line numbers (Ctrl+L)").changed() {
                if app.show_line_numbers != before { crate::settings::save_settings_to_disk(app); }
            }
            ui.label(egui::RichText::new("Line Numbers").color(text_color));
        });
    } else {
        ui.checkbox(&mut app.show_line_numbers, "Line Numbers").on_hover_text("Toggle line numbers (Ctrl+L)");
    }
    if app.dark_mode != prev_dark {
        // Keep theme synced with quick toggle
        app.theme = if app.dark_mode { crate::app::Theme::Dark } else { crate::app::Theme::Light };
        app.apply_theme(ctx);
    }
    if app.dark_mode != prev_dark || app.show_line_numbers != prev_lines {
        crate::settings::save_settings_to_disk(app);
    }
    // Applying selected theme if changed via combobox
    ui.ctx().style_mut(|_| {}); // force borrow split
    if app.dark_mode != app.theme.is_dark() {
        app.dark_mode = app.theme.is_dark();
        app.apply_theme(ctx);
        crate::settings::save_settings_to_disk(app);
    }
    ui.separator();

    if (if is_allison { rainbow_button(ui, "🧹 Clear", &mut rainbow_idx) } else { ui.button("🗑️ Clear") }).on_hover_text("Clear current view").clicked() {
        app.content = None;
        app.current_path = None;
        app.error_message = None;
    }

    if matches!(app.content, Some(crate::app::Content::Image(_))) {
        ui.separator();
        let prev_fit = app.image_fit;
        if let Some(cur) = app.current_path.clone() {
            if (if is_allison { rainbow_button(ui, "Prev", &mut rainbow_idx) } else { ui.button("Prev") }).clicked() {
                if let Some(prev) = crate::io::neighbor_image(&cur, false) {
                    *file_to_load = Some(prev);
                }
            }
            if (if is_allison { rainbow_button(ui, "Next", &mut rainbow_idx) } else { ui.button("Next") }).clicked() {
                if let Some(next) = crate::io::neighbor_image(&cur, true) {
                    *file_to_load = Some(next);
                }
            }
            ui.separator();
        }
        ui.checkbox(&mut app.image_fit, "Fit to Window").on_hover_text("Scale image to fit the window");
        if app.image_fit != prev_fit { crate::settings::save_settings_to_disk(app); }
        if (if is_allison { rainbow_button(ui, "🔍−", &mut rainbow_idx) } else { ui.button("🔍−") }).on_hover_text("Zoom out").clicked() { app.image_fit = false; app.image_zoom = (app.image_zoom / 1.10).clamp(0.1, 6.0); }
        if (if is_allison { rainbow_button(ui, "🔍+", &mut rainbow_idx) } else { ui.button("🔍+") }).on_hover_text("Zoom in").clicked() { app.image_fit = false; app.image_zoom = (app.image_zoom * 1.10).clamp(0.1, 6.0); }
        if (if is_allison { rainbow_button(ui, "100%", &mut rainbow_idx) } else { ui.button("100%") }).on_hover_text("Reset zoom").clicked() { app.image_fit = false; app.image_zoom = 1.0; }
    } else if matches!(app.content, Some(crate::app::Content::Text(_))) {
        if let Some(cur) = app.current_path.clone() {
            ui.separator();
            if (if is_allison { rainbow_button(ui, "Prev", &mut rainbow_idx) } else { ui.button("Prev") }).clicked() {
                if let Some(prev) = crate::io::neighbor_text(&cur, false) { *file_to_load = Some(prev); }
            }
            if (if is_allison { rainbow_button(ui, "Next", &mut rainbow_idx) } else { ui.button("Next") }).clicked() {
                if let Some(next) = crate::io::neighbor_text(&cur, true) { *file_to_load = Some(next); }
            }
        }
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
            ui.monospace(format!("📄 {}", path.to_string_lossy()));
            if let Ok(metadata) = fs::metadata(path) {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("({:.1} KB)", metadata.len() as f64 / 1024.0));
                });
            }
            if ui.button("📋 Copy Path").on_hover_text("Copy path to clipboard").clicked() {
                ui.ctx().copy_text(path.to_string_lossy().into());
            }
            if ui.button("📂 Open Folder").clicked() {
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
                ui.label(format!("🖼️ {}x{} px", size[0], size[1]));
                let eff = if app.image_fit { None } else { Some(app.image_zoom) };
                if let Some(z) = eff { ui.label(format!("🔍 {:.0}%", z * 100.0)); }
                let est = (size[0] as usize).saturating_mul(size[1] as usize).saturating_mul(4);
                ui.label(format!("🧮 ~{:.1} MB", est as f64 / (1024.0 * 1024.0)));
                if app.image_fit { ui.label("Fit: on"); }
            }
            Some(crate::app::Content::Text(_)) => {
                ui.label(format!("📄 Lines: {}", app.text_line_count));
                ui.label(format!("🔍 {:.0}%", app.text_zoom * 100.0));
                if app.text_is_big { ui.label("⚠️ Large file: reduced features"); }
                if app.text_is_lossy { ui.label("ℹ️ UTF-8 (lossy)"); }
            }
            _ => {}
        }
    });
}
