//! User preferences, saved as JSON in the app's config folder.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// After a successful conversion, move the original file to the Recycle Bin.
    pub delete_original: bool,
    /// Pressing Shift while dragging a file opens the wheel.
    pub gesture_enabled: bool,
    /// Target ids shown on the wheel, per category key ("image", "video", ...), in wheel order.
    /// A missing category uses the built-in defaults.
    pub wheel_formats: HashMap<String, Vec<String>>,
    /// "vi", "en", or "auto" to follow the Windows display language.
    pub language: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            delete_original: false,
            gesture_enabled: true,
            wheel_formats: HashMap::new(),
            language: "auto".into(),
        }
    }
}

pub struct SettingsState(Mutex<Settings>);

fn path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("settings.json"))
}

impl SettingsState {
    pub fn load(app: &AppHandle) -> Self {
        let settings = path(app)
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Self(Mutex::new(settings))
    }

    pub fn get(&self) -> Settings {
        self.0.lock().unwrap().clone()
    }

    pub fn update(&self, app: &AppHandle, change: impl FnOnce(&mut Settings)) -> Settings {
        let mut s = self.0.lock().unwrap();
        change(&mut s);
        if let Some(p) = path(app) {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Ok(json) = serde_json::to_vec_pretty(&*s) {
                let _ = std::fs::write(p, json);
            }
        }
        s.clone()
    }
}
