use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use sequoia_reactive::Signal;

#[derive(Clone)]
pub struct Settings {
    dir: PathBuf,
    file_name: String,
    map: Signal<HashMap<String, String>>,
    watchers: Rc<RefCell<HashMap<(String, String), Signal<String>>>>,
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum SettingsError {
    Io { path: String, message: String },

    Mkdir { path: String, message: String },

    Json { path: String, message: String },

    Encode { message: String },
}

impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, message } => write!(f, "odczyt {path}: {message}"),
            Self::Mkdir { path, message } => write!(f, "mkdir {path}: {message}"),
            Self::Json { path, message } => write!(f, "JSON {path}: {message}"),
            Self::Encode { message } => write!(f, "JSON: {message}"),
        }
    }
}

impl std::error::Error for SettingsError {}

impl Settings {
    pub fn with_dir(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            file_name: "settings.json".to_string(),
            map: Signal::new(HashMap::new()),
            watchers: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn for_app(qualifier: &str, organization: &str, application: &str) -> Self {
        let dir = directories::ProjectDirs::from(qualifier, organization, application)
            .map(|p| p.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        Self::with_dir(dir)
    }

    pub fn path(&self) -> PathBuf {
        self.dir.join(&self.file_name)
    }

    pub fn file_name(mut self, name: impl Into<String>) -> Self {
        self.file_name = name.into();
        self
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.map.with_untracked(|m| m.get(key).cloned())
    }

    pub fn get_or(&self, key: &str, default: &str) -> String {
        self.get(key).unwrap_or_else(|| default.to_string())
    }

    pub fn set(&self, key: impl Into<String>, value: impl Into<String>) {
        let (k, v) = (key.into(), value.into());
        self.map.update(|m| {
            m.insert(k, v);
        });
    }

    pub fn remove(&self, key: &str) -> bool {
        self.map.with_untracked(|m| m.contains_key(key));
        let mut removed = false;
        self.map.update(|m| {
            removed = m.remove(key).is_some();
        });
        removed
    }

    pub fn watch(&self, key: String, default: String) -> Signal<String> {
        let cache_key = (key, default);
        if let Some(sig) = self.watchers.borrow().get(&cache_key) {
            return sig.clone();
        }
        let (key, default) = cache_key.clone();
        let map = self.map.clone();
        let out = Signal::new(
            map.with_untracked(|m| m.get(&key).cloned().unwrap_or_else(|| default.clone())),
        );
        let out2 = out.clone();
        sequoia_reactive::EffectHandle::new(sequoia_reactive::effect(move || {
            let v = map
                .with(|m| m.get(&key).cloned())
                .unwrap_or_else(|| default.clone());
            out2.set(v);
        }))
        .leak();
        self.watchers.borrow_mut().insert(cache_key, out.clone());
        out
    }

    pub fn load(&self) -> Result<(), SettingsError> {
        let path = self.path();
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => {
                return Err(SettingsError::Io {
                    path: path.display().to_string(),
                    message: e.to_string(),
                });
            }
        };
        let map: HashMap<String, String> =
            serde_json::from_str(&text).map_err(|e| SettingsError::Json {
                path: path.display().to_string(),
                message: e.to_string(),
            })?;
        self.map.set(map);
        Ok(())
    }

    pub fn save(&self) -> Result<(), SettingsError> {
        let path = self.path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| SettingsError::Mkdir {
                path: parent.display().to_string(),
                message: e.to_string(),
            })?;
        }
        let map = self.map.get_untracked();
        let text = serde_json::to_string_pretty(&map).map_err(|e| SettingsError::Encode {
            message: e.to_string(),
        })?;
        std::fs::write(&path, text).map_err(|e| SettingsError::Io {
            path: path.display().to_string(),
            message: e.to_string(),
        })?;
        Ok(())
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn len(&self) -> usize {
        self.map.with_untracked(|m| m.len())
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_app(name: &str) -> (Settings, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "sequoia-settings-test-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        (Settings::with_dir(dir.clone()), dir)
    }

    #[test]
    fn roundtrip_through_disk() {
        let (s, dir) = tmp_app("roundtrip");
        s.set("theme", "light");
        s.set("w", "1200");
        s.save().unwrap();
        assert!(s.path().starts_with(&dir));

        let s2 = Settings::with_dir(dir.clone());
        s2.load().unwrap();
        assert_eq!(s2.get("theme").as_deref(), Some("light"));
        assert_eq!(s2.get_or("missing", "d"), "d");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_is_empty_not_error() {
        let (s, dir) = tmp_app("missing");
        s.load().unwrap();
        assert!(s.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn watch_follows_key() {
        let (s, dir) = tmp_app("watch");
        let t = s.watch("theme".to_string(), "dark".to_string());
        assert_eq!(t.get_untracked(), "dark");
        s.set("theme", "light");
        sequoia_reactive::flush_effects();
        assert_eq!(t.get_untracked(), "light");
        assert!(s.remove("theme"));
        assert!(!s.remove("theme"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn repeated_watch_returns_same_signal() {
        let (s, dir) = tmp_app("watch-cache");
        let a = s.watch("theme".to_string(), "dark".to_string());
        let b = s.watch("theme".to_string(), "dark".to_string());

        assert_eq!(a.id(), b.id(), "second watch must return the same signal");
        s.set("theme", "light");
        sequoia_reactive::flush_effects();
        assert_eq!(a.get_untracked(), "light");
        assert_eq!(b.get_untracked(), "light");

        let c = s.watch("theme".to_string(), "sepia".to_string());
        assert_ne!(c.id(), a.id(), "different default = separate signal");
        assert_eq!(
            c.get_untracked(),
            "light",
            "key already exists - default does not overwrite"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bad_json_is_error_not_panic() {
        let (s, dir) = tmp_app("badjson");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(s.path(), "{nie json").unwrap();
        assert!(s.load().is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
