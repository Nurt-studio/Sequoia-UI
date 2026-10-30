use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::photo::PhotoError;
use crate::raster::{MAX_RASTER_PX, Sprite, rasterize};
use crate::svg::{SvgError, SvgIcon, parse_svg};

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AssetError {
    Unknown(String),

    Photo(PhotoError),
}

impl std::fmt::Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetError::Unknown(name) => write!(f, "unknown icon `{name}`"),
            AssetError::Photo(e) => write!(f, "photo error: {e}"),
        }
    }
}

impl std::error::Error for AssetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AssetError::Unknown(_) => None,
            AssetError::Photo(e) => Some(e),
        }
    }
}

impl From<PhotoError> for AssetError {
    fn from(e: PhotoError) -> Self {
        AssetError::Photo(e)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IconDirReport {
    pub registered: Vec<String>,
    pub failed: Vec<(PathBuf, SvgError)>,
}

#[derive(Default)]
struct Inner {
    fonts: Vec<(String, Rc<[u8]>)>,
    default_font: Option<String>,
    icons: HashMap<String, Rc<SvgIcon>>,
    rasters: HashMap<(String, u32), Rc<Sprite>>,
    raster_sizes: HashMap<String, Vec<u32>>,

    photo_sizes: HashMap<String, (u32, u32)>,
}

const PHOTO_SIZES_MAX: usize = 1024;

const RASTERS_MAX: usize = 128;

#[derive(Clone, Default)]
pub struct AssetRegistry {
    inner: Rc<RefCell<Inner>>,
}

impl AssetRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_builtin_icons() -> Self {
        let registry = Self::new();
        for (name, svg) in crate::icons::builtin() {
            if let Err(e) = registry.register_icon(*name, svg) {
                debug_assert!(false, "bundled icon `{name}` must be valid: {e}");
                log::error!("sequoia-assets: bundled icon `{name}` invalid: {e} — skipped");
            }
        }
        registry
    }

    pub fn register_font(&self, name: impl Into<String>, data: Vec<u8>) {
        self.inner
            .borrow_mut()
            .fonts
            .push((name.into(), Rc::from(data)));
    }

    pub fn set_default_font(&self, name: impl Into<String>) {
        self.inner.borrow_mut().default_font = Some(name.into());
    }

    pub fn default_font(&self) -> Option<String> {
        self.inner.borrow().default_font.clone()
    }

    pub fn fonts(&self) -> Vec<(String, Rc<[u8]>)> {
        self.inner.borrow().fonts.clone()
    }

    pub fn register_icon(&self, name: impl Into<String>, svg_src: &str) -> Result<(), SvgError> {
        let icon = parse_svg(svg_src)?;
        self.register_icon_data(name, icon);
        Ok(())
    }

    pub fn register_icon_dir(&self, dir: impl AsRef<Path>) -> std::io::Result<IconDirReport> {
        let root = dir.as_ref();
        let mut files = Vec::new();
        collect_svg_files(root, &mut files)?;
        files.sort();

        let mut report = IconDirReport::default();
        for path in files {
            let Some(name) = icon_name(root, &path) else {
                continue;
            };
            let svg = std::fs::read_to_string(&path)?;
            match self.register_icon(name.clone(), &svg) {
                Ok(()) => report.registered.push(name),
                Err(e) => report.failed.push((path, e)),
            }
        }
        Ok(report)
    }

    pub fn register_icon_data(&self, name: impl Into<String>, icon: SvgIcon) {
        let name = name.into();
        let mut inner = self.inner.borrow_mut();
        if let Some(sizes) = inner.raster_sizes.remove(&name) {
            for px in sizes {
                inner.rasters.remove(&(name.clone(), px));
            }
        }
        inner.icons.insert(name, Rc::new(icon));
    }

    pub fn icon(&self, name: &str) -> Option<Rc<SvgIcon>> {
        let inner = self.inner.borrow();
        if let Some(icon) = inner.icons.get(name) {
            return Some(Rc::clone(icon));
        }
        if let Some((_, short)) = name.split_once(':')
            && let Some(icon) = inner.icons.get(short)
        {
            return Some(Rc::clone(icon));
        }
        None
    }

    pub fn icon_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.inner.borrow().icons.keys().cloned().collect();
        names.sort();
        names
    }

    pub fn icons_in_namespace(&self, ns: &str) -> Vec<String> {
        let prefix = format!("{ns}:");
        let mut names: Vec<String> = self
            .inner
            .borrow()
            .icons
            .keys()
            .filter(|k| k.starts_with(&prefix))
            .cloned()
            .collect();
        names.sort();
        names
    }

    pub fn rasterize_icon(&self, name: &str, px: u32) -> Result<Rc<Sprite>, AssetError> {
        if px > MAX_RASTER_PX {
            return Err(AssetError::Photo(PhotoError::TooLarge {
                width: px,
                height: px,
            }));
        }
        let resolved = self
            .resolve_icon_name(name)
            .ok_or_else(|| AssetError::Unknown(name.into()))?;
        let mut inner = self.inner.borrow_mut();
        if let Some(sprite) = inner.rasters.get(&(resolved.clone(), px)) {
            return Ok(Rc::clone(sprite));
        }
        let icon = inner
            .icons
            .get(&resolved)
            .cloned()
            .ok_or_else(|| AssetError::Unknown(name.into()))?;
        let sprite = Rc::new(rasterize(&icon, px));
        if inner.rasters.len() >= RASTERS_MAX
            && let Some(victim) = inner.rasters.keys().next().cloned()
        {
            inner.rasters.remove(&victim);
            if let Some(sizes) = inner.raster_sizes.get_mut(&victim.0) {
                sizes.retain(|&s| s != victim.1);
                if sizes.is_empty() {
                    inner.raster_sizes.remove(&victim.0);
                }
            }
        }
        let sizes = inner.raster_sizes.entry(resolved.clone()).or_default();
        if !sizes.contains(&px) {
            sizes.push(px);
        }
        inner.rasters.insert((resolved, px), Rc::clone(&sprite));
        Ok(sprite)
    }

    fn resolve_icon_name(&self, name: &str) -> Option<String> {
        let inner = self.inner.borrow();
        if inner.icons.contains_key(name) {
            return Some(name.to_owned());
        }
        if let Some((_, short)) = name.split_once(':')
            && inner.icons.contains_key(short)
        {
            return Some(short.to_owned());
        }
        None
    }

    pub fn photo_size(&self, path: &str) -> Result<(u32, u32), AssetError> {
        let mut inner = self.inner.borrow_mut();
        if let Some(size) = inner.photo_sizes.get(path) {
            return Ok(*size);
        }
        let size = image::image_dimensions(path)
            .map_err(|_| AssetError::Photo(PhotoError::Io(path.into())))?;
        if size.0 == 0 || size.1 == 0 {
            return Err(AssetError::Photo(PhotoError::Decode("empty image".into())));
        }
        if inner.photo_sizes.len() >= PHOTO_SIZES_MAX {
            inner.photo_sizes.clear();
        }
        inner.photo_sizes.insert(path.to_owned(), size);
        Ok(size)
    }
}

fn collect_svg_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_svg_files(&path, out)?;
        } else if file_type.is_file()
            && path
                .extension()
                .is_some_and(|ext| ext.to_string_lossy().eq_ignore_ascii_case("svg"))
        {
            out.push(path);
        }
    }
    Ok(())
}

fn icon_name(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let mut parts = Vec::new();
    for component in relative.components() {
        let std::path::Component::Normal(part) = component else {
            continue;
        };
        parts.push(part.to_str()?.to_owned());
    }
    let file = parts.pop()?;
    let stem = Path::new(&file).file_stem()?.to_str()?.to_owned();
    parts.push(stem);
    Some(parts.join("/"))
}

pub fn icon_key(name: &str, px: u32) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in name.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash ^= u64::from(px);
    hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOT: &str = r#"<svg viewBox="0 0 10 10"><circle cx="5" cy="5" r="4"/></svg>"#;

    #[test]
    fn register_and_rasterize_with_cache() {
        let registry = AssetRegistry::new();
        registry.register_icon("dot", DOT).unwrap();
        let a = registry.rasterize_icon("dot", 16).unwrap();
        let b = registry.rasterize_icon("dot", 16).unwrap();
        assert!(Rc::ptr_eq(&a, &b), "cache returns the same sprite");
        assert_eq!((a.width, a.height), (16, 16));
        assert!(registry.rasterize_icon("missing", 16).is_err());
    }

    #[test]
    fn reregister_invalidates_cache() {
        let registry = AssetRegistry::new();
        registry.register_icon("dot", DOT).unwrap();
        let first = registry.rasterize_icon("dot", 16).unwrap();
        registry
            .register_icon(
                "dot",
                r#"<svg viewBox="0 0 10 10"><rect width="10" height="10"/></svg>"#,
            )
            .unwrap();
        let second = registry.rasterize_icon("dot", 16).unwrap();
        assert!(!Rc::ptr_eq(&first, &second));
    }

    #[test]
    fn bad_svg_is_error() {
        let registry = AssetRegistry::new();
        assert!(matches!(
            registry.register_icon("x", "<svg><path d='M1'/></svg>"),
            Err(SvgError::BadPath(_, _))
        ));
        assert!(registry.icon("x").is_none());
    }

    #[test]
    fn oversize_raster_request_is_refused() {
        use crate::raster::MAX_RASTER_PX;
        let registry = AssetRegistry::new();
        registry.register_icon("dot", DOT).unwrap();
        assert!(
            registry.rasterize_icon("dot", MAX_RASTER_PX).is_ok(),
            "limit przechodzi"
        );
        assert!(
            registry.rasterize_icon("dot", MAX_RASTER_PX + 1).is_err(),
            "past the limit: refused instead of px alloc²"
        );
        assert!(registry.rasterize_icon("dot", u32::MAX).is_err());
    }

    #[test]
    fn fonts_roundtrip() {
        let registry = AssetRegistry::new();
        registry.register_font("Inter", vec![1, 2, 3]);
        registry.set_default_font("Inter");
        assert_eq!(registry.default_font(), Some("Inter".into()));
        let fonts = registry.fonts();
        assert_eq!(fonts.len(), 1);
        assert_eq!(fonts[0].0, "Inter");
        assert_eq!(&*fonts[0].1, &[1, 2, 3]);
    }

    #[test]
    fn clone_shares_state() {
        let registry = AssetRegistry::new();
        let clone = registry.clone();
        clone.register_icon("dot", DOT).unwrap();
        assert!(registry.icon("dot").is_some());
    }

    #[test]
    fn icon_key_stable_and_distinct() {
        assert_eq!(icon_key("check", 16), icon_key("check", 16));
        assert_ne!(icon_key("check", 16), icon_key("check", 24));
        assert_ne!(icon_key("check", 16), icon_key("close", 16));
    }

    #[test]
    fn builtin_icons_all_parse_and_rasterize() {
        let registry = AssetRegistry::with_builtin_icons();
        let names = registry.icon_names();
        assert!(names.len() >= 12, "wbudowany zestaw: {names:?}");
        for name in names {
            let sprite = registry.rasterize_icon(&name, 24).expect("rasteryzacja");
            assert!(!sprite.is_blank(), "icon {name} must not be blank");
        }
    }

    #[test]
    fn icon_dir_loads_recursively_by_relative_name() {
        let root = std::env::temp_dir().join(format!("sequoia-icons-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("tool")).unwrap();
        std::fs::write(root.join("tool/select.svg"), DOT).unwrap();
        std::fs::write(root.join("bad.svg"), "<svg></svg>").unwrap();
        std::fs::write(root.join("notes.txt"), DOT).unwrap();

        let registry = AssetRegistry::new();
        let report = registry.register_icon_dir(&root).unwrap();
        assert_eq!(report.registered, vec!["tool/select".to_owned()]);
        assert_eq!(report.failed.len(), 1);
        assert_eq!(report.failed[0].0, root.join("bad.svg"));
        assert!(registry.icon("tool/select").is_some());
        assert!(registry.icon("notes").is_none());
        assert!(registry.register_icon_dir(root.join("missing")).is_err());

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn photo_size_probes_without_decoding() {
        let registry = AssetRegistry::new();
        assert!(registry.photo_size("no/such/file.png").is_err());

        let dir = std::env::temp_dir().join("sequoia-photo-probe");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("probe.png");
        let pixels: Vec<u8> = (0..4 * 3).flat_map(|_| [10u8, 20, 30, 255]).collect();
        image::save_buffer(&path, &pixels, 4, 3, image::ColorType::Rgba8).unwrap();
        let size = registry
            .photo_size(path.to_str().unwrap())
            .expect("wymiary PNG");
        assert_eq!(size, (4, 3));

        std::fs::remove_file(&path).unwrap();
        assert_eq!(
            registry.photo_size(path.to_str().unwrap()),
            Ok((4, 3)),
            "dimension cache"
        );
    }

    #[test]
    fn raster_cache_evicts_one_entry_not_everything() {
        let registry = AssetRegistry::new();
        registry.register_icon("dot", DOT).unwrap();
        for px in 1..=super::RASTERS_MAX as u32 {
            assert!(registry.rasterize_icon("dot", px).is_ok());
        }
        assert_eq!(registry.inner.borrow().rasters.len(), super::RASTERS_MAX);

        assert!(
            registry
                .rasterize_icon("dot", super::RASTERS_MAX as u32 + 1)
                .is_ok()
        );

        let inner = registry.inner.borrow();
        assert_eq!(inner.rasters.len(), super::RASTERS_MAX);
        let survivors = (1..=super::RASTERS_MAX as u32)
            .filter(|px| inner.rasters.contains_key(&("dot".to_owned(), *px)))
            .count();
        assert_eq!(
            survivors,
            super::RASTERS_MAX - 1,
            "single-entry eviction keeps the rest"
        );
    }
}
