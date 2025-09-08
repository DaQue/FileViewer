# Gemini File Viewer

A lightweight desktop viewer for text/code and images, built with egui/eframe.

Project status
- This is the master crate for all platforms (Linux and Windows). The older `gemini-file-viewer-linux` crate remains temporarily for reference and will be retired after Windows validation.

Builds
- See `BUILDING.md` for Linux builds and Windows cross‑compile (MSVC and GNU) instructions.

## Highlights
- Persistent settings: Remembers Dark Mode, Line Numbers, and Recent Files across runs.
- Recent Files: Wide, non-wrapping menu with a Clear option.
- Image formats: PNG, JPEG, GIF, BMP, WEBP (scaled smoothly).
- Text view: Optional line numbers for code-like files.

## Usage
- Open files: Use "Open File..." or pick from "Recent Files".
- Toggles: Dark Mode and Line Numbers in the top toolbar.
- Clear: Resets the current view without changing settings.

## Persistence Details
- Settings are saved immediately when toggles or recents change and again on exit.
- Location:
  - Windows: %APPDATA%/gemini-file-viewer/settings.json
  - macOS: ~/Library/Application Support/gemini-file-viewer/settings.json
  - Linux: ~/.config/gemini-file-viewer/settings.json

## Build
- Rust 1.89+ recommended.
- Release build: `cargo build --release`
- Binary: `target/release/gemini-file-viewer` (or `.exe` on Windows)

## Notes
- The Windows file dialog follows the OS setting for showing file extensions. Filters include extensions explicitly.
