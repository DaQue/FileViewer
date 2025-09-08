#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod app;
mod highlight;
mod search;
mod io;
mod settings;
mod ui;

use app::FileViewerApp;
use eframe::egui;

fn make_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_resizable(true)
            .with_title("Gemini File Viewer 2.1"),
        ..Default::default()
    }
}

fn select_backend_from_env() -> Option<&'static str> {
    let xdg_session = std::env::var("XDG_SESSION_TYPE").unwrap_or_default().to_lowercase();
    let has_wayland = std::env::var("WAYLAND_DISPLAY").map(|v| !v.is_empty()).unwrap_or(false);
    let has_x11 = std::env::var("DISPLAY").map(|v| !v.is_empty()).unwrap_or(false);

    if xdg_session == "wayland" && has_wayland {
        Some("wayland")
    } else if has_x11 {
        Some("x11")
    } else if has_wayland {
        Some("wayland")
    } else {
        Some("x11")
    }
}

fn configure_backend(backend: Option<&str>) {
    match backend {
        Some("x11") => {
            unsafe { std::env::set_var("WINIT_UNIX_BACKEND", "x11") };
            unsafe { std::env::remove_var("WAYLAND_DISPLAY") };
        }
        Some("wayland") => {
            unsafe { std::env::remove_var("WINIT_UNIX_BACKEND") };
        }
        _ => {}
    }
}

fn main() -> Result<(), eframe::Error> {
    let chosen = select_backend_from_env();
    configure_backend(chosen);

    let res = eframe::run_native(
        "Gemini File Viewer 2.1",
        make_options(),
        Box::new(|cc| Ok(Box::new(FileViewerApp::new(cc))))
    );

    if let Err(ref e) = res {
        eprintln!(
            "Failed to start GUI: {e}\nHints: if on Wayland, ensure a compositor and xdg-desktop-portal are running; or try 'WINIT_UNIX_BACKEND=x11'."
        );
    }

    res
}
