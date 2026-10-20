use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

impl FileFilter {
    pub fn new(name: impl Into<String>, extensions: &[&str]) -> Self {
        Self {
            name: name.into(),
            extensions: extensions.iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn matches(&self, file_name: &str) -> bool {
        if self.extensions.iter().any(|e| e == "*") {
            return true;
        }
        let lower = file_name.to_lowercase();
        self.extensions.iter().any(|ext| {
            let ext = ext.to_lowercase();
            lower.ends_with(&format!(".{ext}"))
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct NativeFileDialog {
    title: String,
    directory: Option<PathBuf>,
    file_name: Option<String>,
    filters: Vec<FileFilter>,
}

impl NativeFileDialog {
    pub fn open_file(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }

    pub fn save_file(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }

    pub fn pick_folder(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn directory(mut self, dir: impl Into<PathBuf>) -> Self {
        self.directory = Some(dir.into());
        self
    }

    pub fn file_name(mut self, name: impl Into<String>) -> Self {
        self.file_name = Some(name.into());
        self
    }

    pub fn filter(mut self, name: impl Into<String>, extensions: &[&str]) -> Self {
        self.filters.push(FileFilter::new(name, extensions));
        self
    }

    pub fn dialog_title(&self) -> &str {
        &self.title
    }

    pub fn filters(&self) -> &[FileFilter] {
        &self.filters
    }

    fn apply_to_rfd(&self, mut dlg: rfd::FileDialog) -> rfd::FileDialog {
        if !self.title.is_empty() {
            dlg = dlg.set_title(&self.title);
        }
        if let Some(dir) = &self.directory {
            dlg = dlg.set_directory(dir);
        }
        if let Some(name) = &self.file_name {
            dlg = dlg.set_file_name(name);
        }
        for f in &self.filters {
            let exts: Vec<&str> = f.extensions.iter().map(|s| s.as_str()).collect();
            dlg = dlg.add_filter(&f.name, &exts);
        }
        dlg
    }

    pub fn pick(&self) -> Option<PathBuf> {
        self.apply_to_rfd(rfd::FileDialog::new()).pick_file()
    }

    pub fn pick_many(&self) -> Vec<PathBuf> {
        self.apply_to_rfd(rfd::FileDialog::new())
            .pick_files()
            .unwrap_or_default()
    }

    pub fn save(&self) -> Option<PathBuf> {
        self.apply_to_rfd(rfd::FileDialog::new()).save_file()
    }

    pub fn pick_dir(&self) -> Option<PathBuf> {
        self.apply_to_rfd(rfd::FileDialog::new()).pick_folder()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageKind {
    Info,
    Warning,
    Error,
}

pub fn native_message(title: &str, description: &str, kind: MessageKind) {
    let dlg = rfd::MessageDialog::new()
        .set_title(title)
        .set_description(description);
    let dlg = match kind {
        MessageKind::Info => dlg.set_level(rfd::MessageLevel::Info),
        MessageKind::Warning => dlg.set_level(rfd::MessageLevel::Warning),
        MessageKind::Error => dlg.set_level(rfd::MessageLevel::Error),
    };
    dlg.show();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_matches_by_extension_case_insensitive() {
        let f = FileFilter::new("Obrazy", &["png", "jpg"]);
        assert!(f.matches("foto.PNG"));
        assert!(f.matches("a.b.jpg"));
        assert!(!f.matches("doc.txt"));
        assert!(!f.matches("png"));
        let all = FileFilter::new("Wszystkie", &["*"]);
        assert!(all.matches("cokolwiek.bin"));
    }

    #[test]
    fn builder_keeps_title_filters_and_paths() {
        let d = NativeFileDialog::save_file("Zapisz")
            .directory("/tmp")
            .file_name("doc.txt")
            .filter("Tekst", &["txt"])
            .filter("Wszystkie", &["*"]);
        assert_eq!(d.dialog_title(), "Zapisz");
        assert_eq!(d.filters().len(), 2);
        assert_eq!(d.filters()[0].extensions, vec!["txt".to_string()]);
    }
}
