use std::collections::HashSet;

pub(crate) use backend::save_completed_levels;

pub(crate) fn load_completed_levels() -> HashSet<String> {
    backend::try_load_completed_levels().unwrap_or_default()
}

macro_rules! warn_err {
    ($expr:expr, $($arg:tt)+) => {
        $expr.map_err(|e| log::warn!($($arg)+, e)).ok()
    };
}

#[cfg(target_arch = "wasm32")]
mod backend {
    use super::*;

    const STORAGE_KEY: &str = "infestation_completed_levels";

    pub(crate) fn save_completed_levels(completed: &HashSet<String>) {
        let json = serde_json::to_string(completed).unwrap();
        quad_storage::STORAGE
            .lock()
            .unwrap()
            .set(STORAGE_KEY, &json);
    }

    pub(super) fn try_load_completed_levels() -> Option<HashSet<String>> {
        let s = quad_storage::STORAGE.lock().unwrap().get(STORAGE_KEY)?;
        warn_err!(serde_json::from_str(&s), "Failed to parse {}: {}", s)
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use super::*;
    use std::fs::{create_dir_all, read_to_string, write};
    use std::path::PathBuf;

    pub(crate) fn save_completed_levels(completed: &HashSet<String>) {
        if let Some(path) = save_path() {
            if let Some(parent) = path.parent() {
                let _ = create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string(completed) {
                let _ = write(path, json);
            }
        }
    }

    pub(super) fn try_load_completed_levels() -> Option<HashSet<String>> {
        let path = save_path()?;
        let s = warn_err!(
            read_to_string(&path),
            "Failed to read {}: {}",
            path.display()
        )?;
        warn_err!(
            serde_json::from_str(&s),
            "Failed to parse {}: {}",
            path.display()
        )
    }

    fn save_path() -> Option<PathBuf> {
        let dirs = directories::ProjectDirs::from("", "dspyz", "InfestationGame")?;
        Some(dirs.data_dir().join("completed_levels.json"))
    }
}
