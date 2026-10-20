use std::path::PathBuf;

use sequoia_style::Theme;
use sequoia_view::AnyView;

use crate::bindings::HostBindings;
use crate::error::SeqError;
use crate::resolve::{FsSource, MapSource, SeqSource, resolve_imports};
use crate::{check, lower, lower_with, parse_seq_file, resolve_theme};

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

    fn load_from(
        file: &str,
        src: &str,
        source: &dyn SeqSource,
        bindings: Option<&HostBindings>,
    ) -> Result<LoadedApp, SeqError> {
        let seq = parse_seq_file(file, src)?;
        let resolved = resolve_imports(file, seq, source)?;
        let app = crate::resolve_seq_calls(resolved.app);
        check(file, &app)?;
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
}

fn read_file(path: &str) -> Result<String, SeqError> {
    std::fs::read_to_string(path)
        .map_err(|e| SeqError::new(path, 1, 1, format!("cannot be read: {e}")))
}
