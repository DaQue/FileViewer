use std::fs;
use std::path::PathBuf;

pub(crate) fn settings_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "gemini-file-viewer")
        .map(|dirs| dirs.config_dir().join("settings.json"))
}

pub(crate) fn load_settings_from_disk() -> Option<crate::app::FileViewerApp> {
    let path = settings_path()?;
    let data = fs::read(&path).ok()?;
    serde_json::from_slice::<crate::app::FileViewerApp>(&data).ok()
}

pub(crate) fn save_settings_to_disk(app: &crate::app::FileViewerApp) {
    if let Some(path) = settings_path() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(s) = serde_json::to_vec_pretty(app) {
            let _ = fs::write(path, s);
        }
    }
}

