use std::path::{Path, PathBuf};

use sequoia_ui::prelude::*;

use crate::model::*;

pub fn choose_open() -> Option<PathBuf> {
    NativeFileDialog::open_file("Open image")
        .filter("Images", &["png", "jpg", "jpeg"])
        .filter("All files", &["*"])
        .pick()
}

pub fn choose_save(suggested: Option<&str>) -> Option<PathBuf> {
    let mut dlg = NativeFileDialog::save_file("Save image")
        .filter("PNG image", &["png"])
        .filter("JPEG image", &["jpg", "jpeg"]);
    if let Some(name) = suggested {
        dlg = dlg.file_name(name);
    }
    dlg.save()
}

pub fn load(path: &Path) -> Result<Pixels, String> {
    let s = path.to_string_lossy();
    sequoia_ui::assets::load_pixels(&s).map_err(|e| e.to_string())
}

pub fn save(path: &Path, doc: &Pixels) -> Result<(), String> {
    let img = image::RgbaImage::from_raw(doc.width(), doc.height(), doc.data().to_vec())
        .ok_or_else(|| "Could not create image buffer".to_string())?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_ascii_lowercase();
    let result = match ext.as_str() {
        "jpg" | "jpeg" => image::DynamicImage::ImageRgba8(img).to_rgb8().save(path),
        _ => img.save(path),
    };
    result.map_err(|e| e.to_string())
}

pub fn open_into(state: &mut State, path: PathBuf) {
    match load(&path) {
        Ok(px) => {
            state.clear_selection_committing();
            state.text = None;
            state.doc = px;
            state.doc_version = state.doc_version.wrapping_add(1);
            state.path = Some(path.clone());
            state.dirty = false;
            state.undo.clear();
            state.redo.clear();
            state.selection = None;
            state.scroll = Vec2::ZERO;
            push_recent(state, path);
            state.set_status("Image opened");
        }
        Err(e) => state.set_status(format!("Could not open file: {e}")),
    }
}

pub fn save_current(state: &mut State, save_as: bool) -> bool {
    let path = if save_as || state.path.is_none() {
        let suggested = state
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .or_else(|| Some("Untitled.png".to_string()));
        match choose_save(suggested.as_deref()) {
            Some(p) => p,
            None => return false,
        }
    } else {
        state
            .path
            .clone()
            .unwrap_or_else(|| PathBuf::from("Untitled.png"))
    };
    match save(&path, &state.doc) {
        Ok(()) => {
            state.path = Some(path.clone());
            state.dirty = false;
            push_recent(state, path);
            state.set_status("Image saved");
            true
        }
        Err(e) => {
            state.set_status(format!("Could not save: {e}"));
            false
        }
    }
}

pub fn push_recent(state: &mut State, path: PathBuf) {
    state.recent.retain(|p| p != &path);
    state.recent.push(path);
    if state.recent.len() > 10 {
        state.recent.remove(0);
    }
}
