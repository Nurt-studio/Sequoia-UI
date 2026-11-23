use std::path::PathBuf;

use sequoia_style::Theme;
use sequoia_view::AnyView;

use crate::bindings::HostBindings;
use crate::error::SeqError;
use crate::resolve::{FsSource, MapSource, SeqSource, resolve_imports, resolve_imports_all};
use crate::{check_all, lower, lower_with, parse_seq_file_all, resolve_theme};

#[derive(Clone, Debug)]
pub struct AppMeta {
    pub title: String,

    pub size: (f64, f64),

    pub theme: Theme,
}

#[derive(Clone)]
pub struct LoadedApp {
    pub meta: AppMeta,

    pub view: AnyView,

    pub files: Vec<PathBuf>,
}

pub struct SeqLoader;

impl SeqLoader {
    pub fn load_file(path: &str) -> Result<LoadedApp, SeqError> {
        let src = read_file(path)?;
        Self::load_from(path, &src, &FsSource, None)
    }

    pub fn load_str(file: &str, src: &str) -> Result<LoadedApp, SeqError> {
        Self::load_from(file, src, &FsSource, None)
    }

    pub fn load_with(path: &str, bindings: &HostBindings) -> Result<LoadedApp, SeqError> {
        let src = read_file(path)?;
        Self::load_from(path, &src, &FsSource, Some(bindings))
    }

    pub fn load_str_with(
        file: &str,
        src: &str,
        bindings: &HostBindings,
    ) -> Result<LoadedApp, SeqError> {
        Self::load_from(file, src, &FsSource, Some(bindings))
    }

    pub fn load_str_map(
        file: &str,
        src: &str,
        files: &[(&str, &str)],
    ) -> Result<LoadedApp, SeqError> {
        Self::load_from(file, src, &MapSource::new(files), None)
    }

    pub fn load_str_with_map(
        file: &str,
        src: &str,
        files: &[(&str, &str)],
        bindings: &HostBindings,
    ) -> Result<LoadedApp, SeqError> {
        Self::load_from(file, src, &MapSource::new(files), Some(bindings))
    }

    pub fn load_file_all(path: &str) -> (Option<LoadedApp>, Vec<SeqError>) {
        match read_file(path) {
            Ok(src) => Self::load_all(path, &src, &FsSource, None),
            Err(error) => (None, vec![error]),
        }
    }

    pub fn load_str_all(file: &str, src: &str) -> (Option<LoadedApp>, Vec<SeqError>) {
        Self::load_all(file, src, &FsSource, None)
    }

    pub fn load_with_all(
        path: &str,
        bindings: &HostBindings,
    ) -> (Option<LoadedApp>, Vec<SeqError>) {
        match read_file(path) {
            Ok(src) => Self::load_all(path, &src, &FsSource, Some(bindings)),
            Err(error) => (None, vec![error]),
        }
    }

    pub fn load_str_with_all(
        file: &str,
        src: &str,
        bindings: &HostBindings,
    ) -> (Option<LoadedApp>, Vec<SeqError>) {
        Self::load_all(file, src, &FsSource, Some(bindings))
    }

    pub fn load_str_map_all(
        file: &str,
        src: &str,
        files: &[(&str, &str)],
    ) -> (Option<LoadedApp>, Vec<SeqError>) {
        Self::load_all(file, src, &MapSource::new(files), None)
    }

    pub fn load_str_with_map_all(
        file: &str,
        src: &str,
        files: &[(&str, &str)],
        bindings: &HostBindings,
    ) -> (Option<LoadedApp>, Vec<SeqError>) {
        Self::load_all(file, src, &MapSource::new(files), Some(bindings))
    }

    fn load_from(
        file: &str,
        src: &str,
        source: &dyn SeqSource,
        bindings: Option<&HostBindings>,
    ) -> Result<LoadedApp, SeqError> {
        let (seq, errors) = parse_seq_file_all(file, src);
        if let Some(error) = SeqError::aggregate(errors) {
            return Err(error);
        }
        let resolved = resolve_imports(file, seq, source)?;
        let app = crate::resolve_seq_calls(resolved.app);
        if let Some(error) = SeqError::aggregate(check_all(file, &app)) {
            return Err(error);
        }
        let theme = resolve_theme(file, &app)?;
        let view = match bindings {
            Some(b) => lower_with(file, &app, b)?,
            None => lower(file, &app)?,
        };
        Ok(LoadedApp {
            meta: AppMeta {
                title: app.title.clone(),
                size: (f64::from(app.size.0), f64::from(app.size.1)),
                theme,
            },
            view,
            files: resolved.files,
        })
    }

    fn load_all(
        file: &str,
        src: &str,
        source: &dyn SeqSource,
        bindings: Option<&HostBindings>,
    ) -> (Option<LoadedApp>, Vec<SeqError>) {
        let (seq, mut diagnostics) = parse_seq_file_all(file, src);
        let (resolved, import_errors) = resolve_imports_all(file, seq, source);
        push_all(&mut diagnostics, import_errors);
        let Some(resolved) = resolved else {
            return (None, diagnostics);
        };
        let app = crate::resolve_seq_calls(resolved.app);
        push_all(&mut diagnostics, check_all(file, &app));
        if diagnostics.iter().any(SeqError::is_error) {
            return (None, diagnostics);
        }
        let theme = resolve_theme(file, &app);
        let view = match bindings {
            Some(b) => lower_with(file, &app, b),
            None => lower(file, &app),
        };
        let loaded = match (theme, view) {
            (Ok(theme), Ok(view)) => Some(LoadedApp {
                meta: AppMeta {
                    title: app.title.clone(),
                    size: (f64::from(app.size.0), f64::from(app.size.1)),
                    theme,
                },
                view,
                files: resolved.files,
            }),
            (theme, view) => {
                for result in [theme.map(|_| ()), view.map(|_| ())] {
                    if let Err(error) = result {
                        push_unique(&mut diagnostics, error);
                    }
                }
                None
            }
        };
        (loaded, diagnostics)
    }
}

fn push_all(target: &mut Vec<SeqError>, errors: Vec<SeqError>) {
    for error in errors {
        push_unique(target, error);
    }
}

fn push_unique(target: &mut Vec<SeqError>, error: SeqError) {
    if !target
        .iter()
        .any(|e| e.file == error.file && e.line == error.line && e.message == error.message)
    {
        target.push(error);
    }
}

fn read_file(path: &str) -> Result<String, SeqError> {
    std::fs::read_to_string(path)
        .map_err(|e| SeqError::new(path, 1, 1, format!("cannot be read: {e}")))
}
