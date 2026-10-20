use std::path::PathBuf;

use sequoia_reactive::Signal;

use crate::settings::Settings;

#[derive(Clone)]
pub struct RecentFiles {
    settings: Settings,
    key: String,
    max: usize,
    list: Signal<Vec<PathBuf>>,
}

impl RecentFiles {
    pub fn new(settings: Settings, max: usize) -> Self {
        Self {
            settings,
            key: "recent_files".to_string(),
            max: max.max(1),
            list: Signal::new(Vec::new()),
        }
    }

    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = key.into();
        self
    }

    pub fn files(&self) -> Vec<PathBuf> {
        self.list.get_untracked()
    }

    pub fn files_signal(&self) -> Signal<Vec<PathBuf>> {
        self.list.clone()
    }

    pub fn push(&self, path: impl Into<PathBuf>) {
        let path = path.into();
        if path.as_os_str().is_empty() {
            return;
        }
        let max = self.max;
        self.list.update(|list| {
            list.retain(|p| p != &path);
            list.insert(0, path);
            list.truncate(max);
        });
        self.persist();
    }

    pub fn remove(&self, path: &PathBuf) {
        self.list.update(|list| {
            list.retain(|p| p != path);
        });
        self.persist();
    }

    pub fn clear(&self) {
        self.list.set(Vec::new());
        self.persist();
    }

    fn persist(&self) {
        let paths: Vec<String> = self
            .list
            .get_untracked()
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        let json = serde_json::to_string(&paths).unwrap_or_else(|_| "[]".to_string());
        self.settings.set(&self.key, &json);
        let _ = self.settings.save();
    }

    pub fn load(&self) {
        let paths: Vec<String> = self
            .settings
            .get(&self.key)
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();
        let max = self.max;
        let mut list: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
        list.truncate(max);
        self.list.set(list);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(name: &str) -> (Settings, PathBuf) {
        let dir =
            std::env::temp_dir().join(format!("sequoia-recent-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        (Settings::with_dir(dir.clone()), dir)
    }

    #[test]
    fn push_dedups_trims_and_persists() {
        let (s, dir) = settings("dedupe");
        let r = RecentFiles::new(s.clone(), 3);
        r.push("/a.png");
        r.push("/b.png");
        r.push("/a.png");
        assert_eq!(
            r.files(),
            vec![PathBuf::from("/a.png"), PathBuf::from("/b.png")]
        );
        r.push("/c.png");
        r.push("/d.png");
        assert_eq!(r.files().len(), 3, "limit 3");
        assert_eq!(r.files()[0], PathBuf::from("/d.png"));

        let r2 = RecentFiles::new(s, 3);
        r2.load();
        assert_eq!(r2.files(), r.files());
        r.clear();
        assert!(r.files().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bad_json_is_empty_not_panic() {
        let (s, dir) = settings("badjson");
        s.set("recent_files", "nie json");
        let r = RecentFiles::new(s, 5);
        r.load();
        assert!(r.files().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
