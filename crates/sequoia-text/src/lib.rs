#![forbid(unsafe_code)]

mod rich;

pub use rich::{
    RichAlign, RichCaret, RichHit, RichLine, RichParagraph, RichRect, Squiggle, TextSpan,
    styled_spans_to_text_spans,
};

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use cosmic_text::{
    Attrs, Buffer, CacheKey, Family, FontSystem, Metrics, Shaping, Style as CosmicStyle,
    SwashCache, Weight,
};
use sequoia_buffer::{LineIndex, StyledBuffer, StyledSpan, hash_spans_rebased, rebase_spans};
use sequoia_core::{Color, GlyphQuad, Rect, RenderList, TextDraw};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontWeight(u16);

impl FontWeight {
    pub const THIN: Self = Self(100);

    pub const EXTRA_LIGHT: Self = Self(200);

    pub const LIGHT: Self = Self(300);

    pub const NORMAL: Self = Self(400);

    pub const MEDIUM: Self = Self(500);

    pub const SEMIBOLD: Self = Self(600);

    pub const BOLD: Self = Self(700);

    pub const EXTRA_BOLD: Self = Self(800);

    pub const BLACK: Self = Self(900);

    pub const fn new(value: u16) -> Self {
        Self(if value < 1 {
            1
        } else if value > 1000 {
            1000
        } else {
            value
        })
    }

    pub const fn value(self) -> u16 {
        self.0
    }
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::NORMAL
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
    Oblique,
}

impl FontStyle {
    pub const fn is_italic(self) -> bool {
        !matches!(self, Self::Normal)
    }

    pub(crate) fn cosmic(self) -> CosmicStyle {
        match self {
            Self::Normal => CosmicStyle::Normal,
            Self::Italic => CosmicStyle::Italic,
            Self::Oblique => CosmicStyle::Oblique,
        }
    }

    pub(crate) fn key(self) -> u8 {
        match self {
            Self::Normal => 0,
            Self::Italic => 1,
            Self::Oblique => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WrapMode {
    #[default]
    Word,

    Character,
}

impl WrapMode {
    pub(crate) fn cosmic(self) -> cosmic_text::Wrap {
        match self {
            Self::Word => cosmic_text::Wrap::Word,
            Self::Character => cosmic_text::Wrap::Glyph,
        }
    }

    pub(crate) fn key(self) -> u8 {
        match self {
            Self::Word => 0,
            Self::Character => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ElideMode {
    #[default]
    Right,

    Left,

    Middle,
}

pub const ELLIPSIS: char = '…';

/// How a line box height is derived from the font size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineHeight {
    /// A multiplier of the font size (CSS-like unitless line height).
    Factor(f32),
    /// An absolute logical-pixel height.
    Absolute(f32),
}

impl Default for LineHeight {
    fn default() -> Self {
        Self::Factor(1.35)
    }
}

impl LineHeight {
    /// Resolves the line box height for a font size.
    pub fn resolve(self, size: f32) -> f32 {
        let size = if size.is_finite() { size.max(0.0) } else { 0.0 };
        let px = match self {
            Self::Factor(factor) => size * factor,
            Self::Absolute(px) => px,
        };
        if px.is_finite() {
            px.max(0.0)
        } else {
            size * 1.35
        }
    }

    pub(crate) fn key(self) -> u32 {
        match self {
            Self::Factor(factor) => ((factor.max(0.0) * 4.0).round() as u32).max(1),
            Self::Absolute(px) => (px.max(0.0) * 4.0).round() as u32,
        }
    }
}

/// Vertical placement of the text block inside its content box.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextVAlign {
    #[default]
    Top,
    Center,
    Bottom,
    /// Aligns the first text baseline with the top of the content box.
    Baseline,
}

pub struct ShapedText {
    glyphs: Vec<ShapedGlyph>,
    rows: Vec<LineRow>,
    width: f32,
    height: f32,
    line_height: f32,
}

impl ShapedText {
    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    pub fn glyph_count(&self) -> usize {
        self.glyphs.len()
    }

    /// Number of laid-out lines. Empty text has zero lines.
    pub fn line_count(&self) -> usize {
        self.rows.len()
    }

    /// Distance from the top of the first line box to its baseline.
    pub fn first_baseline(&self) -> f32 {
        self.rows.first().map(|row| row.baseline).unwrap_or(0.0)
    }

    /// Resolved line box height (the first row, or the configured height for empty text).
    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    /// Per-line metrics relative to the top of the text block.
    pub fn rows(&self) -> &[LineRow] {
        &self.rows
    }

    pub fn row(&self, index: usize) -> Option<&LineRow> {
        self.rows.get(index)
    }
}

struct ShapedGlyph {
    x: f32,
    y: f32,
    key: CacheKey,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineRow {
    pub top: f32,

    pub height: f32,

    pub baseline: f32,

    pub width: f32,
}

impl LineRow {
    pub fn bottom(&self) -> f32 {
        self.top + self.height
    }
}

pub struct ShapedLine {
    glyphs: Vec<ShapedGlyph>,
    rows: Vec<LineRow>,
    width: f32,
    height: f32,

    hash: u64,

    width_q: u32,
}

impl ShapedLine {
    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    pub fn glyph_count(&self) -> usize {
        self.glyphs.len()
    }

    pub fn row_count(&self) -> usize {
        self.rows.len().max(1)
    }

    pub fn rows(&self) -> &[LineRow] {
        &self.rows
    }

    pub fn row(&self, index: usize) -> Option<&LineRow> {
        self.rows.get(index)
    }

    pub fn hash(&self) -> u64 {
        self.hash
    }

    pub fn width_quant(&self) -> u32 {
        self.width_q
    }

    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }
}

const LINE_CACHE_MAX: usize = 16_384;

pub type LineCacheKey = (u64, u32);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtlasEntry {
    pub rect: [u32; 4],

    pub left: i32,

    pub top: i32,

    pub color: bool,
}

const ATLAS_MAX_DIM: u32 = 2048;

const ATLAS_BPP: usize = 4;

fn alpha_len(w: u32, h: u32) -> Option<usize> {
    (w as usize).checked_mul(h as usize)
}

fn rgba_len(w: u32, h: u32) -> Option<usize> {
    alpha_len(w, h)?.checked_mul(ATLAS_BPP)
}

fn checked_atlas_bytes(w: u32, h: u32) -> Option<usize> {
    rgba_len(w, h)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasDirty {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub full: bool,
}

static ATLAS_SERIAL: AtomicU64 = AtomicU64::new(1);

pub struct GlyphAtlas {
    id: u64,
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    entries: HashMap<CacheKey, AtlasEntry>,
    sprites: HashMap<u64, AtlasEntry>,
    cursor_x: u32,
    cursor_y: u32,
    row_h: u32,
    version: u64,

    epoch: u64,
    dirty: Option<(u32, u32, u32, u32)>,
    dirty_full: bool,

    tick: Cell<u64>,
    used: RefCell<HashMap<CacheKey, u64>>,
    sprite_used: RefCell<HashMap<u64, u64>>,
}

impl GlyphAtlas {
    pub fn new(width: u32, height: u32) -> Self {
        let width = width.clamp(1, ATLAS_MAX_DIM);
        let height = height.clamp(1, ATLAS_MAX_DIM);

        let bytes = checked_atlas_bytes(width, height).unwrap_or(0);

        Self {
            id: ATLAS_SERIAL.fetch_add(1, Ordering::Relaxed),
            width,
            height,
            pixels: vec![0; bytes],
            entries: HashMap::new(),
            sprites: HashMap::new(),
            cursor_x: 0,
            cursor_y: 0,
            row_h: 0,
            version: 0,
            epoch: 0,
            dirty: None,
            dirty_full: false,
            tick: Cell::new(0),
            used: RefCell::new(HashMap::new()),
            sprite_used: RefCell::new(HashMap::new()),
        }
    }

    /// Unique identity for this atlas instance.
    ///
    /// Glyph UVs are only meaningful for the atlas that produced them; caches
    /// must compare this id before reusing shaped output across text systems.
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    fn mark_dirty(&mut self, x: u32, y: u32, w: u32, h: u32) {
        if w == 0 || h == 0 {
            return;
        }
        if self.dirty_full {
            return;
        }
        match self.dirty {
            None => self.dirty = Some((x, y, w, h)),
            Some((dx, dy, dw, dh)) => {
                let x0 = dx.min(x);
                let y0 = dy.min(y);
                let x1 = (dx + dw).max(x + w).min(self.width);
                let y1 = (dy + dh).max(y + h).min(self.height);
                self.dirty = Some((x0, y0, x1.saturating_sub(x0), y1.saturating_sub(y0)));
            }
        }
    }

    pub fn take_dirty(&mut self) -> Option<AtlasDirty> {
        if self.dirty_full {
            self.dirty_full = false;
            self.dirty = None;
            return Some(AtlasDirty {
                x: 0,
                y: 0,
                w: self.width,
                h: self.height,
                full: true,
            });
        }
        self.dirty.take().map(|(x, y, w, h)| AtlasDirty {
            x,
            y,
            w: w.min(self.width.saturating_sub(x)),
            h: h.min(self.height.saturating_sub(y)),
            full: false,
        })
    }

    pub fn copy_rect(&self, x: u32, y: u32, w: u32, h: u32, out: &mut Vec<u8>) {
        out.clear();
        if w == 0 || h == 0 {
            return;
        }
        let (w, h) = (
            w.min(self.width.saturating_sub(x)),
            h.min(self.height.saturating_sub(y)),
        );
        if let Some(want) = checked_atlas_bytes(w, h) {
            out.reserve(want);
        }
        let row_bytes = w as usize * ATLAS_BPP;
        for row in 0..h {
            let src = (((y + row) * self.width + x) as usize) * ATLAS_BPP;
            debug_assert!(src.saturating_add(row_bytes) <= self.pixels.len());
            out.extend_from_slice(&self.pixels[src..src + row_bytes]);
            debug_assert_eq!(out.len() % ATLAS_BPP, 0);
        }
    }

    pub fn entry(&self, key: &CacheKey) -> Option<AtlasEntry> {
        let entry = self.entries.get(key).copied()?;
        self.touch(self.used.borrow_mut().entry(*key).or_default());
        Some(entry)
    }

    pub fn sprite(&self, key: u64) -> Option<AtlasEntry> {
        let entry = self.sprites.get(&key).copied()?;
        self.touch(self.sprite_used.borrow_mut().entry(key).or_default());
        Some(entry)
    }

    fn touch(&self, slot: &mut u64) {
        let t = self.tick.get() + 1;
        self.tick.set(t);
        *slot = t;
    }

    pub fn insert_sprite(&mut self, key: u64, w: u32, h: u32, alpha: &[u8]) -> AtlasEntry {
        let (w, h) = (w.max(1), h.max(1));

        let Some(count) = alpha_len(w, h).filter(|want| alpha.len() >= *want) else {
            return AtlasEntry {
                rect: [0, 0, 1, 1],
                left: 0,
                top: 0,
                color: false,
            };
        };
        let mut rgba = Vec::with_capacity(count * ATLAS_BPP);
        for &a in &alpha[..count] {
            rgba.extend_from_slice(&[255, 255, 255, a]);
        }
        if !self.blit(w, h, &rgba) {
            log::warn!(
                "sequoia-text: atlas full (max {ATLAS_MAX_DIM}px) - sprite skipped, icon will have a hole"
            );
            return AtlasEntry {
                rect: [0, 0, 1, 1],
                left: 0,
                top: 0,
                color: false,
            };
        }
        let entry = AtlasEntry {
            rect: [self.cursor_x, self.cursor_y, w, h],
            left: 0,
            top: 0,
            color: false,
        };
        self.sprites.insert(key, entry);
        self.touch(self.sprite_used.borrow_mut().entry(key).or_default());
        self.cursor_x += w + 1;
        self.row_h = self.row_h.max(h);
        self.version += 1;
        entry
    }

    fn blit(&mut self, w: u32, h: u32, data: &[u8]) -> bool {
        if !self.ensure_space(w, h) {
            return false;
        }
        let (bx, by) = (self.cursor_x, self.cursor_y);
        let row_bytes = w as usize * ATLAS_BPP;
        for row in 0..h {
            let src = row as usize * row_bytes;
            let dst = (((by + row) * self.width + bx) as usize) * ATLAS_BPP;
            self.pixels[dst..dst + row_bytes].copy_from_slice(&data[src..src + row_bytes]);
        }
        self.mark_dirty(bx, by, w, h);
        true
    }

    fn insert(
        &mut self,
        key: CacheKey,
        w: u32,
        h: u32,
        data: &[u8],
        left: i32,
        top: i32,
        color: bool,
    ) -> AtlasEntry {
        let (w, h) = (w.max(1), h.max(1));
        if rgba_len(w, h).is_none_or(|want| data.len() < want) {
            return AtlasEntry {
                rect: [0, 0, 1, 1],
                left: 0,
                top: 0,
                color: false,
            };
        }
        if !self.blit(w, h, data) {
            return AtlasEntry {
                rect: [0, 0, 1, 1],
                left: 0,
                top: 0,
                color: false,
            };
        }

        let entry = AtlasEntry {
            rect: [self.cursor_x, self.cursor_y, w, h],
            left,
            top,
            color,
        };
        self.entries.insert(key, entry);
        self.touch(self.used.borrow_mut().entry(key).or_default());
        self.cursor_x += w + 1;
        self.row_h = self.row_h.max(h);
        self.version += 1;
        entry
    }

    fn ensure_space(&mut self, w: u32, h: u32) -> bool {
        if w > ATLAS_MAX_DIM || h > ATLAS_MAX_DIM {
            return false;
        }

        if w > self.width {
            let nw = w.next_power_of_two().min(ATLAS_MAX_DIM);
            if nw < w {
                return false;
            }
            self.resize(nw, self.height);
        }
        if self.cursor_x + w > self.width {
            self.cursor_y += self.row_h + 1;
            self.cursor_x = 0;
            self.row_h = 0;
        }
        if self.cursor_y + h <= self.height {
            return true;
        }

        let mut nh = self.height;
        while self.cursor_y + h > nh && nh < ATLAS_MAX_DIM {
            nh = (nh * 2).min(ATLAS_MAX_DIM);
        }
        if self.cursor_y + h <= nh {
            self.resize(self.width, nh);
            return true;
        }

        log::debug!("sequoia-text: atlas overflow - LRU compaction (newest entries kept)");
        self.repack();
        if self.cursor_x + w > self.width {
            self.cursor_y += self.row_h + 1;
            self.cursor_x = 0;
            self.row_h = 0;
        }
        if self.cursor_y + h > self.height {
            log::debug!("sequoia-text: atlas still full after compaction - full clear");
            self.gc();
        }
        while self.cursor_y + h > self.height && self.height < ATLAS_MAX_DIM {
            let nh = (self.height * 2).min(ATLAS_MAX_DIM);
            self.resize(self.width, nh);
        }
        self.cursor_x + w <= self.width && self.cursor_y + h <= self.height
    }

    fn repack(&mut self) {
        enum Kind {
            Glyph(CacheKey),
            Sprite(u64),
        }
        let mut items: Vec<(u64, Kind, AtlasEntry)> =
            Vec::with_capacity(self.entries.len() + self.sprites.len());
        for (key, entry) in &self.entries {
            let used = self.used.borrow().get(key).copied().unwrap_or(0);
            items.push((used, Kind::Glyph(*key), *entry));
        }
        for (key, entry) in &self.sprites {
            let used = self.sprite_used.borrow().get(key).copied().unwrap_or(0);
            items.push((used, Kind::Sprite(*key), *entry));
        }

        items.sort_by_key(|b| std::cmp::Reverse(b.0));

        items.truncate(items.len().div_ceil(2));

        let total = items.len() as u64;
        let (width, height) = (self.width, self.height);
        let mut pixels = vec![0u8; self.pixels.len()];
        let mut entries = HashMap::new();
        let mut sprites = HashMap::new();
        let mut used = HashMap::new();
        let mut sprite_used = HashMap::new();
        let (mut cx, mut cy, mut rh) = (0u32, 0u32, 0u32);
        let mut age = total;
        for (_, kind, entry) in items {
            age = age.saturating_sub(1);
            let [sx, sy, w, h] = entry.rect;
            if w == 0 || h == 0 || w > width {
                continue;
            }
            if cx + w > width {
                cy += rh + 1;
                cx = 0;
                rh = 0;
            }
            if cy + h > height {
                break;
            }
            let row_bytes = w as usize * ATLAS_BPP;
            for row in 0..h {
                let src = (((sy + row) * width + sx) as usize) * ATLAS_BPP;
                let dst = (((cy + row) * width + cx) as usize) * ATLAS_BPP;
                pixels[dst..dst + row_bytes].copy_from_slice(&self.pixels[src..src + row_bytes]);
            }
            let moved = AtlasEntry {
                rect: [cx, cy, w, h],
                left: entry.left,
                top: entry.top,
                color: entry.color,
            };
            match kind {
                Kind::Glyph(key) => {
                    entries.insert(key, moved);
                    used.insert(key, age);
                }
                Kind::Sprite(key) => {
                    sprites.insert(key, moved);
                    sprite_used.insert(key, age);
                }
            }
            cx += w + 1;
            rh = rh.max(h);
        }
        self.pixels = pixels;
        self.entries = entries;
        self.sprites = sprites;
        *self.used.borrow_mut() = used;
        *self.sprite_used.borrow_mut() = sprite_used;
        self.tick.set(total);
        self.cursor_x = cx;
        self.cursor_y = cy;
        self.row_h = rh;
        self.version += 1;
        self.epoch += 1;
        self.dirty = None;
        self.dirty_full = true;
    }

    fn gc(&mut self) {
        self.pixels.fill(0);
        self.entries.clear();
        self.sprites.clear();
        self.used.borrow_mut().clear();
        self.sprite_used.borrow_mut().clear();
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.row_h = 0;
        self.version += 1;
        self.epoch += 1;
        self.dirty = None;
        self.dirty_full = true;
    }

    fn resize(&mut self, new_w: u32, new_h: u32) {
        let new_w = new_w.clamp(1, ATLAS_MAX_DIM);
        let new_h = new_h.clamp(1, ATLAS_MAX_DIM);
        let bytes = checked_atlas_bytes(new_w, new_h).unwrap_or(0);
        let mut pixels = vec![0u8; bytes];
        let row_bytes = self.width as usize * ATLAS_BPP;
        for row in 0..self.height {
            let src = row as usize * row_bytes;
            let dst = row as usize * (new_w as usize) * ATLAS_BPP;
            pixels[dst..dst + row_bytes].copy_from_slice(&self.pixels[src..src + row_bytes]);
        }
        self.pixels = pixels;
        self.width = new_w;
        self.height = new_h;
        self.version += 1;
        self.epoch += 1;
        self.dirty = None;
        self.dirty_full = true;
    }
}

const SHAPING_CACHE_MAX: usize = 256;

const CARET_CACHE_MAX: usize = 128;

pub type LetterSpacing = f32;

pub(crate) fn sanitize_spacing(spacing: f32) -> f32 {
    if spacing.is_finite() { spacing } else { 0.0 }
}

pub(crate) fn spacing_key(spacing: f32) -> i32 {
    (sanitize_spacing(spacing) * 4.0).round() as i32
}

const DEFAULT_FONT_REGULAR: &[u8] = include_bytes!("../fonts/Inter-Regular.otf");

const DEFAULT_FONT_MEDIUM: &[u8] = include_bytes!("../fonts/Inter-Medium.otf");

const DEFAULT_FONT_SEMIBOLD: &[u8] = include_bytes!("../fonts/Inter-SemiBold.otf");

const DEFAULT_FONT_BOLD: &[u8] = include_bytes!("../fonts/Inter-Bold.otf");

const DEFAULT_FAMILY: &str = "Inter";

const BUNDLED_WEIGHTS: [u16; 4] = [400, 500, 600, 700];

type ShapeKey = (String, u32, u16, String, u8, u32, u8, i32, u32);
type CaretKey = (String, u32, u16, String, u8, u32, i32);

#[derive(Clone)]
struct MruShape {
    text: String,
    size_q: u32,
    weight_v: u16,
    family: Option<String>,
    style_k: u8,
    width_q: u32,
    mode_k: u8,
    spacing_k: i32,
    line_height_k: u32,
    key: ShapeKey,
    shaped: Rc<ShapedText>,
}

pub struct TextSystem {
    font_system: FontSystem,
    swash: SwashCache,
    atlas: GlyphAtlas,
    shaped: HashMap<ShapeKey, Rc<ShapedText>>,

    shaping_order: VecDeque<ShapeKey>,

    mru: Option<MruShape>,

    carets: HashMap<CaretKey, Rc<[f32]>>,

    carets_order: VecDeque<CaretKey>,

    rich: HashMap<rich::RichKey, Rc<rich::RichParagraph>>,

    rich_order: VecDeque<rich::RichKey>,

    lines: HashMap<LineCacheKey, Rc<ShapedLine>>,
    lines_order: VecDeque<LineCacheKey>,

    line_to_key: HashMap<(usize, u32), LineCacheKey>,

    line_reshapes: u64,

    scratch_placed: Vec<(CacheKey, i32, i32)>,
    shape_scratch: Option<Buffer>,
    scale: f32,

    default_family: Option<String>,

    default_weights: Vec<u16>,

    aliases: HashMap<String, String>,

    subpixel_y: bool,
}

impl Default for TextSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl TextSystem {
    pub fn new() -> Self {
        Self::with_atlas(512, 256)
    }

    pub fn with_atlas(width: u32, height: u32) -> Self {
        let mut this = Self {
            font_system: FontSystem::new(),
            swash: SwashCache::new(),
            atlas: GlyphAtlas::new(width, height),
            shaped: HashMap::new(),
            shaping_order: VecDeque::new(),
            mru: None,
            carets: HashMap::new(),
            carets_order: VecDeque::new(),
            rich: HashMap::new(),
            rich_order: VecDeque::new(),
            lines: HashMap::new(),
            lines_order: VecDeque::new(),
            line_to_key: HashMap::new(),
            line_reshapes: 0,
            scratch_placed: Vec::new(),
            shape_scratch: None,
            scale: 1.0,
            default_family: None,
            default_weights: Vec::new(),
            aliases: HashMap::new(),
            subpixel_y: false,
        };

        this.load_font(DEFAULT_FONT_REGULAR.to_vec());
        this.load_font(DEFAULT_FONT_MEDIUM.to_vec());
        this.load_font(DEFAULT_FONT_SEMIBOLD.to_vec());
        this.load_font(DEFAULT_FONT_BOLD.to_vec());
        this.default_family = Some(DEFAULT_FAMILY.to_owned());
        this.refresh_default_weights();
        this
    }

    fn refresh_default_weights(&mut self) {
        let mut weights: Vec<u16> = Vec::new();
        if let Some(family) = self.default_family.as_deref() {
            for face in self.font_system.db().faces() {
                if face.families.iter().any(|(n, _)| n == family)
                    && !weights.contains(&face.weight.0)
                {
                    weights.push(face.weight.0);
                }
            }
            weights.sort_unstable();
        }
        self.default_weights = weights;
    }

    pub(crate) fn resolve_weight_for(&self, weight: FontWeight, eff_family: &str) -> FontWeight {
        let want = weight.value();
        if self.default_weights.contains(&want) {
            return weight;
        }
        if eff_family != DEFAULT_FAMILY {
            return weight;
        }
        let snapped = BUNDLED_WEIGHTS
            .into_iter()
            .min_by(|a, b| a.abs_diff(want).cmp(&b.abs_diff(want)).then(b.cmp(a)))
            .unwrap_or(FontWeight::NORMAL.value());
        FontWeight::new(snapped)
    }

    pub(crate) fn effective_family(&self, family: Option<&str>) -> String {
        let requested = match family {
            Some(f) if !f.is_empty() => f,
            _ => return self.default_family.clone().unwrap_or_default(),
        };
        if let Some(real) = self.aliases.get(requested) {
            return real.clone();
        }
        requested.to_owned()
    }

    fn clear_caches(&mut self) {
        self.shaped.clear();
        self.shaping_order.clear();
        self.mru = None;
        self.carets.clear();
        self.carets_order.clear();
        self.rich.clear();
        self.rich_order.clear();
        self.lines.clear();
        self.lines_order.clear();
        self.line_to_key.clear();
    }

    fn clear_line_cache(&mut self) {
        self.lines.clear();
        self.lines_order.clear();
        self.line_to_key.clear();
    }

    pub fn register_alias(&mut self, alias: impl Into<String>, family: impl Into<String>) {
        let (alias, family) = (alias.into(), family.into());
        if alias.is_empty() || family.is_empty() {
            return;
        }
        if self.aliases.get(&alias).is_some_and(|f| f == &family) {
            return;
        }
        self.aliases.insert(alias, family);
        self.clear_caches();
    }

    pub fn font_families(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for face in self.font_system.db().faces() {
            for (name, _) in face.families.iter() {
                if !out.contains(name) {
                    out.push(name.clone());
                }
            }
        }
        out.sort();
        out
    }

    pub fn set_subpixel_y(&mut self, on: bool) {
        self.subpixel_y = on;
    }

    pub fn subpixel_y(&self) -> bool {
        self.subpixel_y
    }

    pub fn load_font(&mut self, data: Vec<u8>) -> Option<String> {
        let db = self.font_system.db_mut();
        let before = db.faces().count();
        db.load_font_data(data);
        self.mru = None;
        self.refresh_default_weights();
        self.font_system
            .db()
            .faces()
            .nth(before)
            .and_then(|face| face.families.first().map(|(name, _)| name.clone()))
    }

    pub fn set_default_family(&mut self, family: Option<String>) {
        if self.default_family != family {
            self.default_family = family;
            self.clear_caches();
            self.refresh_default_weights();
        }
    }

    pub fn default_family(&self) -> Option<&str> {
        self.default_family.as_deref()
    }

    pub fn set_scale_factor(&mut self, scale: f32) {
        if !scale.is_finite() || scale <= 0.0 {
            return;
        }
        if (scale - self.scale).abs() > f32::EPSILON {
            self.scale = scale;
            self.shaped.clear();
            self.shaping_order.clear();
            self.mru = None;
            self.carets.clear();
            self.carets_order.clear();
            self.rich.clear();
            self.rich_order.clear();
            self.clear_line_cache();
        }
    }

    pub fn scale_factor(&self) -> f32 {
        self.scale
    }

    pub fn shape(&mut self, text: &str, size: f32) -> Rc<ShapedText> {
        self.shape_weighted(text, size, FontWeight::NORMAL)
    }

    pub fn shape_weighted(&mut self, text: &str, size: f32, weight: FontWeight) -> Rc<ShapedText> {
        self.shape_wrapped(text, size, weight, 0.0, WrapMode::default())
    }

    pub fn shape_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        spacing: LetterSpacing,
    ) -> Rc<ShapedText> {
        self.shape_wrapped_spaced(text, size, weight, 0.0, WrapMode::default(), spacing)
    }

    pub fn shape_wrapped(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        width: f32,
        mode: WrapMode,
    ) -> Rc<ShapedText> {
        self.shape_wrapped_spaced(text, size, weight, width, mode, 0.0)
    }

    pub fn shape_wrapped_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
    ) -> Rc<ShapedText> {
        self.shape_wrapped_styled_spaced(
            text,
            size,
            weight,
            width,
            mode,
            spacing,
            None,
            FontStyle::Normal,
        )
    }

    pub fn shape_styled(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        family: Option<&str>,
        style: FontStyle,
    ) -> Rc<ShapedText> {
        self.shape_wrapped_styled_spaced(
            text,
            size,
            weight,
            0.0,
            WrapMode::default(),
            0.0,
            family,
            style,
        )
    }

    pub fn shape_spaced_styled(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
    ) -> Rc<ShapedText> {
        self.shape_wrapped_styled_spaced(
            text,
            size,
            weight,
            0.0,
            WrapMode::default(),
            spacing,
            family,
            style,
        )
    }

    pub fn shape_wrapped_styled(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        width: f32,
        mode: WrapMode,
        family: Option<&str>,
        style: FontStyle,
    ) -> Rc<ShapedText> {
        self.shape_wrapped_styled_spaced(text, size, weight, width, mode, 0.0, family, style)
    }

    #[allow(clippy::too_many_arguments)]
    fn mru_hit(
        &mut self,
        text: &str,
        size_q: u32,
        weight_v: u16,
        family: Option<&str>,
        style_k: u8,
        width_q: u32,
        mode_k: u8,
        spacing_k: i32,
        line_height_k: u32,
    ) -> Option<Rc<ShapedText>> {
        let (key, shaped) = match self.mru.as_ref() {
            Some(mru)
                if mru.text == text
                    && mru.size_q == size_q
                    && mru.weight_v == weight_v
                    && mru.family.as_deref() == family
                    && mru.style_k == style_k
                    && mru.width_q == width_q
                    && mru.mode_k == mode_k
                    && mru.spacing_k == spacing_k
                    && mru.line_height_k == line_height_k =>
            {
                (mru.key.clone(), mru.shaped.clone())
            }
            _ => return None,
        };
        if self.shaping_order.front().is_none_or(|front| front != &key) {
            if let Some(pos) = self.shaping_order.iter().position(|k| k == &key) {
                self.shaping_order.remove(pos);
            }
            self.shaping_order.push_front(key);
        }
        Some(shaped)
    }

    pub fn shape_wrapped_styled_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
    ) -> Rc<ShapedText> {
        self.shape_wrapped_styled_spaced_lh(
            text,
            size,
            weight,
            width,
            mode,
            spacing,
            family,
            style,
            LineHeight::default(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn shape_wrapped_styled_spaced_lh(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
        line_height: LineHeight,
    ) -> Rc<ShapedText> {
        let size_q = (size * 4.0).round() as u32;
        let width_q = (width.max(0.0) * 4.0).round() as u32;
        let spacing = sanitize_spacing(spacing);
        let spacing_k = spacing_key(spacing);
        let line_height_k = line_height.key();

        let weight_raw_v = weight.value();
        if let Some(hit) = self.mru_hit(
            text,
            size_q,
            weight_raw_v,
            family,
            style.key(),
            width_q,
            mode.key(),
            spacing_k,
            line_height_k,
        ) {
            return hit;
        }
        let eff_family = self.effective_family(family);
        let weight = self.resolve_weight_for(weight, &eff_family);
        let key: ShapeKey = (
            text.to_owned(),
            size_q,
            weight.value(),
            eff_family.clone(),
            style.key(),
            width_q,
            mode.key(),
            spacing_k,
            line_height_k,
        );
        if let Some(rc) = self.shaped.get(&key) {
            if let Some(pos) = self.shaping_order.iter().position(|k| k == &key) {
                self.shaping_order.remove(pos);
            }
            self.shaping_order.push_front(key.clone());
            let shaped = rc.clone();
            self.mru = Some(MruShape {
                text: text.to_owned(),
                size_q,
                weight_v: weight_raw_v,
                family: family.map(str::to_owned),
                style_k: style.key(),
                width_q,
                mode_k: mode.key(),
                spacing_k,
                line_height_k,
                key,
                shaped: shaped.clone(),
            });
            return shaped;
        }
        let shaped = self.shape_uncached_styled(
            text,
            size,
            weight,
            &eff_family,
            style,
            width,
            mode,
            spacing,
            line_height,
        );
        self.shaped.insert(key.clone(), shaped.clone());
        self.shaping_order.push_front(key.clone());
        while self.shaped.len() > SHAPING_CACHE_MAX {
            if let Some(oldest) = self.shaping_order.pop_back() {
                self.shaped.remove(&oldest);
            } else {
                break;
            }
        }
        self.mru = Some(MruShape {
            text: text.to_owned(),
            size_q,
            weight_v: weight_raw_v,
            family: family.map(str::to_owned),
            style_k: style.key(),
            width_q,
            mode_k: mode.key(),
            spacing_k,
            line_height_k,
            key,
            shaped: shaped.clone(),
        });
        shaped
    }

    pub fn line_hash(
        line_text: &str,
        spans: &[StyledSpan],
        line_byte_start: usize,
        width: f32,
    ) -> u64 {
        sequoia_buffer::line_hash(line_text, &rebase_spans(spans, line_byte_start), width)
    }

    pub fn hash_line_content(
        text: &str,
        extra_hash: u64,
        size: f32,
        weight: FontWeight,
        family: &str,
        style: FontStyle,
        spacing: f32,
        mode: WrapMode,
        scale_bits: u32,
    ) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        text.hash(&mut h);
        extra_hash.hash(&mut h);
        ((size * 4.0).round() as u32).hash(&mut h);
        weight.value().hash(&mut h);
        family.hash(&mut h);
        style.key().hash(&mut h);
        spacing_key(sanitize_spacing(spacing)).hash(&mut h);
        mode.key().hash(&mut h);
        scale_bits.hash(&mut h);
        h.finish()
    }

    fn line_key_for(
        &self,
        text: &str,
        extra_hash: u64,
        size: f32,
        weight: FontWeight,
        eff_family: &str,
        style: FontStyle,
        spacing: f32,
        width: f32,
        mode: WrapMode,
    ) -> LineCacheKey {
        let hash = Self::hash_line_content(
            text,
            extra_hash,
            size,
            weight,
            eff_family,
            style,
            spacing,
            mode,
            self.scale.to_bits(),
        );
        let width_q = (width.max(0.0) * 4.0).round() as u32;
        (hash, width_q)
    }

    pub fn shape_line_full(
        &mut self,
        line: usize,
        text: &str,
        size: f32,
        weight: FontWeight,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
        extra_hash: u64,
    ) -> Rc<ShapedLine> {
        let spacing = sanitize_spacing(spacing);
        let eff_family = self.effective_family(family);
        let weight = self.resolve_weight_for(weight, &eff_family);
        let width_q = (width.max(0.0) * 4.0).round() as u32;
        let key = self.line_key_for(
            text,
            extra_hash,
            size,
            weight,
            &eff_family,
            style,
            spacing,
            width,
            mode,
        );
        if let Some(rc) = self.lines.get(&key) {
            if let Some(pos) = self.lines_order.iter().position(|k| k == &key) {
                self.lines_order.remove(pos);
            }
            self.lines_order.push_front(key);
            self.line_to_key.insert((line, width_q), key);
            return rc.clone();
        }
        let shaped = self.shape_line_uncached(
            text,
            size,
            weight,
            &eff_family,
            style,
            width,
            mode,
            spacing,
            key.0,
            width_q,
        );
        self.line_reshapes += 1;
        self.lines.insert(key, shaped.clone());
        self.lines_order.push_front(key);
        self.line_to_key.insert((line, width_q), key);
        while self.lines.len() > LINE_CACHE_MAX {
            if let Some(oldest) = self.lines_order.pop_back() {
                self.lines.remove(&oldest);

                self.line_to_key.retain(|_, k| k != &oldest);
            } else {
                break;
            }
        }
        shaped
    }

    pub fn shape_line_wrapped(
        &mut self,
        line: usize,
        text: &str,
        size: f32,
        weight: FontWeight,
        width: f32,
        mode: WrapMode,
    ) -> Rc<ShapedLine> {
        self.shape_line_full(
            line,
            text,
            size,
            weight,
            width,
            mode,
            0.0,
            None,
            FontStyle::Normal,
            0,
        )
    }

    pub fn shape_line(&mut self, line: usize, text: &str, size: f32) -> Rc<ShapedLine> {
        self.shape_line_wrapped(
            line,
            text,
            size,
            FontWeight::NORMAL,
            0.0,
            WrapMode::default(),
        )
    }

    pub fn shape_lines_visible<F>(
        &mut self,
        range: std::ops::Range<usize>,
        get_line: F,
        size: f32,
        weight: FontWeight,
        width: f32,
        mode: WrapMode,
    ) -> Vec<Rc<ShapedLine>>
    where
        F: Fn(usize) -> Option<String>,
    {
        let mut out = Vec::new();
        let (start, end) = (range.start, range.end);
        out.reserve(end.saturating_sub(start));
        for line in start..end {
            if let Some(text) = get_line(line) {
                out.push(self.shape_line_wrapped(line, &text, size, weight, width, mode));
            }
        }
        out
    }

    pub fn shape_lines_visible_full<F>(
        &mut self,
        range: std::ops::Range<usize>,
        get_line: F,
        size: f32,
        weight: FontWeight,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
        extra_hash_for: &dyn Fn(usize) -> u64,
    ) -> Vec<Rc<ShapedLine>>
    where
        F: Fn(usize) -> Option<String>,
    {
        let mut out = Vec::with_capacity(range.end.saturating_sub(range.start));
        for line in range {
            if let Some(text) = get_line(line) {
                let extra = extra_hash_for(line);
                out.push(self.shape_line_full(
                    line, &text, size, weight, width, mode, spacing, family, style, extra,
                ));
            }
        }
        out
    }

    #[allow(clippy::too_many_arguments)]
    pub fn shape_styled_line(
        &mut self,
        line: usize,
        line_text: &str,
        line_byte_start: usize,
        spans: &[StyledSpan],
        size: f32,
        weight: FontWeight,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
    ) -> Rc<ShapedLine> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;

        let extra = if spans.iter().all(|s| s.is_empty()) {
            0
        } else {
            let mut h = DefaultHasher::new();
            hash_spans_rebased(spans, line_byte_start, &mut h);
            h.finish()
        };
        self.shape_line_full(
            line, line_text, size, weight, width, mode, spacing, family, style, extra,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn shape_lines_visible_buffer(
        &mut self,
        buf: &StyledBuffer,
        range: std::ops::Range<usize>,
        size: f32,
        weight: FontWeight,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
    ) -> Vec<Rc<ShapedLine>> {
        let line_count = buf.lines().line_count();
        let end = range.end.min(line_count);
        let mut out = Vec::with_capacity(end.saturating_sub(range.start));
        for line in range.start..end {
            let (s, e) = buf.lines().line_range(line);
            let text = buf.substring(s..e);
            let spans = buf.spans_for_line(s..e);
            out.push(self.shape_styled_line(
                line, &text, s, &spans, size, weight, width, mode, spacing, family, style,
            ));
        }
        out
    }

    pub fn shape_rich_from_styled(
        &mut self,
        line_text: &str,
        line_byte_start: usize,
        styled: &[StyledSpan],
        size: f32,
        width: f32,
        mode: WrapMode,
        spacing: f32,
        align: rich::RichAlign,
    ) -> Rc<rich::RichParagraph> {
        let spans = styled_spans_to_text_spans(line_text, line_byte_start, styled);
        self.shape_rich_full(&spans, size, width, mode, spacing, align)
    }

    pub fn invalidate_lines(&mut self, edited: std::ops::Range<usize>) {
        if edited.is_empty() {
            return;
        }
        let mut dropped: Vec<LineCacheKey> = Vec::new();
        self.line_to_key.retain(|(ln, _), key| {
            if edited.contains(ln) {
                dropped.push(*key);
                false
            } else {
                true
            }
        });
        for key in dropped {
            let still_used = self.line_to_key.values().any(|k| k == &key);
            if !still_used {
                self.lines.remove(&key);
                if let Some(pos) = self.lines_order.iter().position(|k| k == &key) {
                    self.lines_order.remove(pos);
                }
            }
        }
    }

    pub fn invalidate_for_edit(
        &mut self,
        index: &LineIndex,
        edit_byte_start: usize,
        edit_byte_end: usize,
    ) {
        let a = index.line_of(edit_byte_start);
        let b = index.line_of(edit_byte_end);
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        self.invalidate_lines(lo..hi.saturating_add(1));
    }

    pub fn apply_edit_invalidate(
        &mut self,
        edit_byte_start: usize,
        edit_byte_end: usize,
        line_of: &dyn Fn(usize) -> usize,
    ) {
        let a = line_of(edit_byte_start);
        let b = line_of(edit_byte_end);
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        self.invalidate_lines(lo..hi.saturating_add(1));
    }

    pub fn line_reshapes(&self) -> u64 {
        self.line_reshapes
    }

    pub fn reset_line_reshapes(&mut self) {
        self.line_reshapes = 0;
    }

    pub fn line_cache_len(&self) -> usize {
        self.lines.len()
    }

    fn shape_line_uncached(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        family: &str,
        style: FontStyle,
        wrap_width: f32,
        wrap_mode: WrapMode,
        spacing: f32,
        hash: u64,
        width_q: u32,
    ) -> Rc<ShapedLine> {
        let spacing = sanitize_spacing(spacing);
        let scale = self.scale;
        let px = size * scale;
        let line_h_px = px * 1.35;
        let mut buffer = self
            .shape_scratch
            .take()
            .unwrap_or_else(|| Buffer::new(&mut self.font_system, Metrics::new(px, line_h_px)));
        buffer.set_metrics(Metrics::new(px, line_h_px));
        buffer.set_wrap(wrap_mode.cosmic());
        buffer.set_size(
            if wrap_width > 0.0 {
                Some(wrap_width * scale)
            } else {
                None
            },
            None,
        );
        let mut attrs = Attrs::new()
            .weight(Weight(weight.value()))
            .style(style.cosmic());
        if !family.is_empty() {
            attrs = attrs.family(Family::Name(family));
        }

        debug_assert!(!text.contains('\n'), "shape_line: single logical line");
        let single = text.replace('\n', " ");
        buffer.set_text(&single, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut self.font_system, false);

        let sp_px = spacing * scale;
        let mut glyphs = Vec::new();
        let mut rows: Vec<LineRow> = Vec::new();
        let mut width_px = 0.0f32;
        let mut height_px = 0.0f32;
        let mut first_top: Option<f32> = None;
        let mut first_y: Option<f32> = None;
        for run in buffer.layout_runs() {
            let base_top = *first_top.get_or_insert(run.line_top);
            let base_y = *first_y.get_or_insert(run.line_y);
            let top = (run.line_top - base_top) / scale;
            let baseline = (run.line_y - base_y) / scale;
            let h = run.line_height / scale;
            let extra = sp_px * run.glyphs.len().saturating_sub(1) as f32;

            let w_phys = run.line_w + extra;
            let w = w_phys / scale;
            width_px = width_px.max(w_phys);
            height_px = (top + h).max(height_px);
            rows.push(LineRow {
                top,
                height: h,
                baseline,
                width: w,
            });
            for (j, g) in run.glyphs.iter().enumerate() {
                let p = g.physical((0.0, 0.0), 1.0);
                let x_offset = g.font_size * g.x_offset;
                let y_offset = g.font_size * g.y_offset;
                glyphs.push(ShapedGlyph {
                    x: g.x + x_offset + j as f32 * sp_px,
                    y: (run.line_y - base_y) + (g.y - y_offset),
                    key: p.cache_key,
                });
            }
        }
        if rows.is_empty() {
            rows.push(LineRow {
                top: 0.0,
                height: line_h_px / scale,
                baseline: (line_h_px / scale) * 0.5,
                width: 0.0,
            });
            height_px = line_h_px / scale;
        }
        let out = Rc::new(ShapedLine {
            glyphs,
            rows,
            width: width_px / scale,
            height: height_px,
            hash,
            width_q,
        });
        self.shape_scratch = Some(buffer);
        out
    }

    pub fn draw_line(
        &mut self,
        line: &ShapedLine,
        origin: (f32, f32),
        color: Color,
        list: &mut RenderList,
    ) {
        if line.glyphs.is_empty() {
            return;
        }
        let scale = self.scale;
        let (ox, oy) = (origin.0 * scale, origin.1 * scale);
        let mut placed = std::mem::take(&mut self.scratch_placed);
        placed.clear();
        for g in &line.glyphs {
            let x = ox + g.x;
            let y = if self.subpixel_y {
                oy + g.y
            } else {
                (oy + g.y).trunc()
            };
            let (key, x_int, y_int) = CacheKey::new(
                g.key.font_id,
                g.key.glyph_id,
                f32::from_bits(g.key.font_size_bits),
                (x, y),
                g.key.font_weight,
                g.key.flags,
            );
            if self.atlas.entry(&key).is_none() {
                self.rasterize(key);
            }
            placed.push((key, x_int, y_int));
        }
        let (aw, ah) = (self.atlas.width as f32, self.atlas.height as f32);
        let mut quads = Vec::with_capacity(placed.len());
        for &(key, x_int, y_int) in placed.iter() {
            let Some(entry) = self.atlas.entry(&key) else {
                continue;
            };
            let [rx, ry, rw, rh] = entry.rect;
            if rw == 0 || rh == 0 {
                continue;
            }
            let x = (x_int + entry.left) as f32 / scale;
            let y = (y_int - entry.top) as f32 / scale;
            quads.push(GlyphQuad {
                rect: Rect::from_xywh(x, y, rw as f32 / scale, rh as f32 / scale),
                uv: [
                    rx as f32 / aw,
                    ry as f32 / ah,
                    (rx + rw) as f32 / aw,
                    (ry + rh) as f32 / ah,
                ],
                color_glyph: entry.color,
            });
        }
        self.scratch_placed = placed;
        list.push_text(TextDraw {
            color,
            glyphs: quads,
        });
    }

    pub fn text_width(&mut self, text: &str, size: f32) -> f32 {
        self.shape(text, size).width()
    }

    pub fn text_width_weighted(&mut self, text: &str, size: f32, weight: FontWeight) -> f32 {
        self.shape_weighted(text, size, weight).width()
    }

    pub fn text_width_spaced(&mut self, text: &str, size: f32, spacing: LetterSpacing) -> f32 {
        self.shape_spaced(text, size, FontWeight::NORMAL, spacing)
            .width()
    }

    pub fn text_width_weighted_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        spacing: LetterSpacing,
    ) -> f32 {
        self.shape_spaced(text, size, weight, spacing).width()
    }

    pub fn text_width_styled_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
    ) -> f32 {
        self.shape_spaced_styled(text, size, weight, spacing, family, style)
            .width()
    }

    pub fn caret_positions(&mut self, text: &str, size: f32, weight: FontWeight) -> Vec<f32> {
        self.caret_positions_spaced(text, size, weight, 0.0)
    }

    pub fn caret_positions_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        spacing: LetterSpacing,
    ) -> Vec<f32> {
        self.caret_positions_styled_spaced(text, size, weight, spacing, None, FontStyle::Normal)
    }

    pub fn caret_positions_styled_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
    ) -> Vec<f32> {
        let eff_family = self.effective_family(family);
        let weight = self.resolve_weight_for(weight, &eff_family);
        let spacing = sanitize_spacing(spacing);
        let key: CaretKey = (
            text.to_owned(),
            (size * 4.0).round() as u32,
            weight.value(),
            eff_family.clone(),
            style.key(),
            self.scale.to_bits(),
            spacing_key(spacing),
        );
        if let Some(rc) = self.carets.get(&key) {
            if let Some(pos) = self.carets_order.iter().position(|k| k == &key) {
                self.carets_order.remove(pos);
            }
            self.carets_order.push_front(key);
            return rc.to_vec();
        }
        let out =
            self.caret_positions_uncached_styled(text, size, weight, &eff_family, style, spacing);
        self.carets.insert(key.clone(), Rc::from(out.clone()));
        self.carets_order.push_front(key);
        while self.carets.len() > CARET_CACHE_MAX {
            if let Some(oldest) = self.carets_order.pop_back() {
                self.carets.remove(&oldest);
            } else {
                break;
            }
        }
        out
    }

    fn caret_positions_uncached_styled(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        family: &str,
        style: FontStyle,
        spacing: f32,
    ) -> Vec<f32> {
        let char_count = text.chars().count();
        if text.is_empty() {
            return vec![0.0];
        }

        let mut byte_offsets: Vec<usize> = Vec::with_capacity(char_count + 1);
        byte_offsets.push(0);
        for (i, _) in text.char_indices() {
            if i != 0 {
                byte_offsets.push(i);
            }
        }
        byte_offsets.push(text.len());
        debug_assert_eq!(byte_offsets.len(), char_count + 1);

        let scale = self.scale;
        let px = size * scale;
        let mut buffer = self
            .shape_scratch
            .take()
            .unwrap_or_else(|| Buffer::new(&mut self.font_system, Metrics::new(px, px * 1.35)));
        buffer.set_metrics(Metrics::new(px, px * 1.35));
        buffer.set_wrap(cosmic_text::Wrap::None);
        buffer.set_size(None, None);
        let mut attrs = Attrs::new()
            .weight(Weight(weight.value()))
            .style(style.cosmic());
        if !family.is_empty() {
            attrs = attrs.family(Family::Name(family));
        }
        buffer.set_text(text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut self.font_system, false);

        let mut out = vec![0.0f32; char_count + 1];
        let mut line_w_px = 0.0f32;
        for run in buffer.layout_runs() {
            line_w_px = line_w_px.max(run.line_w);
            for g in run.glyphs {
                let gx = g.x;
                let gw = g.w;

                for (ci, &bo) in byte_offsets.iter().enumerate() {
                    if bo >= g.end {
                        let end_x = gx + gw;
                        if end_x / scale > out[ci] {
                            out[ci] = end_x / scale;
                        }
                    } else if bo == g.start && gx / scale > out[ci] {
                        out[ci] = gx / scale;
                    }
                }
            }
        }

        let total = line_w_px / scale;
        let mut prev = 0.0f32;
        for v in out.iter_mut() {
            if *v < prev {
                *v = prev;
            }
            if *v > total {
                *v = total;
            }
            prev = *v;
        }
        if let Some(last) = out.last_mut() {
            *last = total;
        }

        if spacing != 0.0 && char_count > 0 {
            let n = char_count;
            for (i, v) in out.iter_mut().enumerate() {
                if i < n {
                    *v += spacing * i as f32;
                }
            }
            let width = (total + spacing * n.saturating_sub(1) as f32).max(0.0);
            if let Some(last) = out.last_mut() {
                *last = width;
            }
            let mut prev = 0.0f32;
            for v in out.iter_mut().take(n) {
                if *v < prev {
                    *v = prev;
                }
                prev = *v;
            }
            if let Some(last) = out.last_mut() {
                *last = width;
            }
        }
        self.shape_scratch = Some(buffer);
        out
    }

    pub fn caret_x(&mut self, text: &str, size: f32, weight: FontWeight, char_index: usize) -> f32 {
        self.caret_x_spaced(text, size, weight, char_index, 0.0)
    }

    pub fn caret_x_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        char_index: usize,
        spacing: LetterSpacing,
    ) -> f32 {
        self.caret_x_styled_spaced(
            text,
            size,
            weight,
            char_index,
            spacing,
            None,
            FontStyle::Normal,
        )
    }

    pub fn caret_x_styled_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        char_index: usize,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
    ) -> f32 {
        let pos = self.caret_positions_styled_spaced(text, size, weight, spacing, family, style);
        pos.get(char_index.min(pos.len() - 1))
            .copied()
            .unwrap_or(0.0)
    }

    pub fn char_index_from_x(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        x: f32,
    ) -> usize {
        self.char_index_from_x_spaced(text, size, weight, x, 0.0)
    }

    pub fn char_index_from_x_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        x: f32,
        spacing: LetterSpacing,
    ) -> usize {
        self.char_index_from_x_styled_spaced(
            text,
            size,
            weight,
            x,
            spacing,
            None,
            FontStyle::Normal,
        )
    }

    pub fn char_index_from_x_styled_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        x: f32,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
    ) -> usize {
        let pos = self.caret_positions_styled_spaced(text, size, weight, spacing, family, style);
        if pos.len() <= 1 {
            return 0;
        }
        if x <= 0.0 {
            return 0;
        }
        if x >= *pos.last().unwrap_or(&0.0) {
            return pos.len() - 1;
        }
        let mut best = 0;
        let mut best_d = f32::MAX;
        for (i, &px) in pos.iter().enumerate() {
            let d = (px - x).abs();
            if d < best_d {
                best_d = d;
                best = i;
            }
        }
        best
    }

    pub fn truncate(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        max_width: f32,
    ) -> String {
        self.truncate_spaced(text, size, weight, max_width, 0.0)
    }

    pub fn truncate_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        max_width: f32,
        spacing: LetterSpacing,
    ) -> String {
        self.truncate_styled_spaced(
            text,
            size,
            weight,
            max_width,
            spacing,
            None,
            FontStyle::Normal,
        )
    }

    pub fn truncate_styled_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        max_width: f32,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
    ) -> String {
        let spacing = sanitize_spacing(spacing);
        if max_width <= 0.0 || text.is_empty() {
            return String::new();
        }
        if self.measure_width_styled_spaced(text, size, weight, spacing, family, style) <= max_width
        {
            return text.to_owned();
        }
        let chars: Vec<char> = text.chars().collect();
        let mut lo = 0usize;
        let mut hi = chars.len();
        while lo < hi {
            let mid = (lo + hi).div_ceil(2);
            let prefix: String = chars[..mid].iter().collect();
            if self.measure_width_styled_spaced(&prefix, size, weight, spacing, family, style)
                <= max_width
            {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        chars[..lo].iter().collect()
    }

    pub fn elide(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        max_width: f32,
        mode: ElideMode,
    ) -> String {
        self.elide_spaced(text, size, weight, max_width, mode, 0.0)
    }

    pub fn elide_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        max_width: f32,
        mode: ElideMode,
        spacing: LetterSpacing,
    ) -> String {
        self.elide_styled_spaced(
            text,
            size,
            weight,
            max_width,
            mode,
            spacing,
            None,
            FontStyle::Normal,
        )
    }

    pub fn elide_styled_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        max_width: f32,
        mode: ElideMode,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
    ) -> String {
        let spacing = sanitize_spacing(spacing);
        if max_width <= 0.0 || text.is_empty() {
            return String::new();
        }
        if self.measure_width_styled_spaced(text, size, weight, spacing, family, style) <= max_width
        {
            return text.to_owned();
        }
        let ell = ELLIPSIS.to_string();
        if self.measure_width_styled_spaced(&ell, size, weight, spacing, family, style) > max_width
        {
            return ell;
        }
        let chars: Vec<char> = text.chars().collect();
        let mut fits = |s: &str| {
            self.measure_width_styled_spaced(s, size, weight, spacing, family, style) <= max_width
        };
        match mode {
            ElideMode::Right => {
                let mut lo = 0usize;
                let mut hi = chars.len();
                while lo < hi {
                    let mid = (lo + hi).div_ceil(2);
                    let cand: String = chars[..mid].iter().collect::<String>() + &ell;
                    if fits(&cand) {
                        lo = mid;
                    } else {
                        hi = mid - 1;
                    }
                }
                chars[..lo].iter().collect::<String>() + &ell
            }
            ElideMode::Left => {
                let mut lo = 0usize;
                let mut hi = chars.len();
                while lo < hi {
                    let mid = (lo + hi).div_ceil(2);
                    let cand: String =
                        ell.clone() + &chars[chars.len() - mid..].iter().collect::<String>();
                    if fits(&cand) {
                        lo = mid;
                    } else {
                        hi = mid - 1;
                    }
                }
                ell + &chars[chars.len() - lo..].iter().collect::<String>()
            }
            ElideMode::Middle => {
                let mut lo = 0usize;
                let mut hi = chars.len();
                while lo < hi {
                    let mid = (lo + hi).div_ceil(2);
                    let front = mid.div_ceil(2);
                    let back = mid / 2;
                    let cand: String = chars[..front].iter().collect::<String>()
                        + &ell
                        + &chars[chars.len() - back..].iter().collect::<String>();
                    if fits(&cand) {
                        lo = mid;
                    } else {
                        hi = mid - 1;
                    }
                }
                let front = lo.div_ceil(2);
                let back = lo / 2;
                chars[..front].iter().collect::<String>()
                    + &ell
                    + &chars[chars.len() - back..].iter().collect::<String>()
            }
        }
    }

    /// Returns the width of the longest unbreakable unit for the wrap mode.
    ///
    /// For word wrapping this is the longest whitespace-separated word; for
    /// character wrapping it is the widest single character.
    #[allow(clippy::too_many_arguments)]
    pub fn min_content_width_styled_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
        mode: WrapMode,
    ) -> f32 {
        match mode {
            WrapMode::Character => text
                .chars()
                .map(|ch| {
                    let mut buf = [0u8; 4];
                    self.measure_width_styled_spaced(
                        ch.encode_utf8(&mut buf),
                        size,
                        weight,
                        spacing,
                        family,
                        style,
                    )
                })
                .fold(0.0_f32, f32::max),
            _ => text
                .split_whitespace()
                .map(|word| {
                    self.measure_width_styled_spaced(word, size, weight, spacing, family, style)
                })
                .fold(0.0_f32, f32::max),
        }
    }

    /// Elides wrapped text to at most `max_lines`, appending an ellipsis to the
    /// last visible line when content is cut.
    #[allow(clippy::too_many_arguments)]
    pub fn elide_lines_styled_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        width: f32,
        mode: WrapMode,
        spacing: LetterSpacing,
        family: Option<&str>,
        style: FontStyle,
        line_height: LineHeight,
        max_lines: usize,
    ) -> String {
        if text.is_empty() || max_lines == 0 || width <= 0.0 {
            return String::new();
        }
        let full = self.shape_wrapped_styled_spaced_lh(
            text,
            size,
            weight,
            width,
            mode,
            spacing,
            family,
            style,
            line_height,
        );
        if full.line_count() <= max_lines {
            return text.to_owned();
        }
        let chars: Vec<char> = text.chars().collect();
        let ell = ELLIPSIS.to_string();
        let mut lo = 0usize;
        let mut hi = chars.len();
        while lo < hi {
            let mid = (lo + hi).div_ceil(2);
            let candidate: String = chars[..mid].iter().collect::<String>() + &ell;
            let shaped = self.shape_wrapped_styled_spaced_lh(
                &candidate,
                size,
                weight,
                width,
                mode,
                spacing,
                family,
                style,
                line_height,
            );
            if shaped.line_count() <= max_lines {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        chars[..lo].iter().collect::<String>() + &ell
    }

    fn measure_width_styled_spaced(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        spacing: f32,
        family: Option<&str>,
        style: FontStyle,
    ) -> f32 {
        if text.is_empty() {
            return 0.0;
        }
        let eff_family = self.effective_family(family);
        let weight = self.resolve_weight_for(weight, &eff_family);
        let spacing = sanitize_spacing(spacing);
        let scale = self.scale;
        let px = size * scale;
        let mut buffer = Buffer::new(&mut self.font_system, Metrics::new(px, px * 1.35));
        let mut attrs = Attrs::new()
            .weight(Weight(weight.value()))
            .style(style.cosmic());
        if !eff_family.is_empty() {
            attrs = attrs.family(Family::Name(&eff_family));
        }
        buffer.set_text(text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut self.font_system, false);
        let sp_px = spacing * scale;
        buffer
            .layout_runs()
            .map(|r| r.line_w + sp_px * r.glyphs.len().saturating_sub(1) as f32)
            .fold(0.0f32, f32::max)
            / scale
    }

    #[allow(clippy::too_many_arguments)]
    fn shape_uncached_styled(
        &mut self,
        text: &str,
        size: f32,
        weight: FontWeight,
        family: &str,
        style: FontStyle,
        wrap_width: f32,
        wrap_mode: WrapMode,
        spacing: f32,
        line_height: LineHeight,
    ) -> Rc<ShapedText> {
        let spacing = sanitize_spacing(spacing);
        let scale = self.scale;
        let px = size * scale;
        let line_h = line_height.resolve(size).max(1.0);
        let line_h_px = line_h * scale;
        let mut buffer = self
            .shape_scratch
            .take()
            .unwrap_or_else(|| Buffer::new(&mut self.font_system, Metrics::new(px, line_h_px)));
        buffer.set_metrics(Metrics::new(px, line_h_px));
        buffer.set_wrap(wrap_mode.cosmic());
        buffer.set_size(
            if wrap_width > 0.0 {
                Some(wrap_width * scale)
            } else {
                None
            },
            None,
        );
        let mut attrs = Attrs::new()
            .weight(Weight(weight.value()))
            .style(style.cosmic());
        if !family.is_empty() {
            attrs = attrs.family(Family::Name(family));
        }
        buffer.set_text(text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut self.font_system, false);

        let sp_px = spacing * scale;
        let mut glyphs = Vec::new();
        let mut rows = Vec::new();
        let mut width_px = 0.0_f32;
        let mut height_px = line_h_px;

        let mut first_line_top: Option<f32> = None;
        let mut first_line_y: Option<f32> = None;
        for run in buffer.layout_runs() {
            let base_top = *first_line_top.get_or_insert(run.line_top);
            let extra = sp_px * run.glyphs.len().saturating_sub(1) as f32;
            width_px = width_px.max(run.line_w + extra);
            height_px = height_px.max(run.line_top - base_top + run.line_height);
            rows.push(LineRow {
                top: (run.line_top - base_top) / scale,
                height: run.line_height / scale,
                baseline: (run.line_y - base_top) / scale,
                width: (run.line_w + extra) / scale,
            });
            let dy_base = run.line_y - *first_line_y.get_or_insert(run.line_y);
            for (j, g) in run.glyphs.iter().enumerate() {
                let p = g.physical((0.0, 0.0), 1.0);
                let x_offset = g.font_size * g.x_offset;
                let y_offset = g.font_size * g.y_offset;
                glyphs.push(ShapedGlyph {
                    x: g.x + x_offset + j as f32 * sp_px,

                    y: dy_base + (g.y - y_offset),
                    key: p.cache_key,
                });
            }
        }
        let out = Rc::new(ShapedText {
            glyphs,
            rows,
            width: width_px / scale,
            height: height_px / scale,
            line_height: line_h,
        });
        self.shape_scratch = Some(buffer);
        out
    }

    pub fn draw(
        &mut self,
        shaped: &ShapedText,
        origin: (f32, f32),
        color: Color,
        list: &mut RenderList,
    ) {
        if shaped.glyphs.is_empty() {
            return;
        }
        let scale = self.scale;
        let (ox, oy) = (origin.0 * scale, origin.1 * scale);

        let mut placed = std::mem::take(&mut self.scratch_placed);
        placed.clear();
        for g in &shaped.glyphs {
            let x = ox + g.x;

            let y = if self.subpixel_y {
                oy + g.y
            } else {
                (oy + g.y).trunc()
            };
            let (key, x_int, y_int) = CacheKey::new(
                g.key.font_id,
                g.key.glyph_id,
                f32::from_bits(g.key.font_size_bits),
                (x, y),
                g.key.font_weight,
                g.key.flags,
            );
            if self.atlas.entry(&key).is_none() {
                self.rasterize(key);
            }
            placed.push((key, x_int, y_int));
        }

        let (aw, ah) = (self.atlas.width as f32, self.atlas.height as f32);
        let mut quads = Vec::with_capacity(placed.len());
        for &(key, x_int, y_int) in placed.iter() {
            let Some(entry) = self.atlas.entry(&key) else {
                continue;
            };
            let [rx, ry, rw, rh] = entry.rect;
            if rw == 0 || rh == 0 {
                continue;
            }

            let x = (x_int + entry.left) as f32 / scale;
            let y = (y_int - entry.top) as f32 / scale;
            quads.push(GlyphQuad {
                rect: Rect::from_xywh(x, y, rw as f32 / scale, rh as f32 / scale),
                uv: [
                    rx as f32 / aw,
                    ry as f32 / ah,
                    (rx + rw) as f32 / aw,
                    (ry + rh) as f32 / ah,
                ],
                color_glyph: entry.color,
            });
        }

        self.scratch_placed = placed;
        list.push_text(TextDraw {
            color,
            glyphs: quads,
        });
    }

    pub fn draw_sprite(
        &mut self,
        key: u64,
        w: u32,
        h: u32,
        alpha: &[u8],
        dst: Rect,
        color: Color,
        list: &mut RenderList,
    ) {
        if self.atlas.sprite(key).is_none() {
            self.atlas.insert_sprite(key, w, h, alpha);
        }
        let Some(entry) = self.atlas.sprite(key) else {
            return;
        };
        let [rx, ry, rw, rh] = entry.rect;
        if rw == 0 || rh == 0 {
            return;
        }
        let scale = self.scale;
        let rect = Rect::from_xywh(
            (dst.left() * scale).round() / scale,
            (dst.top() * scale).round() / scale,
            rw as f32 / scale,
            rh as f32 / scale,
        );
        let (aw, ah) = (self.atlas.width as f32, self.atlas.height as f32);
        list.push_text(TextDraw {
            color,
            glyphs: vec![GlyphQuad {
                rect,
                uv: [
                    rx as f32 / aw,
                    ry as f32 / ah,
                    (rx + rw) as f32 / aw,
                    (ry + rh) as f32 / ah,
                ],
                color_glyph: false,
            }],
        });
    }

    pub fn shape_rich(&mut self, spans: &[rich::TextSpan], size: f32) -> Rc<rich::RichParagraph> {
        self.shape_rich_full(
            spans,
            size,
            0.0,
            WrapMode::default(),
            0.0,
            rich::RichAlign::Left,
        )
    }

    pub fn shape_rich_wrapped(
        &mut self,
        spans: &[rich::TextSpan],
        size: f32,
        width: f32,
        mode: WrapMode,
    ) -> Rc<rich::RichParagraph> {
        self.shape_rich_full(spans, size, width, mode, 0.0, rich::RichAlign::Left)
    }

    pub fn shape_rich_aligned(
        &mut self,
        spans: &[rich::TextSpan],
        size: f32,
        width: f32,
        mode: WrapMode,
        align: rich::RichAlign,
    ) -> Rc<rich::RichParagraph> {
        self.shape_rich_full(spans, size, width, mode, 0.0, align)
    }

    pub fn shape_rich_full(
        &mut self,
        spans: &[rich::TextSpan],
        size: f32,
        width: f32,
        mode: WrapMode,
        spacing: f32,
        align: rich::RichAlign,
    ) -> Rc<rich::RichParagraph> {
        self.shape_rich_full_lh(
            spans,
            size,
            width,
            mode,
            spacing,
            align,
            LineHeight::default(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn shape_rich_full_lh(
        &mut self,
        spans: &[rich::TextSpan],
        size: f32,
        width: f32,
        mode: WrapMode,
        spacing: f32,
        align: rich::RichAlign,
        line_height: LineHeight,
    ) -> Rc<rich::RichParagraph> {
        let spacing = sanitize_spacing(spacing);
        let key = rich::RichKey::new(spans, size, width, mode.key(), spacing, align, line_height);
        if let Some(rc) = self.rich.get(&key) {
            if let Some(pos) = self.rich_order.iter().position(|k| k == &key) {
                self.rich_order.remove(pos);
            }
            self.rich_order.push_front(key);
            return rc.clone();
        }
        let shaped =
            self.shape_rich_uncached(spans, size, width, mode, spacing, align, line_height);
        self.rich.insert(key.clone(), shaped.clone());
        self.rich_order.push_front(key);
        while self.rich.len() > rich::RICH_CACHE_MAX {
            if let Some(oldest) = self.rich_order.pop_back() {
                self.rich.remove(&oldest);
            } else {
                break;
            }
        }
        shaped
    }

    /// Elides rich text to at most `max_lines`, appending an ellipsis to the
    /// last visible line when content is cut.
    #[allow(clippy::too_many_arguments)]
    pub fn elide_rich_lines(
        &mut self,
        spans: &[rich::TextSpan],
        size: f32,
        width: f32,
        mode: WrapMode,
        spacing: f32,
        align: rich::RichAlign,
        line_height: LineHeight,
        max_lines: usize,
    ) -> Vec<rich::TextSpan> {
        if max_lines == 0 || width <= 0.0 || spans.iter().all(|s| s.text.is_empty()) {
            return Vec::new();
        }
        let full = self.shape_rich_full_lh(spans, size, width, mode, spacing, align, line_height);
        if full.line_count() <= max_lines {
            return spans.to_vec();
        }
        let char_count = full.len_chars();
        let mut lo = 0usize;
        let mut hi = char_count;
        while lo < hi {
            let mid = (lo + hi).div_ceil(2);
            let mut candidate = rich::truncate_spans(spans, mid);
            rich::append_ellipsis(&mut candidate);
            let shaped =
                self.shape_rich_full_lh(&candidate, size, width, mode, spacing, align, line_height);
            if shaped.line_count() <= max_lines {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        let mut out = rich::truncate_spans(spans, lo);
        rich::append_ellipsis(&mut out);
        out
    }

    fn shape_rich_uncached(
        &mut self,
        spans: &[rich::TextSpan],
        size: f32,
        width: f32,
        mode: WrapMode,
        spacing: f32,
        align: rich::RichAlign,
        line_height: LineHeight,
    ) -> Rc<rich::RichParagraph> {
        use std::ops::Range;

        let spacing = sanitize_spacing(spacing);
        let scale = self.scale;
        let px = size * scale;
        let line_h = line_height.resolve(size).max(1.0);
        let line_h_px = line_h * scale;

        let mut full = String::new();
        let mut span_bytes: Vec<Range<usize>> = Vec::with_capacity(spans.len());
        let mut span_chars: Vec<Range<usize>> = Vec::with_capacity(spans.len());
        let mut char_count = 0usize;
        for span in spans {
            let normalized = span.text.replace("\r\n", "\n").replace('\r', "\n");
            let byte_start = full.len();
            let char_start = char_count;
            full.push_str(&normalized);
            char_count += normalized.chars().count();
            span_bytes.push(byte_start..full.len());
            span_chars.push(char_start..char_count);
        }

        let mut byte_offsets: Vec<usize> = Vec::with_capacity(char_count + 1);
        byte_offsets.push(0);
        for (i, _) in full.char_indices() {
            if i != 0 {
                byte_offsets.push(i);
            }
        }
        byte_offsets.push(full.len());

        let mut line_bases: Vec<usize> = vec![0];
        for (i, b) in full.bytes().enumerate() {
            if b == b'\n' {
                line_bases.push(i + 1);
            }
        }
        let byte_to_char = |byte: usize| -> usize {
            match byte_offsets.binary_search(&byte) {
                Ok(k) => k.min(char_count),
                Err(k) => k.saturating_sub(1).min(char_count),
            }
        };

        let mut buffer = self
            .shape_scratch
            .take()
            .unwrap_or_else(|| Buffer::new(&mut self.font_system, Metrics::new(px, line_h_px)));
        buffer.set_metrics(Metrics::new(px, line_h_px));
        buffer.set_wrap(mode.cosmic());
        buffer.set_size(
            if width > 0.0 {
                Some(width * scale)
            } else {
                None
            },
            None,
        );
        let cosmic_align = match align {
            rich::RichAlign::Left => cosmic_text::Align::Left,
            rich::RichAlign::Center => cosmic_text::Align::Center,
            rich::RichAlign::Right => cosmic_text::Align::Right,
        };

        let mut eff_families: Vec<String> = Vec::with_capacity(spans.len());
        for span in spans {
            eff_families.push(self.effective_family(span.family.as_deref()));
        }
        let mut attrs_list: Vec<(&str, Attrs)> = Vec::new();
        for (i, span) in spans.iter().enumerate() {
            if span_bytes[i].is_empty() {
                continue;
            }
            let weight = self.resolve_weight_for(span.weight.unwrap_or_default(), &eff_families[i]);
            let style = span.style.unwrap_or_default();
            let mut attrs = Attrs::new()
                .weight(Weight(weight.value()))
                .style(style.cosmic())
                .metadata(i);
            if !eff_families[i].is_empty() {
                attrs = attrs.family(Family::Name(&eff_families[i]));
            }

            let delta = if span.size_delta.is_finite() {
                span.size_delta
            } else {
                0.0
            };
            if delta != 0.0 {
                let span_size = (size + delta).max(1.0);
                let px_i = span_size * scale;
                let span_line_h = line_height.resolve(span_size).max(1.0) * scale;
                attrs = attrs.metrics(Metrics::new(px_i, span_line_h));
            }
            attrs_list.push((&full[span_bytes[i].clone()], attrs));
        }
        buffer.set_rich_text(
            attrs_list,
            &Attrs::new(),
            Shaping::Advanced,
            Some(cosmic_align),
        );
        buffer.shape_until_scroll(&mut self.font_system, false);

        let sp_px = spacing * scale;
        let mut glyphs: Vec<rich::RichGlyph> = Vec::new();
        let mut run_carets: Vec<Vec<f32>> = Vec::new();
        let mut lines: Vec<rich::RichLine> = Vec::new();
        let mut width_px: f32 = 0.0;
        let mut first_line_top: Option<f32> = None;

        let mut cursor_char = 0usize;

        for run in buffer.layout_runs() {
            let base_top = *first_line_top.get_or_insert(run.line_top);
            let top = (run.line_top - base_top) / scale;
            let height = run.line_height / scale;
            let baseline = (run.line_y - base_top) / scale;
            let extra = sp_px * run.glyphs.len().saturating_sub(1) as f32;
            width_px = width_px.max(run.line_w + extra);

            let line_base = line_bases.get(run.line_i).copied().unwrap_or(0);
            let mut table: HashMap<usize, f32> = HashMap::new();
            let mut gmax = 0usize;
            let mut has_glyphs = false;
            for (j, g) in run.glyphs.iter().enumerate() {
                let s = line_base + g.start;
                let e = line_base + g.end;
                gmax = gmax.max(e);
                has_glyphs = true;
                let gx = g.x + g.font_size * g.x_offset + j as f32 * sp_px;
                let gw = g.w;
                table
                    .entry(s)
                    .and_modify(|v| *v = (*v).max(gx / scale))
                    .or_insert(gx / scale);
                table
                    .entry(e)
                    .and_modify(|v| *v = (*v).max((gx + gw) / scale))
                    .or_insert((gx + gw) / scale);
            }

            let start_char = cursor_char;
            let mut end_char = if has_glyphs {
                byte_to_char(gmax).max(cursor_char)
            } else {
                cursor_char
            };
            end_char = end_char.min(char_count);

            let line_end_x = (run.line_w + extra) / scale;
            let mut carets = Vec::with_capacity(end_char - start_char + 1);
            for &byte in byte_offsets.iter().take(end_char + 1).skip(start_char) {
                carets.push(table.get(&byte).copied().unwrap_or(f32::MIN));
            }

            let mut prev = 0.0f32;
            let n = carets.len();
            for (k, v) in carets.iter_mut().enumerate() {
                if *v == f32::MIN {
                    *v = if k == n - 1 {
                        line_end_x.max(prev)
                    } else {
                        prev
                    };
                }
                if *v < prev {
                    *v = prev;
                }
                prev = *v;
            }
            if let Some(last) = carets.last_mut() {
                *last = line_end_x.max(prev);
            }
            if carets.is_empty() {
                carets.push(match align {
                    rich::RichAlign::Left => 0.0,
                    rich::RichAlign::Center => (width * 0.5).max(0.0),
                    rich::RichAlign::Right => width.max(0.0),
                });
            }

            let index = lines.len();
            lines.push(rich::RichLine {
                index,
                top,
                height,
                baseline,
                width: line_end_x,
                chars: start_char..end_char,
            });
            run_carets.push(carets);

            for (j, g) in run.glyphs.iter().enumerate() {
                let p = g.physical((0.0, 0.0), 1.0);
                let x_offset = g.font_size * g.x_offset;
                let y_offset = g.font_size * g.y_offset;
                let span = g.metadata.min(spans.len().saturating_sub(1));
                glyphs.push(rich::RichGlyph {
                    x: g.x + x_offset + j as f32 * sp_px,
                    y: (run.line_y - base_top) + (g.y - y_offset),
                    key: p.cache_key,
                    span,
                    color: spans.get(span).and_then(|s| s.color),
                });
            }

            cursor_char = end_char;

            if cursor_char < char_count
                && full.as_bytes().get(byte_offsets[cursor_char]).copied() == Some(b'\n')
            {
                cursor_char += 1;
            }
        }

        for line in &mut lines {
            line.chars.start = line.chars.start.min(char_count);
            line.chars.end = line.chars.end.min(char_count);
        }

        if full.ends_with('\n') && !full.is_empty() {
            let top = lines.last().map(|l| l.top + l.height).unwrap_or(0.0);
            let index = lines.len();
            let x = match align {
                rich::RichAlign::Left => 0.0,
                rich::RichAlign::Center => (width * 0.5).max(0.0),
                rich::RichAlign::Right => width.max(0.0),
            };
            lines.push(rich::RichLine {
                index,
                top,
                height: line_h,
                baseline: top + line_h * 0.5,
                width: 0.0,
                chars: char_count..char_count,
            });
            run_carets.push(vec![x]);
        }

        let total_height = lines.last().map(|l| l.top + l.height).unwrap_or(0.0);

        let mut backgrounds: Vec<rich::RichRect> = Vec::new();
        let mut decorations: Vec<rich::RichRect> = Vec::new();
        let thickness = (size * 0.07).max(1.0);
        for (si, span) in spans.iter().enumerate() {
            let range = &span_chars[si];
            if range.is_empty() {
                continue;
            }
            for (li, line) in lines.iter().enumerate() {
                let s0 = range.start.max(line.chars.start);
                let s1 = range.end.min(line.chars.end);
                if s1 <= s0 {
                    continue;
                }
                let carets = &run_carets[li];
                let x_of = |c: usize| -> f32 {
                    let k = c.saturating_sub(line.chars.start);
                    carets
                        .get(k.min(carets.len().saturating_sub(1)))
                        .copied()
                        .unwrap_or(0.0)
                };
                let x0 = x_of(s0);
                let x1 = x_of(s1).max(x0);
                if x1 <= x0 {
                    continue;
                }
                if let Some(bg) = span.background {
                    backgrounds.push(rich::RichRect {
                        rect: Rect::from_xywh(x0, line.top, x1 - x0, line.height),
                        color: bg,
                    });
                }
                if span.underline {
                    decorations.push(rich::RichRect {
                        rect: Rect::from_xywh(
                            x0,
                            (line.baseline + size * 0.12).round(),
                            x1 - x0,
                            thickness,
                        ),
                        color: span.color.unwrap_or(Color::WHITE),
                    });
                }
                if span.strikethrough {
                    decorations.push(rich::RichRect {
                        rect: Rect::from_xywh(
                            x0,
                            (line.baseline - size * 0.30).round(),
                            x1 - x0,
                            thickness,
                        ),
                        color: span.color.unwrap_or(Color::WHITE),
                    });
                }
            }
        }

        let out = Rc::new(rich::RichParagraph {
            glyphs,
            backgrounds,
            decorations,
            lines,
            carets: run_carets,
            span_ranges: span_chars,
            span_links: spans.iter().map(|s| s.link.clone()).collect(),
            span_mentions: spans.iter().map(|s| s.mention).collect(),
            span_codes: spans.iter().map(|s| s.code).collect(),
            squiggles: Vec::new(),
            squiggle_rects: Vec::new(),
            width: width_px / scale,
            height: total_height,
            line_height: line_h,
            font_size: size,
            len_chars: char_count,
        });
        self.shape_scratch = Some(buffer);
        out
    }

    pub fn draw_rich(
        &mut self,
        rich: &rich::RichParagraph,
        origin: (f32, f32),
        default_color: Color,
        list: &mut RenderList,
    ) {
        use sequoia_core::GlyphQuad;

        for bg in rich.backgrounds() {
            let r = bg.rect;
            list.push_quad(
                Rect::from_xywh(
                    r.left() + origin.0,
                    r.top() + origin.1,
                    r.size.width,
                    r.size.height,
                ),
                bg.color,
                0.0,
            );
        }
        if rich.glyph_count() > 0 {
            let scale = self.scale;
            let (ox, oy) = (origin.0 * scale, origin.1 * scale);

            let mut placed: Vec<(CacheKey, i32, i32, Color)> = Vec::new();
            for g in rich.glyphs.iter() {
                let color = g.color.unwrap_or(default_color);
                let x = ox + g.x;
                let y = if self.subpixel_y {
                    oy + g.y
                } else {
                    (oy + g.y).trunc()
                };
                let (key, x_int, y_int) = CacheKey::new(
                    g.key.font_id,
                    g.key.glyph_id,
                    f32::from_bits(g.key.font_size_bits),
                    (x, y),
                    g.key.font_weight,
                    g.key.flags,
                );
                if self.atlas.entry(&key).is_none() {
                    self.rasterize(key);
                }
                placed.push((key, x_int, y_int, color));
            }

            let (aw, ah) = (self.atlas.width as f32, self.atlas.height as f32);
            let mut run: Vec<GlyphQuad> = Vec::new();
            let mut run_color = placed[0].3;
            let push_run = |run: &mut Vec<GlyphQuad>, color: Color, list: &mut RenderList| {
                if run.is_empty() {
                    return;
                }
                list.push_text(TextDraw {
                    color,
                    glyphs: std::mem::take(run),
                });
            };
            for (key, x_int, y_int, color) in placed.iter() {
                if *color != run_color && !run.is_empty() {
                    push_run(&mut run, run_color, &mut *list);
                    run_color = *color;
                }
                let Some(entry) = self.atlas.entry(key) else {
                    continue;
                };
                let [rx, ry, rw, rh] = entry.rect;
                if rw == 0 || rh == 0 {
                    continue;
                }
                let x = (*x_int + entry.left) as f32 / scale;
                let y = (*y_int - entry.top) as f32 / scale;
                run.push(GlyphQuad {
                    rect: Rect::from_xywh(x, y, rw as f32 / scale, rh as f32 / scale),
                    uv: [
                        rx as f32 / aw,
                        ry as f32 / ah,
                        (rx + rw) as f32 / aw,
                        (ry + rh) as f32 / ah,
                    ],
                    color_glyph: entry.color,
                });
            }
            push_run(&mut run, run_color, &mut *list);
        }

        for deco in rich.decorations() {
            let r = deco.rect;
            list.push_quad(
                Rect::from_xywh(
                    r.left() + origin.0,
                    r.top() + origin.1,
                    r.size.width,
                    r.size.height,
                ),
                deco.color,
                0.0,
            );
        }
        for sq in rich.squiggle_rects() {
            let r = sq.rect;
            list.push_quad(
                Rect::from_xywh(
                    r.left() + origin.0,
                    r.top() + origin.1,
                    r.size.width,
                    r.size.height,
                ),
                sq.color,
                0.0,
            );
        }
    }

    fn rasterize(&mut self, key: CacheKey) -> AtlasEntry {
        let Some(img) = self.swash.get_image_uncached(&mut self.font_system, key) else {
            return self.atlas.insert(key, 1, 1, &[0, 0, 0, 0], 0, 0, false);
        };
        let left = img.placement.left;
        let top = img.placement.top;
        let w = img.placement.width;
        let h = img.placement.height;
        if w == 0 || h == 0 {
            return self
                .atlas
                .insert(key, 1, 1, &[0, 0, 0, 0], left, top, false);
        }

        let n = (w * h) as usize;
        let mut rgba = vec![0u8; n * ATLAS_BPP];
        let color;
        match img.data.len() {
            len if len == n => {
                for (i, px) in rgba.chunks_exact_mut(ATLAS_BPP).enumerate() {
                    px[0] = 255;
                    px[1] = 255;
                    px[2] = 255;
                    px[3] = img.data[i];
                }
                color = false;
            }
            len if len == n * 4 => {
                for (i, px) in rgba.chunks_exact_mut(ATLAS_BPP).enumerate() {
                    let a = img.data[i * 4 + 3];
                    if a == 0 {
                        continue;
                    }
                    let scale = a as f32 / 255.0;
                    let lin = Color::rgb(
                        img.data[i * 4] as f32 / 255.0,
                        img.data[i * 4 + 1] as f32 / 255.0,
                        img.data[i * 4 + 2] as f32 / 255.0,
                    )
                    .to_linear();
                    px[0] = (lin.r * scale * 255.0).round().clamp(0.0, 255.0) as u8;
                    px[1] = (lin.g * scale * 255.0).round().clamp(0.0, 255.0) as u8;
                    px[2] = (lin.b * scale * 255.0).round().clamp(0.0, 255.0) as u8;
                    px[3] = a;
                }
                color = true;
            }
            _ => {
                return self
                    .atlas
                    .insert(key, 1, 1, &[0, 0, 0, 0], left, top, false);
            }
        }
        self.atlas.insert(key, w, h, &rgba, left, top, color)
    }

    pub fn atlas(&self) -> &GlyphAtlas {
        &self.atlas
    }

    pub fn atlas_mut(&mut self) -> &mut GlyphAtlas {
        &mut self.atlas
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequoia_core::RenderItem;

    fn solid(w: u32, h: u32) -> Vec<u8> {
        vec![255; (w * h) as usize]
    }

    #[test]
    fn sprite_insert_and_entry() {
        let mut atlas = GlyphAtlas::new(64, 64);
        assert!(atlas.sprite(7).is_none());
        let entry = atlas.insert_sprite(7, 10, 8, &solid(10, 8));
        assert_eq!(entry.rect, [0, 0, 10, 8]);
        assert_eq!(atlas.sprite(7), Some(entry));
        assert_eq!(atlas.version(), 1);

        assert_eq!(atlas.pixels()[(7 * 64 + 9) * 4 + 3], 255, "alfa");
        assert_eq!(atlas.pixels()[(7 * 64 + 9) * 4], 255, "biel (RGB)");
        assert_eq!(atlas.pixels()[(7 * 64 + 10) * 4 + 3], 0, "gutter");
    }

    #[test]
    fn atlas_construction_is_clamped_and_inserts_validate_data() {
        let atlas = GlyphAtlas::new(u32::MAX, u32::MAX);
        assert_ne!(atlas.id(), GlyphAtlas::new(8, 8).id());
        assert_eq!(atlas.size(), (ATLAS_MAX_DIM, ATLAS_MAX_DIM));
        assert_eq!(
            atlas.pixels().len(),
            ATLAS_MAX_DIM as usize * ATLAS_MAX_DIM as usize * ATLAS_BPP
        );

        let mut atlas = GlyphAtlas::new(64, 64);
        let v0 = atlas.version();
        let short = vec![255u8; 10];
        let entry = atlas.insert_sprite(99, 10, 8, &short);
        assert_eq!(entry.rect, [0, 0, 1, 1], "sentinel as on full atlas");
        assert!(atlas.sprite(99).is_none(), "do not store the corrupt entry");
        assert_eq!(atlas.version(), v0, "rejection does not bump version");
    }

    #[test]
    fn draw_sprite_pushes_glyph_quad_with_uv() {
        let mut text = TextSystem::with_atlas(64, 64);
        let mut list = RenderList::new();
        let dst = Rect::from_xywh(4.0, 5.0, 10.0, 10.0);
        text.draw_sprite(42, 10, 10, &solid(10, 10), dst, Color::RED, &mut list);
        assert_eq!(list.len(), 1);
        let item = &list.items()[0];
        let RenderItem::Text(t) = item else {
            panic!("expected text")
        };
        assert_eq!(t.color, Color::RED);
        assert_eq!(t.glyphs.len(), 1);
        assert_eq!(t.glyphs[0].rect, dst);
        assert!(t.glyphs[0].uv[2] > t.glyphs[0].uv[0], "UV increasing");
        assert!(t.glyphs[0].uv[3] > t.glyphs[0].uv[1]);

        let v0 = text.atlas().version();
        text.draw_sprite(42, 10, 10, &solid(10, 10), dst, Color::RED, &mut list);
        assert_eq!(text.atlas().version(), v0);
    }

    #[test]
    fn wrap_limits_width_and_grows_height() {
        let mut text = TextSystem::new();
        let content = "aaa bbb ccc ddd eee fff ggg hhh iii jjj kkk lll";
        let single = text.shape(content, 16.0);
        let wrapped = text.shape_wrapped(content, 16.0, FontWeight::NORMAL, 80.0, WrapMode::Word);
        assert!(single.width() > 80.0, "single line wider than the limit");
        assert!(
            wrapped.width() <= 80.0 + 0.5,
            "wrapped within limit: {}",
            wrapped.width()
        );
        assert!(
            wrapped.height() > single.height(),
            "block taller: {} > {}",
            wrapped.height(),
            single.height()
        );
        assert!(wrapped.glyph_count() > 0);
    }

    #[test]
    fn wrap_width_is_part_of_cache_key() {
        let mut text = TextSystem::new();
        let a = text.shape_wrapped(
            "aaa bbb ccc",
            16.0,
            FontWeight::NORMAL,
            40.0,
            WrapMode::Word,
        );
        let b = text.shape_wrapped(
            "aaa bbb ccc",
            16.0,
            FontWeight::NORMAL,
            400.0,
            WrapMode::Word,
        );
        assert!(!Rc::ptr_eq(&a, &b), "different widths = different entries");
        let c = text.shape_wrapped(
            "aaa bbb ccc",
            16.0,
            FontWeight::NORMAL,
            40.0,
            WrapMode::Word,
        );
        assert!(Rc::ptr_eq(&a, &c), "same width = hit");
    }

    #[test]
    fn wrap_mode_character_breaks_long_words() {
        let mut text = TextSystem::new();
        let word = "superlongwordwithoutspacestocheck";
        let word_mode = text.shape_wrapped(word, 16.0, FontWeight::NORMAL, 60.0, WrapMode::Word);
        let char_mode =
            text.shape_wrapped(word, 16.0, FontWeight::NORMAL, 60.0, WrapMode::Character);
        assert!(
            char_mode.width() <= 60.0 + 0.5,
            "character within limit: {}",
            char_mode.width()
        );
        assert!(
            char_mode.height() > word_mode.height(),
            "character breaks into more lines: {} > {}",
            char_mode.height(),
            word_mode.height()
        );
    }

    #[test]
    fn line_height_sets_row_geometry() {
        let mut text = TextSystem::new();
        let shaped = text.shape_wrapped_styled_spaced_lh(
            "aaa\nbbb",
            16.0,
            FontWeight::NORMAL,
            0.0,
            WrapMode::Word,
            0.0,
            None,
            FontStyle::Normal,
            LineHeight::Absolute(40.0),
        );
        assert_eq!(shaped.line_count(), 2);
        assert!((shaped.rows()[1].top - 40.0).abs() < 0.5);
        assert!(shaped.height() >= 80.0);
        assert!(shaped.first_baseline() > 0.0);
        assert!(shaped.first_baseline() < 40.0);
    }

    #[test]
    fn line_height_is_part_of_cache_key() {
        let mut text = TextSystem::new();
        let a = text.shape_wrapped_styled_spaced_lh(
            "aaa bbb",
            16.0,
            FontWeight::NORMAL,
            80.0,
            WrapMode::Word,
            0.0,
            None,
            FontStyle::Normal,
            LineHeight::Factor(1.0),
        );
        let b = text.shape_wrapped_styled_spaced_lh(
            "aaa bbb",
            16.0,
            FontWeight::NORMAL,
            80.0,
            WrapMode::Word,
            0.0,
            None,
            FontStyle::Normal,
            LineHeight::Factor(2.0),
        );
        assert!(!Rc::ptr_eq(&a, &b));
        assert!(b.height() > a.height());
    }

    #[test]
    fn multiline_elision_stops_at_max_lines() {
        let mut text = TextSystem::new();
        let content = "one two three four five six seven eight nine ten";
        let shown = text.elide_lines_styled_spaced(
            content,
            16.0,
            FontWeight::NORMAL,
            70.0,
            WrapMode::Word,
            0.0,
            None,
            FontStyle::Normal,
            LineHeight::default(),
            2,
        );
        assert!(shown.ends_with('…'), "ellipsis appended: {shown:?}");
        assert!(shown.chars().count() < content.chars().count());
        let shaped = text.shape_wrapped_styled_spaced_lh(
            &shown,
            16.0,
            FontWeight::NORMAL,
            70.0,
            WrapMode::Word,
            0.0,
            None,
            FontStyle::Normal,
            LineHeight::default(),
        );
        assert!(shaped.line_count() <= 2, "lines: {}", shaped.line_count());
    }

    #[test]
    fn min_content_width_uses_longest_word_or_char() {
        let mut text = TextSystem::new();
        let content = "a enormouslylongword b";
        let word = text.text_width_styled_spaced(
            "enormouslylongword",
            16.0,
            FontWeight::NORMAL,
            0.0,
            None,
            FontStyle::Normal,
        );
        let min_word = text.min_content_width_styled_spaced(
            content,
            16.0,
            FontWeight::NORMAL,
            0.0,
            None,
            FontStyle::Normal,
            WrapMode::Word,
        );
        assert!((min_word - word).abs() < 0.5, "{min_word} vs {word}");
        let min_char = text.min_content_width_styled_spaced(
            content,
            16.0,
            FontWeight::NORMAL,
            0.0,
            None,
            FontStyle::Normal,
            WrapMode::Character,
        );
        assert!(min_char < min_word);
    }

    #[test]
    fn rich_elision_stops_at_max_lines() {
        let mut text = TextSystem::new();
        let spans = [rich::TextSpan::new(
            "one two three four five six seven eight nine ten",
        )];
        let shown = text.elide_rich_lines(
            &spans,
            16.0,
            70.0,
            WrapMode::Word,
            0.0,
            rich::RichAlign::Left,
            LineHeight::default(),
            2,
        );
        let joined: String = shown.iter().map(|s| s.text.as_str()).collect();
        assert!(joined.ends_with('…'), "ellipsis appended: {joined:?}");
        let shaped = text.shape_rich_full_lh(
            &shown,
            16.0,
            70.0,
            WrapMode::Word,
            0.0,
            rich::RichAlign::Left,
            LineHeight::default(),
        );
        assert!(shaped.line_count() <= 2);
    }

    #[test]
    fn truncate_cuts_and_fits() {
        let mut text = TextSystem::new();
        assert_eq!(
            text.truncate("abcdef", 16.0, FontWeight::NORMAL, 10_000.0),
            "abcdef"
        );
        assert_eq!(text.truncate("abcdef", 16.0, FontWeight::NORMAL, 0.0), "");
        assert_eq!(text.truncate("", 16.0, FontWeight::NORMAL, 50.0), "");
        let cut = text.truncate("abcdef", 16.0, FontWeight::NORMAL, 30.0);
        assert!(cut.len() < 6, "cut: {cut}");
        assert!("abcdef".starts_with(&cut), "prefix by chars: {cut}");
        assert!(text.text_width(&cut, 16.0) <= 30.0 + 0.5, "fits: {cut}");
    }

    #[test]
    fn elide_right_middle_left() {
        let mut text = TextSystem::new();
        let content = "Ann has a cat and a panda";

        assert_eq!(
            text.elide(
                content,
                16.0,
                FontWeight::NORMAL,
                10_000.0,
                ElideMode::Right
            ),
            content
        );
        assert_eq!(
            text.elide(content, 16.0, FontWeight::NORMAL, 0.0, ElideMode::Right),
            ""
        );

        let r = text.elide(content, 16.0, FontWeight::NORMAL, 60.0, ElideMode::Right);
        assert!(r.ends_with('…') && !r.starts_with('…'), "right: {r}");
        let l = text.elide(content, 16.0, FontWeight::NORMAL, 60.0, ElideMode::Left);
        assert!(l.starts_with('…') && !l.ends_with('…'), "left: {l}");
        let m = text.elide(content, 16.0, FontWeight::NORMAL, 60.0, ElideMode::Middle);
        assert!(
            m.starts_with('A') && m.ends_with('a') && m.contains('…'),
            "middle keeps the ends: {m}"
        );
        for (name, s) in [("R", &r), ("L", &l), ("M", &m)] {
            assert!(
                text.text_width(s, 16.0) <= 60.0 + 0.5,
                "{name} within limit: {s}"
            );
        }
    }

    #[test]
    fn explicit_newlines_span_lines_without_wrap() {
        let mut text = TextSystem::new();
        let one = text.shape("aaa", 16.0);
        let two = text.shape("aaa\nbbb", 16.0);
        assert!(
            two.height() > one.height(),
            "explicit line break grows height: {} > {}",
            two.height(),
            one.height()
        );
    }

    #[test]
    fn multiline_glyphs_carry_line_offsets() {
        let mut text = TextSystem::new();
        let two = text.shape("aaa\nbbb", 16.0);
        let ys: Vec<f32> = two.glyphs.iter().map(|g| g.y).collect();
        assert!(!ys.is_empty());
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        for y in ys {
            lo = lo.min(y);
            hi = hi.max(y);
        }
        assert!(hi - lo > 10.0, "lines at different heights, span {hi}-{lo}");

        let one = text.shape("aaa", 16.0);
        let one_y: Vec<f32> = one.glyphs.iter().map(|g| g.y).collect();
        let two_first: Vec<f32> = two.glyphs.iter().take(one_y.len()).map(|g| g.y).collect();
        assert_eq!(one_y, two_first, "first line without offset");
    }

    #[test]
    fn draw_bins_glyphs_by_subpixel_position() {
        let mut text = TextSystem::with_atlas(256, 256);
        if text.shape("l", 16.0).glyph_count() == 0 {
            eprintln!("skip: no fonts in environment");
            return;
        }
        let shaped = text.shape("l", 16.0);
        let mut list = RenderList::new();
        text.draw(&shaped, (0.0, 0.0), Color::WHITE, &mut list);
        let v0 = text.atlas().version();
        text.draw(&shaped, (0.5, 0.0), Color::WHITE, &mut list);
        assert!(
            text.atlas().version() > v0,
            "inna faza subpikselowa = osobny raster w atlasie"
        );

        let v1 = text.atlas().version();
        text.draw(&shaped, (0.5, 0.0), Color::WHITE, &mut list);
        assert_eq!(text.atlas().version(), v1, "same origin is a hit");
    }

    #[test]
    fn subpixel_y_adds_a_new_raster_phase() {
        let mut text = TextSystem::with_atlas(256, 256);
        if text.shape("l", 16.0).glyph_count() == 0 {
            eprintln!("skip: no fonts in environment");
            return;
        }
        let shaped = text.shape("l", 16.0);
        let mut list = RenderList::new();
        text.draw(&shaped, (0.0, 10.37), Color::WHITE, &mut list);
        let v0 = text.atlas().version();
        assert!(!text.subpixel_y(), "Y hinted by default");
        text.set_subpixel_y(true);
        let mut list = RenderList::new();
        text.draw(&shaped, (0.0, 10.37), Color::WHITE, &mut list);
        assert!(
            text.atlas().version() > v0,
            "subpikselowy Y = faza rastra spoza binu Zero"
        );
    }

    #[test]
    fn draw_keeps_fractional_origin_shifts() {
        let mut text = TextSystem::with_atlas(256, 256);
        if text.shape("HH", 16.0).glyph_count() == 0 {
            eprintln!("skip: no fonts in environment");
            return;
        }
        let shaped = text.shape("HH", 16.0);
        let rects = |t: &mut TextSystem, x: f32| -> Vec<Rect> {
            let mut list = RenderList::new();
            t.draw(&shaped, (x, 50.0), Color::WHITE, &mut list);
            let Some(RenderItem::Text(d)) = list.items().first() else {
                panic!("missing text")
            };
            d.glyphs.iter().map(|g| g.rect).collect()
        };
        let a = rects(&mut text, 10.0);
        let b = rects(&mut text, 11.0);
        assert_eq!(a.len(), b.len());
        for (i, (p, q)) in a.iter().zip(&b).enumerate() {
            assert!(
                ((q.left() - p.left()) - 1.0).abs() < 0.5,
                "glyph {i}: 1px shift gave {}",
                q.left() - p.left()
            );
        }
    }

    #[test]
    fn draw_sprite_snaps_to_physical_pixels() {
        let mut text = TextSystem::with_atlas(64, 64);
        text.set_scale_factor(2.0);
        let mut list = RenderList::new();
        let dst = Rect::from_xywh(4.5, 5.25, 10.0, 10.0);
        text.draw_sprite(7, 20, 20, &solid(20, 20), dst, Color::WHITE, &mut list);
        assert_eq!(list.len(), 1);
        let item = &list.items()[0];
        let RenderItem::Text(t) = item else {
            panic!("expected text")
        };
        assert_eq!(t.glyphs.len(), 1);
        let r = t.glyphs[0].rect;
        assert_eq!((r.left(), r.top()), (4.5, 5.5), "snap: {r:?}");
        assert_eq!(
            (r.size.width, r.size.height),
            (10.0, 10.0),
            "1:1 with bitmap: {r:?}"
        );

        assert_eq!((r.left() * 2.0, r.top() * 2.0), (9.0, 11.0));
        assert_eq!((r.size.width * 2.0, r.size.height * 2.0), (20.0, 20.0));
    }

    #[test]
    fn load_font_garbage_is_none_without_panic() {
        let mut text = TextSystem::new();
        assert_eq!(text.load_font(vec![0xDE, 0xAD, 0xBE, 0xEF]), None);

        assert_eq!(text.default_family(), Some("Inter"));
    }

    #[test]
    fn set_default_family_takes_effect() {
        let mut text = TextSystem::new();
        text.set_default_family(Some("SequoiaTest".into()));
        assert_eq!(text.default_family(), Some("SequoiaTest"));

        let shaped = text.shape("test", 12.0);
        assert!(shaped.glyph_count() > 0);
    }

    #[test]
    fn shaping_cache_is_bounded_lru() {
        let mut text = TextSystem::new();
        for i in 0..(SHAPING_CACHE_MAX + 50) {
            text.shape(&format!("unique-line-{i}"), 12.0);
        }
        assert_eq!(
            text.shaped.len(),
            SHAPING_CACHE_MAX,
            "cache does not grow unbounded"
        );
        assert_eq!(text.shaping_order.len(), SHAPING_CACHE_MAX);

        let old_key: ShapeKey = (
            "unique-line-0".to_owned(),
            (12.0f32 * 4.0).round() as u32,
            400,
            "Inter".to_owned(),
            FontStyle::Normal.key(),
            0,
            WrapMode::Word.key(),
            0,
            LineHeight::default().key(),
        );
        assert!(
            !text.shaped.contains_key(&old_key),
            "LRU: najstarszy wyparowany"
        );

        let a = text.shape("unique-line-1", 12.0);
        let b = text.shape("unique-line-1", 12.0);
        assert!(Rc::ptr_eq(&a, &b), "hit = same Rc without reshaping");
        assert_eq!(
            text.shaped.len(),
            SHAPING_CACHE_MAX,
            "hit does not grow the cache"
        );
    }

    #[test]
    fn mru_returns_same_shape_without_rehash() {
        let mut text = TextSystem::new();
        let a = text.shape("repeat me", 14.0);

        let b = text.shape("repeat me", 14.0);
        assert!(Rc::ptr_eq(&a, &b), "MRU hit = same Rc");
        assert!(text.mru.is_some(), "MRU remembered");
        assert_eq!(text.shaped.len(), 1, "MRU does not duplicate cache entries");

        let c = text.shape("something else", 14.0);
        assert!(!Rc::ptr_eq(&a, &c));
        let d = text.shape("something else", 14.0);
        assert!(Rc::ptr_eq(&c, &d), "MRU after switching");
        assert_eq!(text.shaped.len(), 2);

        let e = text.shape("repeat me", 14.0);
        assert!(Rc::ptr_eq(&a, &e));
    }

    #[test]
    fn mru_cleared_with_caches() {
        let mut text = TextSystem::new();
        let a = text.shape("trzymaj", 12.0);
        assert!(text.mru.is_some());
        text.set_scale_factor(2.0);
        assert!(text.mru.is_none(), "the scale also cleans the MRU");
        let b = text.shape("trzymaj", 12.0);
        assert!(!Rc::ptr_eq(&a, &b), "po zmianie skali reshape");
    }

    #[test]
    fn atlas_repack_keeps_hot_entries_instead_of_clearing() {
        let mut atlas = GlyphAtlas::new(64, 64);
        for i in 0..10u64 {
            atlas.insert_sprite(i, 16, 16, &[255; 256]);
        }
        assert!(atlas.sprite(9).is_some(), "najnowszy wpis obecny");
        let present = (0..10u64).filter(|i| atlas.sprite(*i).is_some()).count();
        assert!(present >= 2, "hot set survived compaction, got {present}");
        assert!(
            atlas.pixels().iter().any(|&p| p > 0),
            "repacked pixels are not zeroed"
        );
        let (w, h) = atlas.size();
        assert_eq!(atlas.pixels().len(), (w * h) as usize * ATLAS_BPP);
    }

    #[test]
    fn atlas_dims_stay_bounded_under_load() {
        let mut atlas = GlyphAtlas::new(64, 64);
        for i in 0..200u64 {
            atlas.insert_sprite(i, 16, 16, &solid(16, 16));
        }
        let (w, h) = atlas.size();
        assert!(
            w <= ATLAS_MAX_DIM && h <= ATLAS_MAX_DIM,
            "atlas {w}×{h} past the limit"
        );
        assert!(atlas.pixels().len() == (w * h) as usize * ATLAS_BPP);
    }

    #[test]
    fn letter_spacing_widens_text() {
        let mut text = TextSystem::new();
        let base = text.text_width_weighted("abc", 16.0, FontWeight::NORMAL);
        let wide = text.text_width_weighted_spaced("abc", 16.0, FontWeight::NORMAL, 2.0);
        assert!(
            (wide - base - 4.0).abs() < 0.5,
            "3 znaki + spacing 2 → +4 px, base={base} wide={wide}"
        );

        let one = text.text_width_weighted("a", 16.0, FontWeight::NORMAL);
        let one_sp = text.text_width_weighted_spaced("a", 16.0, FontWeight::NORMAL, 5.0);
        assert!((one - one_sp).abs() < 1e-4, "1 znak: {one} vs {one_sp}");

        assert_eq!(
            text.text_width_weighted_spaced("", 16.0, FontWeight::NORMAL, 5.0),
            0.0
        );
    }

    #[test]
    fn letter_spacing_zero_matches_plain() {
        let mut text = TextSystem::new();
        let a = text.shape_weighted("Sequoia", 14.0, FontWeight::BOLD);
        let b = text.shape_spaced("Sequoia", 14.0, FontWeight::BOLD, 0.0);
        assert!((a.width() - b.width()).abs() < 1e-4);
        assert_eq!(a.glyph_count(), b.glyph_count());
    }

    #[test]
    fn letter_spacing_cache_key_is_distinct() {
        let mut text = TextSystem::new();
        let a = text.shape_spaced("ab", 12.0, FontWeight::NORMAL, 0.0);
        let b = text.shape_spaced("ab", 12.0, FontWeight::NORMAL, 3.0);
        assert!(!Rc::ptr_eq(&a, &b), "inny spacing = osobny wpis cache");
        let c = text.shape_spaced("ab", 12.0, FontWeight::NORMAL, 3.0);
        assert!(Rc::ptr_eq(&b, &c), "same spacing = hit");
    }

    #[test]
    fn letter_spacing_shifts_glyphs() {
        let mut text = TextSystem::new();
        let base = text.shape_spaced("ab", 20.0, FontWeight::NORMAL, 0.0);
        let wide = text.shape_spaced("ab", 20.0, FontWeight::NORMAL, 4.0);
        assert_eq!(base.glyph_count(), wide.glyph_count());
        if base.glyph_count() >= 2 {
            let dx0 = wide.glyphs[0].x - base.glyphs[0].x;
            let dx1 = wide.glyphs[1].x - base.glyphs[1].x;
            assert!(dx0.abs() < 0.5, "first glyph stays, dx={dx0}");
            assert!(
                (dx1 - 4.0 * text.scale_factor()).abs() < 1.0,
                "second shifted by spacing, dx={dx1}"
            );
        }
    }

    #[test]
    fn letter_spacing_negative_tightens() {
        let mut text = TextSystem::new();
        let base = text.text_width_weighted("abc", 16.0, FontWeight::NORMAL);
        let tight = text.text_width_weighted_spaced("abc", 16.0, FontWeight::NORMAL, -1.0);
        assert!(tight < base, "negative spacing tightens: {tight} < {base}");
    }

    #[test]
    fn letter_spacing_nonfinite_is_zero() {
        let mut text = TextSystem::new();
        let base = text.text_width_weighted("abc", 16.0, FontWeight::NORMAL);
        assert_eq!(
            text.text_width_weighted_spaced("abc", 16.0, FontWeight::NORMAL, f32::NAN),
            base
        );
        assert_eq!(
            text.text_width_weighted_spaced("abc", 16.0, FontWeight::NORMAL, f32::INFINITY),
            base
        );
    }

    #[test]
    fn letter_spacing_carets_match_width() {
        let mut text = TextSystem::new();
        let pos = text.caret_positions_spaced("abcd", 16.0, FontWeight::NORMAL, 2.0);
        assert_eq!(pos.len(), 5);
        let w = text.text_width_weighted_spaced("abcd", 16.0, FontWeight::NORMAL, 2.0);
        assert!(
            (pos[4] - w).abs() < 0.5,
            "last position = width: {} vs {w}",
            pos[4]
        );
        assert!(
            pos.windows(2).all(|w| w[1] >= w[0] - 1e-4),
            "monotoniczne: {pos:?}"
        );

        assert!(
            pos[1] - pos[0] >= 1.5,
            "step with tracking: {:?}",
            &pos[..2]
        );
    }

    #[test]
    fn oversize_sprite_is_skipped_not_stored() {
        let mut atlas = GlyphAtlas::new(64, 64);
        let w = ATLAS_MAX_DIM + 1;
        let entry = atlas.insert_sprite(1, w, 8, &vec![255u8; (w * 8) as usize]);
        assert!(atlas.sprite(1).is_none(), "oversize entry is not stored");
        assert_eq!(entry.rect[2], 1, "sentinel instead of garbage UV");
        let (aw, ah) = atlas.size();
        assert!(
            aw <= ATLAS_MAX_DIM && ah <= ATLAS_MAX_DIM,
            "atlas did not grow past the limit"
        );
    }

    #[test]
    fn line_cache_edit_single_line_reshapes_at_most_3() {
        use sequoia_buffer::StyledBuffer;
        let mut sys = TextSystem::new();

        let initial: String = (0..10_000)
            .map(|i| format!("line {i:05} some text for shaping"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut buf = StyledBuffer::from(initial);
        let line_count = buf.lines().line_count();
        assert_eq!(line_count, 10_000);

        sys.reset_line_reshapes();
        let all = sys.shape_lines_visible_buffer(
            &buf,
            0..line_count,
            14.0,
            FontWeight::NORMAL,
            0.0,
            WrapMode::Word,
            0.0,
            None,
            FontStyle::Normal,
        );
        assert_eq!(all.len(), 10_000);
        assert_eq!(sys.line_reshapes(), 10_000);
        assert!(sys.line_cache_len() >= 10_000);

        sys.reset_line_reshapes();
        let again = sys.shape_lines_visible_buffer(
            &buf,
            0..line_count,
            14.0,
            FontWeight::NORMAL,
            0.0,
            WrapMode::Word,
            0.0,
            None,
            FontStyle::Normal,
        );
        assert_eq!(sys.line_reshapes(), 0, "bez edycji wszystko trafia w cache");
        assert!(Rc::ptr_eq(&all[5000], &again[5000]));

        let (s, e) = buf.lines().line_range(5000);
        let old_text = buf.substring(s..e);
        assert!(old_text.starts_with("line 05000"));
        buf.replace(s..e, "line 05000 EDITED single line change", None);

        let (ns, ne) = buf.lines().line_range(5000);
        let spans = buf.spans_for_line(ns..ne);
        let line_text = buf.substring(ns..ne);
        let h1 = TextSystem::line_hash(&line_text, &spans, ns, 0.0);
        let h0 = TextSystem::line_hash(&old_text, &[], s, 0.0);
        assert_ne!(h0, h1, "hash changes after editing content");

        sys.invalidate_for_edit(buf.lines(), s, e);

        sys.reset_line_reshapes();
        let after = sys.shape_lines_visible_buffer(
            &buf,
            0..line_count,
            14.0,
            FontWeight::NORMAL,
            0.0,
            WrapMode::Word,
            0.0,
            None,
            FontStyle::Normal,
        );
        assert!(
            sys.line_reshapes() <= 3,
            "edit 1 line = ≤3 reshapes, it was {}",
            sys.line_reshapes()
        );
        assert!(
            !Rc::ptr_eq(&all[5000], &after[5000]),
            "edytowana linia reshape"
        );
        assert!(Rc::ptr_eq(&all[0], &after[0]), "nieedytowane linie hit");
        assert!(Rc::ptr_eq(&all[9999], &after[9999]));
    }

    #[test]
    fn line_cache_apply_edit_invalidate_uses_line_of_only() {
        use sequoia_buffer::{LineIndex, StyledBuffer};
        let mut sys = TextSystem::new();
        let mut buf = StyledBuffer::from("aaa\nbbb\nccc");
        let n = buf.lines().line_count();
        assert_eq!(n, 3);
        let _ = sys.shape_lines_visible_buffer(
            &buf,
            0..n,
            14.0,
            FontWeight::NORMAL,
            0.0,
            WrapMode::Word,
            0.0,
            None,
            FontStyle::Normal,
        );
        sys.reset_line_reshapes();

        let idx: &LineIndex = buf.lines();
        assert_eq!(idx.line_of(4), 1);
        assert_eq!(idx.line_of(7), 1);
        sys.invalidate_for_edit(idx, 4, 7);

        sys.invalidate_for_edit(idx, 4, 7);
        let before = sys.line_reshapes();
        let (s0, e0) = idx.line_range(0);
        let t0 = buf.substring(s0..e0);
        let _ = sys.shape_line_wrapped(0, &t0, 14.0, FontWeight::NORMAL, 0.0, WrapMode::Word);
        assert_eq!(sys.line_reshapes(), before, "line 0 untouched = hit");

        buf.replace(4..7, "BBB", None);
        sys.invalidate_for_edit(buf.lines(), 4, 7);
        let (s1, e1) = buf.lines().line_range(1);
        let t1 = buf.substring(s1..e1);
        assert_eq!(t1, "BBB");
        let _ = sys.shape_line_wrapped(1, &t1, 14.0, FontWeight::NORMAL, 0.0, WrapMode::Word);
        assert_eq!(sys.line_reshapes(), before + 1, "linia 1 miss = 1 reshape");
    }

    #[test]
    fn line_cache_split_join_newline_reshapes_only_touched() {
        use sequoia_buffer::StyledBuffer;
        let mut sys = TextSystem::new();
        let mut buf = StyledBuffer::from("aaa\nbbb\nccc");
        let shape_all = |sys: &mut TextSystem, buf: &StyledBuffer| -> Vec<Rc<ShapedLine>> {
            let n = buf.lines().line_count();
            sys.shape_lines_visible_buffer(
                buf,
                0..n,
                14.0,
                FontWeight::NORMAL,
                0.0,
                WrapMode::Word,
                0.0,
                None,
                FontStyle::Normal,
            )
        };
        let before_lines = shape_all(&mut sys, &buf);
        assert_eq!(before_lines.len(), 3);

        sys.reset_line_reshapes();
        buf.insert(5, "\n", None);
        assert_eq!(buf.lines().line_count(), 4, "split adds a line");
        assert_eq!(buf.to_string(), "aaa\nb\nbb\nccc");

        let edited = {
            let idx = buf.lines();

            idx.line_of(5)..idx.line_of(5).saturating_add(1)
        };
        sys.invalidate_lines(edited);
        let after_split = shape_all(&mut sys, &buf);
        assert_eq!(after_split.len(), 4);
        assert!(
            sys.line_reshapes() <= 3,
            "split 1 line = ≤3 reshapes, it was {}",
            sys.line_reshapes()
        );

        sys.reset_line_reshapes();

        buf.delete(5..6);
        assert_eq!(buf.to_string(), "aaa\nbbb\nccc");
        assert_eq!(buf.lines().line_count(), 3, "join removes line");
        sys.invalidate_lines(1..2);
        let after_join = shape_all(&mut sys, &buf);
        assert_eq!(after_join.len(), 3);
        assert!(
            sys.line_reshapes() <= 3,
            "join 2 lines = ≤3 reshapes, it was {}",
            sys.line_reshapes()
        );

        let _ = (before_lines, after_split, after_join);
    }

    #[test]
    fn line_cache_evict_clears_line_to_key() {
        let mut sys = TextSystem::new();
        let n = LINE_CACHE_MAX + 66;
        for i in 0..n {
            let t = format!("evict probe unique line {i:06} xyz");
            let _ = sys.shape_line_wrapped(i, &t, 12.0, FontWeight::NORMAL, 0.0, WrapMode::Word);
        }
        assert!(
            sys.line_cache_len() <= LINE_CACHE_MAX,
            "LRU bound: {} <= {LINE_CACHE_MAX}",
            sys.line_cache_len()
        );
        let mut checked = 0usize;
        for ((ln, wq), key) in sys.line_to_key.iter() {
            assert!(
                sys.lines.contains_key(key),
                "hanging corpse line_to_key[({ln},{wq})]"
            );
            checked += 1;
        }
        assert!(checked > 0);

        sys.invalidate_lines(0..50);
        for ((ln, wq), key) in sys.line_to_key.iter() {
            assert!(
                sys.lines.contains_key(key),
                "trup po invalidate [({ln},{wq})]"
            );
        }
    }

    #[test]
    fn line_hash_covers_all_span_fields_not_just_len() {
        use sequoia_buffer::{SpanAttr, StyledSpan};

        let mk = |f: fn(SpanAttr) -> SpanAttr| vec![StyledSpan::new(0, 5, f(SpanAttr::plain()))];
        let sets = [
            mk(|a| SpanAttr { bold: true, ..a }),
            mk(|a| SpanAttr { italic: true, ..a }),
            mk(|a| SpanAttr {
                underline: true,
                ..a
            }),
            mk(|a| SpanAttr { strike: true, ..a }),
            mk(|a| SpanAttr {
                color: Some(Color::RED),
                ..a
            }),
            mk(|a| SpanAttr {
                color: Some(Color::BLUE),
                ..a
            }),
            mk(|a| SpanAttr {
                bg: Some(Color::GREEN),
                ..a
            }),
            mk(|a| SpanAttr {
                size_delta: 4.0,
                ..a
            }),
            mk(|a| SpanAttr {
                link: Some("https://x".into()),
                ..a
            }),
            mk(|a| SpanAttr {
                mention: Some(7),
                ..a
            }),
            mk(|a| SpanAttr { code: true, ..a }),
        ];
        let hashes = sets
            .iter()
            .map(|sp| TextSystem::line_hash("hello", sp, 0, 0.0))
            .collect::<Vec<_>>();
        for i in 0..hashes.len() {
            for j in (i + 1)..hashes.len() {
                assert_ne!(hashes[i], hashes[j], "distinguish fields {i} vs {j}");
            }
        }

        let s0 = vec![StyledSpan::new(0, 5, SpanAttr::bold())];
        let s1 = vec![StyledSpan::new(100, 105, SpanAttr::bold())];
        assert_eq!(
            TextSystem::line_hash("hello", &s0, 0, 0.0),
            TextSystem::line_hash("hello", &s1, 100, 0.0),
            "relatywizacja do line_byte_start"
        );

        let w0 = TextSystem::line_hash("hello", &s0, 0, 0.0);
        let w1 = TextSystem::line_hash("hello", &s0, 0, 40.0);
        assert_ne!(w0, w1, "width wchodzi do line_hash");
    }

    #[test]
    fn line_hash_delegates_to_f1_exactly() {
        use sequoia_buffer::{SpanAttr, StyledSpan, rebase_spans};

        let cases: Vec<(&str, Vec<StyledSpan>, usize, f32)> = vec![
            ("", vec![], 0, 0.0),
            ("hello", vec![], 0, 0.0),
            (
                "hello",
                vec![StyledSpan::new(0, 5, SpanAttr::bold())],
                0,
                100.0,
            ),
            (
                "hello",
                vec![StyledSpan::new(100, 105, SpanAttr::link("x"))],
                100,
                40.0,
            ),
            (
                "a👨‍👩‍👧b 🇵🇱 café",
                vec![StyledSpan::new(1, 1 + "👨‍👩‍👧".len(), SpanAttr::bold())],
                0,
                0.0,
            ),
            ("yellow the goose of the self", vec![], 57, 300.0),
            (
                "q",
                vec![StyledSpan::new(
                    0,
                    1,
                    SpanAttr {
                        size_delta: f32::NAN,
                        ..SpanAttr::plain()
                    },
                )],
                0,
                0.0,
            ),
            (
                "q",
                vec![StyledSpan::new(
                    0,
                    1,
                    SpanAttr {
                        size_delta: f32::INFINITY,
                        ..SpanAttr::plain()
                    },
                )],
                0,
                0.0,
            ),
            (
                "q",
                vec![StyledSpan::new(
                    0,
                    1,
                    SpanAttr {
                        size_delta: 0.12,
                        ..SpanAttr::plain()
                    },
                )],
                0,
                0.0,
            ),
            (
                "q",
                vec![StyledSpan::new(
                    0,
                    1,
                    SpanAttr {
                        size_delta: 0.13,
                        ..SpanAttr::plain()
                    },
                )],
                0,
                0.0,
            ),
        ];
        for (text, spans, base, width) in &cases {
            assert_eq!(
                TextSystem::line_hash(text, spans, *base, *width),
                sequoia_buffer::line_hash(text, &rebase_spans(spans, *base), *width),
                "F2==F1 dla {text:?} base={base} w={width}"
            );
        }

        let q1 = TextSystem::line_hash(
            "q",
            &[StyledSpan::new(
                0,
                1,
                SpanAttr {
                    size_delta: 0.12,
                    ..SpanAttr::plain()
                },
            )],
            0,
            0.0,
        );
        let q2 = TextSystem::line_hash(
            "q",
            &[StyledSpan::new(
                0,
                1,
                SpanAttr {
                    size_delta: 0.13,
                    ..SpanAttr::plain()
                },
            )],
            0,
            0.0,
        );
        assert_ne!(q1, q2, "0.12 (kwant 0) vs 0.13 (kwant 1)");

        for d in [0.0, 0.1, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let h = TextSystem::line_hash(
                "q",
                &[StyledSpan::new(
                    0,
                    1,
                    SpanAttr {
                        size_delta: d,
                        ..SpanAttr::plain()
                    },
                )],
                0,
                0.0,
            );

            let h0 = TextSystem::line_hash(
                "q",
                &[StyledSpan::new(
                    0,
                    1,
                    SpanAttr {
                        size_delta: 0.0,
                        ..SpanAttr::plain()
                    },
                )],
                0,
                0.0,
            );
            assert_eq!(h, h0, "delta {d} w kwancie 0");
        }
    }

    #[test]
    fn line_width_is_correct_at_scale_two() {
        let mut lo = TextSystem::new();
        let mut hi = TextSystem::new();
        hi.set_scale_factor(2.0);
        let text = "Sequoia width check";
        let a = lo.shape_line_wrapped(0, text, 16.0, FontWeight::NORMAL, 0.0, WrapMode::Word);
        let b = hi.shape_line_wrapped(0, text, 16.0, FontWeight::NORMAL, 0.0, WrapMode::Word);
        assert!(
            (a.width() - b.width()).abs() < 1.5,
            "scale-independent logical width: {} vs {}",
            a.width(),
            b.width()
        );
        assert!(a.width() > 0.0 && b.width() > 0.0);

        let plain = lo.shape(text, 16.0);
        assert!(
            (a.width() - plain.width()).abs() < 2.0,
            "ShapedLine.width ≈ ShapedText.width: {} vs {}",
            a.width(),
            plain.width()
        );
    }

    #[test]
    fn line_cache_wrap_width_is_per_call_not_global() {
        let mut sys = TextSystem::new();
        let a = sys.shape_line_wrapped(
            0,
            "aaa bbb ccc ddd",
            16.0,
            FontWeight::NORMAL,
            40.0,
            WrapMode::Word,
        );
        let b = sys.shape_line_wrapped(
            0,
            "aaa bbb ccc ddd",
            16.0,
            FontWeight::NORMAL,
            400.0,
            WrapMode::Word,
        );
        assert!(!Rc::ptr_eq(&a, &b), "different widths = separate entries");
        assert!(
            b.row_count() <= a.row_count(),
            "szerszy wrap = mniej wierszy"
        );
        let c = sys.shape_line_wrapped(
            0,
            "aaa bbb ccc ddd",
            16.0,
            FontWeight::NORMAL,
            40.0,
            WrapMode::Word,
        );
        assert!(Rc::ptr_eq(&a, &c), "same width = hit");

        assert!(a.height() >= b.height());
        assert_eq!(a.width_quant(), (40.0f32 * 4.0).round() as u32);
    }

    #[test]
    fn line_cache_bidi_does_not_panic_and_keeps_glyphs() {
        let mut sys = TextSystem::new();
        for text in [
            "Hello שלום world",
            "مرحبا world",
            "aaa בּבבּ ccc",
            "mixed עברית and English 123",
        ] {
            let shaped =
                sys.shape_line_wrapped(0, text, 16.0, FontWeight::NORMAL, 0.0, WrapMode::Word);
            assert!(shaped.glyph_count() > 0, "bidi: {text:?}");
            assert!(shaped.width() >= 0.0);
            assert!(shaped.height() > 0.0);
            assert!(shaped.row_count() >= 1);
        }
    }

    #[test]
    fn bench_shape_lines_visible_100_lines() {
        use std::time::Instant;
        let mut sys = TextSystem::new();
        let lines: Vec<String> = (0..1000)
            .map(|i| format!("visible bench line {i:04} with some words to shape"))
            .collect();

        let _ = sys.shape_lines_visible(
            0..1000,
            |i| lines.get(i).cloned(),
            14.0,
            FontWeight::NORMAL,
            300.0,
            WrapMode::Word,
        );

        sys.reset_line_reshapes();
        let t0 = Instant::now();
        let vis = sys.shape_lines_visible(
            200..300,
            |i| lines.get(i).cloned(),
            14.0,
            FontWeight::NORMAL,
            300.0,
            WrapMode::Word,
        );
        let dt_miss_or_hit = t0.elapsed();
        assert_eq!(vis.len(), 100);

        sys.reset_line_reshapes();
        let t1 = Instant::now();
        let vis2 = sys.shape_lines_visible(
            200..300,
            |i| lines.get(i).cloned(),
            14.0,
            FontWeight::NORMAL,
            300.0,
            WrapMode::Word,
        );
        let dt_hit = t1.elapsed();
        assert_eq!(sys.line_reshapes(), 0, "VirtualList window fully cached");
        assert!(Rc::ptr_eq(&vis[0], &vis2[0]));
        eprintln!(
            "bench shape_lines_visible(100): pierwsze {dt_miss_or_hit:?}, hit {dt_hit:?} ({} linii w cache)",
            sys.line_cache_len()
        );
    }
}
