use std::collections::HashMap;
use std::sync::OnceLock;

use sequoia_ui::prelude::*;
use sequoia_ui::view::Painter;

use crate::model::{Icon, ShapeKind};

struct Entry {
    pixels: Pixels,
}

static ICONS: OnceLock<HashMap<&'static str, Entry>> = OnceLock::new();

macro_rules! icon_files {
    ($($name:literal),* $(,)?) => {
        &[
            $((
                $name,
                &include_bytes!(concat!("../assets/icons/", $name, ".png"))[..],
            )),*
        ]
    };
}

fn load_icons() -> HashMap<&'static str, Entry> {
    let files = icon_files![
        "app",
        "brush",
        "caret",
        "close",
        "close_white",
        "copy",
        "crop",
        "cut",
        "dots",
        "eraser",
        "fdesktop",
        "fexit",
        "fill",
        "filldd",
        "finfo",
        "fmail",
        "fnew",
        "fopen",
        "fprint",
        "fprops",
        "fsave",
        "fsaveas",
        "fullscreen",
        "glass",
        "max",
        "min",
        "outline",
        "paste",
        "pencil",
        "picker",
        "redo",
        "resize",
        "rotate",
        "save",
        "select",
        "shape_arrD",
        "shape_arrL",
        "shape_arrR",
        "shape_arrU",
        "shape_callO",
        "shape_callR",
        "shape_curve",
        "shape_diamond",
        "shape_heart",
        "shape_hexagon",
        "shape_lightning",
        "shape_line",
        "shape_oval",
        "shape_pentagon",
        "shape_polygon",
        "shape_rectangle",
        "shape_rrect",
        "shape_rtri",
        "shape_star4",
        "shape_star5",
        "shape_star6",
        "shape_triangle",
        "size",
        "text",
        "thumb",
        "undo",
        "wheel",
        "zoom11",
        "zoomin",
        "zoomout",
    ];

    let mut map = HashMap::with_capacity(files.len());
    for (name, bytes) in files {
        let Ok(image) = image::load_from_memory(bytes) else {
            continue;
        };
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();
        let Some(pixels) = Pixels::from_rgba(width, height, rgba.into_raw()) else {
            continue;
        };
        map.insert(*name, Entry { pixels });
    }
    map
}

fn icons() -> &'static HashMap<&'static str, Entry> {
    ICONS.get_or_init(load_icons)
}

pub fn draw_named(g: &mut Painter, name: &str, rect: Rect) -> bool {
    let Some(entry) = icons().get(name) else {
        return false;
    };
    let width = entry.pixels.width() as f32;
    let height = entry.pixels.height() as f32;
    if width <= 0.0 || height <= 0.0 {
        return false;
    }
    let scale = (rect.size.width / width)
        .min(rect.size.height / height)
        .min(1.0);
    let w = width * scale;
    let h = height * scale;
    let target = Rect::from_xywh(rect.center().x - w * 0.5, rect.center().y - h * 0.5, w, h);
    g.pixels(target, &entry.pixels);
    true
}

pub fn draw(g: &mut Painter, icon: Icon, rect: Rect) -> bool {
    let Some(name) = name_for(icon) else {
        return false;
    };
    draw_named(g, name, rect)
}

fn name_for(icon: Icon) -> Option<&'static str> {
    Some(match icon {
        Icon::Save => "save",
        Icon::Undo => "undo",
        Icon::Redo => "redo",
        Icon::Paste => "paste",
        Icon::Cut => "cut",
        Icon::Copy => "copy",
        Icon::Select => "select",
        Icon::Crop => "crop",
        Icon::Resize => "resize",
        Icon::RotateLeft | Icon::RotateRight => "rotate",
        Icon::Pencil => "pencil",
        Icon::Fill => "fill",
        Icon::Text => "text",
        Icon::Eraser => "eraser",
        Icon::Picker => "picker",
        Icon::Magnifier => "glass",
        Icon::Brush => "brush",
        Icon::Size => "size",
        Icon::Outline => "outline",
        Icon::FillDropdown => "filldd",
        Icon::Wheel => "wheel",
        Icon::Shape(kind) => shape_name(kind),
        Icon::ZoomIn => "zoomin",
        Icon::ZoomOut => "zoomout",
        Icon::ZoomReset => "zoom11",
        Icon::Thumbnail => "thumb",
        Icon::Fullscreen => "fullscreen",
        Icon::Close => "close",
        Icon::File => "fnew",
        Icon::Folder => "fopen",
        Icon::Print => "fprint",
        Icon::Info => "finfo",
        Icon::Properties => "fprops",
        Icon::Wallpaper => "fdesktop",
        Icon::Fit
        | Icon::Ruler
        | Icon::Grid
        | Icon::StatusBar
        | Icon::Check
        | Icon::FlipH
        | Icon::FlipV => {
            return None;
        }
    })
}

fn shape_name(kind: ShapeKind) -> &'static str {
    match kind {
        ShapeKind::Line => "shape_line",
        ShapeKind::Curve => "shape_curve",
        ShapeKind::Oval => "shape_oval",
        ShapeKind::Rectangle => "shape_rectangle",
        ShapeKind::RoundedRectangle => "shape_rrect",
        ShapeKind::Polygon => "shape_polygon",
        ShapeKind::Triangle => "shape_triangle",
        ShapeKind::RightTriangle => "shape_rtri",
        ShapeKind::Diamond => "shape_diamond",
        ShapeKind::Pentagon => "shape_pentagon",
        ShapeKind::Hexagon => "shape_hexagon",
        ShapeKind::ArrowRight => "shape_arrR",
        ShapeKind::ArrowLeft => "shape_arrL",
        ShapeKind::ArrowUp => "shape_arrU",
        ShapeKind::ArrowDown => "shape_arrD",
        ShapeKind::Star4 => "shape_star4",
        ShapeKind::Star5 => "shape_star5",
        ShapeKind::Star6 => "shape_star6",
        ShapeKind::Callout => "shape_callR",
        ShapeKind::OvalCallout => "shape_callO",
        ShapeKind::Heart => "shape_heart",
        ShapeKind::Lightning => "shape_lightning",
    }
}
