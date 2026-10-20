use std::fs::File;
use std::path::PathBuf;

pub struct SingleInstance {
    path: PathBuf,
}

impl SingleInstance {
    fn lock_path(app_id: &str) -> PathBuf {
        let safe: String = app_id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        std::env::temp_dir().join(format!("sequoia-single-{safe}.lock"))
    }

    pub fn acquire(app_id: &str) -> Result<Self, ()> {
        let path = Self::lock_path(app_id);
        match File::create_new(&path) {
            Ok(_) => Ok(Self { path }),
            Err(_) => Err(()),
        }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn lock_path_for(app_id: &str) -> PathBuf {
        Self::lock_path(app_id)
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_acquire_fails_until_drop() {
        let id = format!("test-{}", std::process::id());
        let g = SingleInstance::acquire(&id).expect("pierwsza");
        assert!(SingleInstance::acquire(&id).is_err(), "druga odpada");
        drop(g);
        assert!(
            SingleInstance::acquire(&id).is_ok(),
            "po drop blokada wolna"
        );
    }

    #[test]
    fn lock_path_is_sanitized() {
        let p = SingleInstance::lock_path_for("a/b:c d");
        let name = p.file_name().unwrap().to_string_lossy();
        assert!(!name.contains('/') && !name.contains(':') && !name.contains(' '));
    }
}
