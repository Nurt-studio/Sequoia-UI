#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::rc::Rc;

use sequoia_core::{Color, RenderItem, RenderList};
use sequoia_text::{FontStyle, FontWeight, TextSystem};

fn fonts_available(sys: &mut TextSystem) -> bool {
    sys.shape("M", 16.0).glyph_count() > 0
}

#[test]
fn shaping_is_cached() {
    let mut sys = TextSystem::new();
    let a = sys.shape("hello", 16.0);
    let b = sys.shape("hello", 16.0);
    assert!(Rc::ptr_eq(&a, &b), "same (text, size) = same Rc");
    assert_ne!(a.width(), sys.shape("hello world", 16.0).width());
}

#[test]
fn weight_is_part_of_the_cache_key() {
    let mut sys = TextSystem::new();
    let regular = sys.shape_weighted("Sequoia", 16.0, FontWeight::NORMAL);
    let same = sys.shape_weighted("Sequoia", 16.0, FontWeight::NORMAL);
    assert!(Rc::ptr_eq(&regular, &same), "same weight = same Rc");

    let bold = sys.shape_weighted("Sequoia", 16.0, FontWeight::BOLD);
    assert!(
        !Rc::ptr_eq(&regular, &bold),
        "other weight is a separate cache entry (cannot alias another cut)"
    );
    assert_eq!(
        sys.shape("Sequoia", 16.0).glyph_count(),
        regular.glyph_count()
    );
}

#[test]
fn heavier_weight_is_never_narrower() {
    let mut sys = TextSystem::new();
    if !fonts_available(&mut sys) {
        eprintln!("skip: no fonts in environment");
        return;
    }
    let regular = sys
        .shape_weighted("Hamburgefonstiv", 20.0, FontWeight::NORMAL)
        .width();
    let bold = sys
        .shape_weighted("Hamburgefonstiv", 20.0, FontWeight::BOLD)
        .width();
    assert!(bold >= regular, "bold {bold} vs regular {regular}");
    assert_eq!(
        sys.text_width_weighted("Hamburgefonstiv", 20.0, FontWeight::BOLD),
        bold,
        "text_width_weighted uses the same weight"
    );
}

#[test]
fn weight_is_clamped_to_valid_range() {
    assert_eq!(FontWeight::new(0).value(), 1);
    assert_eq!(FontWeight::new(450).value(), 450);
    assert_eq!(FontWeight::new(4000).value(), 1000);
    assert_eq!(FontWeight::default(), FontWeight::NORMAL);
}

#[test]
fn default_font_is_bundled_inter() {
    let sys = TextSystem::new();
    assert_eq!(sys.default_family(), Some("Inter"));
}

#[test]
fn bundled_inter_shapes_without_system_fonts() {
    let mut sys = TextSystem::new();
    let shaped = sys.shape("Sequoia", 24.0);
    assert!(shaped.glyph_count() > 0);
    assert!(shaped.width() > 0.0);
}

#[test]
fn bundled_inter_weights_differ() {
    let mut sys = TextSystem::new();
    let regular = sys
        .shape_weighted("Hamburgefonstiv", 20.0, FontWeight::NORMAL)
        .width();
    let medium = sys
        .shape_weighted("Hamburgefonstiv", 20.0, FontWeight::MEDIUM)
        .width();
    let semibold = sys
        .shape_weighted("Hamburgefonstiv", 20.0, FontWeight::SEMIBOLD)
        .width();
    let bold = sys
        .shape_weighted("Hamburgefonstiv", 20.0, FontWeight::BOLD)
        .width();
    assert!(
        regular <= medium && medium <= semibold && semibold <= bold,
        "bundled Inter: regular {regular} medium {medium} semibold {semibold} bold {bold}"
    );
    assert!(
        bold > regular,
        "extreme weights must differ: regular {regular} vs bold {bold}"
    );
}

#[test]
fn arbitrary_weights_snap_to_bundled_inter() {
    let mut sys = TextSystem::new();
    let w =
        |sys: &mut TextSystem, v: u16| sys.shape_weighted("Save", 12.0, FontWeight::new(v)).width();
    let (m400, m500, m600, m700) = (
        w(&mut sys, 400),
        w(&mut sys, 500),
        w(&mut sys, 600),
        w(&mut sys, 700),
    );
    assert_eq!(w(&mut sys, 499), m500);
    assert_eq!(w(&mut sys, 501), m500);
    assert_eq!(w(&mut sys, 550), m600);
    assert_eq!(w(&mut sys, 450), m500, "tie goes up");
    assert_eq!(w(&mut sys, 300), m400);
    assert_eq!(w(&mut sys, 900), m700);
    assert!(
        m400 <= m500 && m500 <= m600 && m600 <= m700,
        "monotonicznie: {m400} {m500} {m600} {m700}"
    );
    assert!(
        m500 < m400 * 1.2,
        "no fallback blow-up (Marlett gave ~1.6x): {m500} vs {m400}"
    );
}

#[test]
fn default_family_can_be_cleared_to_system() {
    let mut sys = TextSystem::new();
    sys.set_default_family(None);
    assert_eq!(sys.default_family(), None);
}

#[test]
fn empty_text_has_no_glyphs_and_draws_nothing() {
    let mut sys = TextSystem::new();
    let shaped = sys.shape("", 16.0);
    assert_eq!(shaped.glyph_count(), 0);

    let mut list = RenderList::new();
    sys.draw(&shaped, (0.0, 0.0), Color::WHITE, &mut list);
    assert!(list.is_empty());
}

#[test]
fn text_measures_and_draws() {
    let mut sys = TextSystem::new();
    if !fonts_available(&mut sys) {
        eprintln!("skip: no fonts in environment");
        return;
    }
    let shaped = sys.shape("Sequoia", 24.0);
    assert!(shaped.glyph_count() > 0);
    assert!(shaped.width() > 0.0);

    let mut list = RenderList::new();
    sys.draw(&shaped, (10.0, 50.0), Color::WHITE, &mut list);

    assert_eq!(list.len(), 1);
    let Some(RenderItem::Text(draw)) = list.items().first() else {
        panic!("expected RenderItem::Text");
    };
    assert!(!draw.glyphs.is_empty());

    for g in &draw.glyphs {
        assert!(g.rect.size.width > 0.0);
        for v in g.uv {
            assert!((0.0..=1.0).contains(&v));
        }
    }

    let v = sys.atlas().version();
    sys.draw(&shaped, (10.0, 50.0), Color::WHITE, &mut list);
    assert_eq!(
        sys.atlas().version(),
        v,
        "second draw does not rasterize again"
    );
}

#[test]
fn atlas_grows_and_bumps_version() {
    let mut sys = TextSystem::with_atlas(64, 64);
    if !fonts_available(&mut sys) {
        eprintln!("skip: no fonts in environment");
        return;
    }
    let start = (sys.atlas().size(), sys.atlas().version());

    let mut list = RenderList::new();
    for i in 0..24u32 {
        let size = 8.0 + i as f32 * 2.0;
        let shaped = sys.shape(&format!("g{i}"), size);
        sys.draw(&shaped, (0.0, size), Color::WHITE, &mut list);
    }

    let (size, version) = (sys.atlas().size(), sys.atlas().version());
    assert!(version > start.1);
    assert!(
        size.0 > start.0.0 || size.1 > start.0.1,
        "64x64 atlas must grow with ~24 glyph sizes (grew to {size:?})"
    );
    assert_eq!(
        sys.atlas().pixels().len() as u64,
        u64::from(size.0) * u64::from(size.1) * 4,
        "atlas RGBA8: 4 bytes per pixel"
    );
}

#[test]
fn scale_factor_defaults_to_one_and_rejects_garbage() {
    let mut sys = TextSystem::new();
    assert_eq!(sys.scale_factor(), 1.0);
    sys.set_scale_factor(2.0);
    assert_eq!(sys.scale_factor(), 2.0);
    sys.set_scale_factor(0.0);
    sys.set_scale_factor(-1.0);
    sys.set_scale_factor(f32::INFINITY);
    sys.set_scale_factor(f32::NAN);
    assert_eq!(sys.scale_factor(), 2.0, "invalid scale keeps state");
}

fn draw_glyph_rects(sys: &mut TextSystem, text: &str, size: f32) -> Vec<sequoia_core::Rect> {
    let shaped = sys.shape(text, size);
    let mut list = RenderList::new();
    sys.draw(&shaped, (10.0, 50.0), Color::WHITE, &mut list);
    let Some(RenderItem::Text(draw)) = list.items().first() else {
        panic!("missing text");
    };
    draw.glyphs.iter().map(|g| g.rect).collect()
}

#[test]
fn scale_keeps_logical_metrics() {
    let mut lo = TextSystem::new();
    if !fonts_available(&mut lo) {
        eprintln!("skip: no fonts in environment");
        return;
    }
    let mut hi = TextSystem::new();
    hi.set_scale_factor(2.0);

    let a = lo.shape("Sequoia", 16.0);
    let b = hi.shape("Sequoia", 16.0);
    assert_eq!(a.glyph_count(), b.glyph_count());

    assert!(
        (a.width() - b.width()).abs() < 1.0,
        "logical width: {} vs {}",
        a.width(),
        b.width()
    );

    let ra = draw_glyph_rects(&mut lo, "Sequoia", 16.0);
    let rb = draw_glyph_rects(&mut hi, "Sequoia", 16.0);
    assert_eq!(ra.len(), rb.len());
    for (i, (x, y)) in ra.iter().zip(rb.iter()).enumerate() {
        for (what, p, q) in [
            ("left", x.left(), y.left()),
            ("top", x.top(), y.top()),
            ("width", x.size.width, y.size.width),
            ("height", x.size.height, y.size.height),
        ] {
            assert!(
                (p - q).abs() < 3.0,
                "glyph {i} {what}: scale 1 = {p}, scale 2 = {q}"
            );
        }
    }

    let right1 = ra.iter().map(|r| r.right()).fold(0.0_f32, f32::max);
    for (i, r) in rb.iter().enumerate() {
        assert!(
            r.right() < right1 + 3.0,
            "glyph {i} sticks out of the word: {} vs {right1}",
            r.right()
        );
    }
}

#[test]
fn uv_stay_valid_when_atlas_grows_mid_draw() {
    let mut sys = TextSystem::with_atlas(32, 32);
    if !fonts_available(&mut sys) {
        eprintln!("skip: no fonts in environment");
        return;
    }

    let text: String = ('A'..='Z').chain('a'..='z').chain('0'..='9').collect();
    let shaped = sys.shape(&text, 24.0);
    assert!(shaped.glyph_count() > 20);
    let before = sys.atlas().size();

    let mut list = RenderList::new();
    sys.draw(&shaped, (0.0, 0.0), Color::WHITE, &mut list);

    let after = sys.atlas().size();
    assert!(
        after.0 > before.0 || after.1 > before.1,
        "atlas should have grown"
    );

    let Some(RenderItem::Text(draw)) = list.items().first() else {
        panic!("missing text");
    };
    assert!(!draw.glyphs.is_empty());
    for (i, g) in draw.glyphs.iter().enumerate() {
        for (j, v) in g.uv.iter().enumerate() {
            assert!(
                (0.0..=1.0).contains(v),
                "glyph {i} uv[{j}] = {v} outside [0,1] (UV computed before growth?)"
            );
        }
    }
}

#[test]
fn draw_offsets_by_origin() {
    let mut sys = TextSystem::new();
    if !fonts_available(&mut sys) {
        eprintln!("skip: no fonts in environment");
        return;
    }
    let shaped = sys.shape("AA", 16.0);
    let mut list = RenderList::new();
    sys.draw(&shaped, (100.0, 200.0), Color::WHITE, &mut list);
    let Some(RenderItem::Text(draw)) = list.items().first() else {
        panic!("missing text");
    };
    let g = &draw.glyphs[0];

    assert!(
        (g.rect.left() - 100.0).abs() < 3.0,
        "glyph 'A' should sit at x=100 (origin), got {}",
        g.rect.left()
    );

    assert!(
        g.rect.top() < 200.0 && g.rect.top() > 170.0,
        "glyph above baseline=200, got {}",
        g.rect.top()
    );
}

#[test]
fn styled_shaping_hits_cache_per_family_and_style() {
    let mut sys = TextSystem::new();
    let base = sys.shape_weighted("Sequoia", 16.0, FontWeight::NORMAL);
    let explicit = sys.shape_styled(
        "Sequoia",
        16.0,
        FontWeight::NORMAL,
        Some("Inter"),
        FontStyle::Normal,
    );
    assert!(
        Rc::ptr_eq(&base, &explicit),
        "explicit Inter == default Inter (same cache entry)"
    );

    let italic = sys.shape_styled("Sequoia", 16.0, FontWeight::NORMAL, None, FontStyle::Italic);
    assert!(
        !Rc::ptr_eq(&base, &italic),
        "italic is a separate cache entry"
    );
    let italic2 = sys.shape_styled("Sequoia", 16.0, FontWeight::NORMAL, None, FontStyle::Italic);
    assert!(Rc::ptr_eq(&italic, &italic2), "same style = hit");

    let oblique = sys.shape_styled(
        "Sequoia",
        16.0,
        FontWeight::NORMAL,
        None,
        FontStyle::Oblique,
    );
    assert!(!Rc::ptr_eq(&italic, &oblique), "oblique != italic");
}

#[test]
fn font_alias_resolves_to_real_family() {
    let mut sys = TextSystem::new();
    sys.register_alias("MyAlias", "Inter");
    let via_alias = sys.shape_styled(
        "Sequoia",
        16.0,
        FontWeight::NORMAL,
        Some("MyAlias"),
        FontStyle::Normal,
    );
    let via_real = sys.shape_styled(
        "Sequoia",
        16.0,
        FontWeight::NORMAL,
        Some("Inter"),
        FontStyle::Normal,
    );
    assert!(
        Rc::ptr_eq(&via_alias, &via_real),
        "alias and real family share the cache entry"
    );
    assert_eq!(
        sys.text_width_styled_spaced(
            "Sequoia",
            16.0,
            FontWeight::NORMAL,
            0.0,
            Some("MyAlias"),
            FontStyle::Normal
        ),
        via_real.width()
    );
}

#[test]
fn unknown_family_falls_back_without_panic() {
    let mut sys = TextSystem::new();
    let shaped = sys.shape_styled(
        "Sequoia",
        16.0,
        FontWeight::NORMAL,
        Some("NoSuchFamily123"),
        FontStyle::Normal,
    );
    assert!(shaped.width() >= 0.0);
    let elided = sys.elide_styled_spaced(
        "Sequoia sequoia",
        16.0,
        FontWeight::NORMAL,
        40.0,
        Default::default(),
        0.0,
        Some("NoSuchFamily123"),
        FontStyle::Normal,
    );
    assert!(!elided.is_empty());
}
