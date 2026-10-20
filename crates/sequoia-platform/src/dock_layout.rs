use sequoia_dock::{DockModel, decode, encode};

use crate::settings::{Settings, SettingsError};

#[derive(Clone)]
pub struct DockLayoutStore {
    settings: Settings,
    file_name: String,
}

impl DockLayoutStore {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            file_name: "dock-layout.txt".to_string(),
        }
    }

    pub fn file_name(mut self, name: impl Into<String>) -> Self {
        self.file_name = name.into();
        self
    }

    pub fn path(&self) -> std::path::PathBuf {
        self.settings.dir().join(&self.file_name)
    }

    pub fn save(&self, model: &DockModel) -> Result<(), SettingsError> {
        let path = self.path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| SettingsError::Mkdir {
                path: parent.display().to_string(),
                message: e.to_string(),
            })?;
        }
        std::fs::write(&path, encode(model)).map_err(|e| SettingsError::Io {
            path: path.display().to_string(),
            message: e.to_string(),
        })?;
        Ok(())
    }

    pub fn load(&self) -> Option<DockModel> {
        let path = self.path();
        let text = std::fs::read_to_string(&path).ok()?;
        match decode(&text) {
            Ok(m) => Some(m),
            Err(e) => {
                log::warn!(
                    "dock-layout {} uszkodzony ({e:?}) — start z defaultem",
                    path.display()
                );
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_layout_is_none() {
        let dir = std::env::temp_dir().join(format!("sequoia-dock-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = DockLayoutStore::new(Settings::with_dir(dir.clone()));
        assert!(store.load().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_load_roundtrip() {
        use sequoia_dock::{DockOp, PanelId};
        let dir = std::env::temp_dir().join(format!("sequoia-dock-rt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = DockLayoutStore::new(Settings::with_dir(dir.clone()));
        let mut model = DockModel::single(PanelId::new(1));
        model.apply(DockOp::Insert {
            panel: PanelId::new(2),
            target: PanelId::new(1),
            drop: sequoia_dock::DockDrop::Right,
            frac: 0.3,
            moving: false,
        });
        store.save(&model).unwrap();
        let back = store.load().expect("zapisany layout");
        assert_eq!(back.panels().len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
