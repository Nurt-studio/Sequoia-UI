use sequoia_core::{Color, Point, Rect};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MinMax {
    pub min: f32,
    pub max: f32,
}

impl MinMax {
    pub const SILENCE: Self = Self { min: 0.0, max: 0.0 };

    pub fn new(min: f32, max: f32) -> Self {
        if min <= max {
            Self { min, max }
        } else {
            Self { min: max, max: min }
        }
    }

    pub fn span(self) -> f32 {
        let d = self.max - self.min;
        if d.is_finite() && d > 0.0 { d } else { 0.0 }
    }

    pub fn merge(self, other: Self) -> Self {
        let a_min = if self.min.is_finite() { self.min } else { 0.0 };
        let a_max = if self.max.is_finite() { self.max } else { 0.0 };
        let b_min = if other.min.is_finite() {
            other.min
        } else {
            0.0
        };
        let b_max = if other.max.is_finite() {
            other.max
        } else {
            0.0
        };
        Self {
            min: a_min.min(b_min),
            max: a_max.max(b_max),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Peaks {
    levels: Vec<Vec<MinMax>>,
    samples_per_peak: usize,
    total_samples: usize,
    version: u64,
}

impl Peaks {
    pub fn from_samples(samples: &[f32], samples_per_peak: usize) -> Self {
        let spp = samples_per_peak.max(1);
        let mut base = Vec::new();
        if !samples.is_empty() {
            base.reserve(samples.len().div_ceil(spp));
            for chunk in samples.chunks(spp) {
                let mut mn = f32::INFINITY;
                let mut mx = f32::NEG_INFINITY;
                for &s in chunk {
                    let v = if s.is_finite() { s } else { 0.0 };
                    mn = mn.min(v);
                    mx = mx.max(v);
                }
                base.push(MinMax::new(mn, mx));
            }
        }
        Self::from_base(base, spp, samples.len())
    }

    pub fn from_min_max(base: Vec<MinMax>, samples_per_peak: usize) -> Self {
        let n = base.len();
        Self::from_base(base, samples_per_peak.max(1), n)
    }

    fn from_base(base: Vec<MinMax>, spp: usize, total: usize) -> Self {
        let mut levels = Vec::new();
        if !base.is_empty() {
            levels.push(base);
            while levels.last().is_some_and(|l| l.len() > 1) {
                let prev = levels.last().expect("poziom istnieje");
                let mut next = Vec::with_capacity(prev.len().div_ceil(2));
                for pair in prev.chunks(2) {
                    match pair {
                        [a, b] => next.push(a.merge(*b)),
                        [a] => next.push(*a),
                        [] => {}
                        _ => {}
                    }
                }
                levels.push(next);
                if levels.len() > 32 {
                    break;
                }
            }
        }
        Self {
            levels,
            samples_per_peak: spp,
            total_samples: total,
            version: 1,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.levels.first().is_none_or(Vec::is_empty)
    }

    pub fn num_levels(&self) -> usize {
        self.levels.len()
    }

    pub fn base_len(&self) -> usize {
        self.levels.first().map_or(0, Vec::len)
    }

    pub fn samples_per_peak(&self) -> usize {
        self.samples_per_peak
    }

    pub fn total_samples(&self) -> usize {
        self.total_samples
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn level(&self, level: usize) -> &[MinMax] {
        self.levels.get(level).map_or(&[], Vec::as_slice)
    }

    pub fn level_for_width(&self, width: usize) -> usize {
        if self.levels.is_empty() || width == 0 {
            return 0;
        }
        for (i, l) in self.levels.iter().enumerate() {
            if l.len() <= width.max(1) {
                return i;
            }
        }
        self.levels.len() - 1
    }

    pub fn query(&self, level: usize, start: usize, count: usize, width: usize) -> Vec<MinMax> {
        let src = self.level(level);
        if src.is_empty() || width == 0 || count == 0 {
            return Vec::new();
        }
        let start = start.min(src.len());
        let end = (start + count).min(src.len());
        if start >= end {
            return Vec::new();
        }
        let slice = &src[start..end];
        if slice.len() == width {
            return slice.to_vec();
        }
        let mut out = Vec::with_capacity(width);
        for i in 0..width {
            let a = (i * slice.len()) / width;
            let mut b = ((i + 1) * slice.len()) / width;
            if b <= a {
                b = (a + 1).min(slice.len());
            }
            let mut m = slice[a];
            for &v in &slice[a + 1..b] {
                m = m.merge(v);
            }
            out.push(m);
        }
        out
    }

    pub fn query_for_width(
        &self,
        start_peak: usize,
        count: usize,
        width: usize,
    ) -> (usize, Vec<MinMax>) {
        if self.is_empty() || width == 0 {
            return (0, Vec::new());
        }
        let base = self.base_len();
        let start = start_peak.min(base);
        let count = if count == 0 {
            base.saturating_sub(start)
        } else {
            count.min(base - start)
        };
        if count == 0 {
            return (0, Vec::new());
        }
        let level = self.level_for_width(width);
        let div = 1usize << level.min(30);
        let lstart = start / div;
        let lcount = count.div_ceil(div).max(1);
        (level, self.query(level, lstart, lcount, width))
    }

    pub fn extend_samples(&mut self, samples: &[f32]) {
        if samples.is_empty() {
            return;
        }
        if self.levels.is_empty() {
            *self = Self::from_samples(samples, self.samples_per_peak.max(1));
            return;
        }
        let spp = self.samples_per_peak.max(1);
        let mut base = std::mem::take(&mut self.levels);
        let level0 = &mut base[0];
        for chunk in samples.chunks(spp) {
            if chunk.len() < spp {
                if let Some(last) = level0.last_mut() {
                    for &s in chunk {
                        let v = if s.is_finite() { s } else { 0.0 };
                        *last = last.merge(MinMax::new(v, v));
                    }
                } else {
                    let mut mn = f32::INFINITY;
                    let mut mx = f32::NEG_INFINITY;
                    for &s in chunk {
                        let v = if s.is_finite() { s } else { 0.0 };
                        mn = mn.min(v);
                        mx = mx.max(v);
                    }
                    level0.push(MinMax::new(mn, mx));
                }
            } else {
                let mut mn = f32::INFINITY;
                let mut mx = f32::NEG_INFINITY;
                for &s in chunk {
                    let v = if s.is_finite() { s } else { 0.0 };
                    mn = mn.min(v);
                    mx = mx.max(v);
                }
                level0.push(MinMax::new(mn, mx));
            }
        }
        base.truncate(1);
        while base.last().is_some_and(|l| l.len() > 1) {
            let prev = base.last().expect("poziom istnieje").clone();
            let mut next = Vec::with_capacity(prev.len().div_ceil(2));
            for pair in prev.chunks(2) {
                match pair {
                    [a, b] => next.push(a.merge(*b)),
                    [a] => next.push(*a),
                    [] => {}
                    _ => {}
                }
            }
            base.push(next);
            if base.len() > 32 {
                break;
            }
        }
        self.levels = base;
        self.total_samples += samples.len();
        self.version = self.version.wrapping_add(1).max(1);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaveformOpts {
    pub gap_px: f32,
    pub min_height_px: f32,
    pub mirrored: bool,
}

impl Default for WaveformOpts {
    fn default() -> Self {
        Self {
            gap_px: 0.0,
            min_height_px: 1.0,
            mirrored: false,
        }
    }
}

pub fn layout_bars(area: Rect, peaks: &[MinMax], opts: WaveformOpts) -> Vec<Rect> {
    if area.is_empty() || peaks.is_empty() {
        return Vec::new();
    }
    let n = peaks.len() as f32;
    let col_w = area.size.width / n;
    if !(col_w > 0.0) {
        return Vec::new();
    }
    let gap = opts.gap_px.clamp(0.0, col_w * 0.9);
    let bar_w = (col_w - gap).max(0.5);
    let mid_y = area.top() + area.size.height * 0.5;
    let half_h = area.size.height * 0.5;
    let mut out = Vec::with_capacity(peaks.len());
    for (i, p) in peaks.iter().enumerate() {
        let x = area.left() + i as f32 * col_w + gap * 0.5;
        let (top, bottom) = if opts.mirrored {
            let lo = p.min.clamp(-1.0, 1.0);
            let hi = p.max.clamp(-1.0, 1.0);
            let amp = lo.abs().max(hi.abs()).clamp(0.0, 1.0);
            let h = (amp * area.size.height).max(opts.min_height_px.max(0.0));
            (mid_y - h * 0.5, mid_y + h * 0.5)
        } else {
            let lo = p.min.clamp(-1.0, 1.0);
            let hi = p.max.clamp(-1.0, 1.0);
            let y0 = mid_y - hi * half_h;
            let y1 = mid_y - lo * half_h;
            let (mut t, mut b) = (y0.min(y1), y0.max(y1));
            if b - t < opts.min_height_px {
                let c = (t + b) * 0.5;
                t = c - opts.min_height_px * 0.5;
                b = c + opts.min_height_px * 0.5;
            }
            (t, b)
        };
        let top = top.max(area.top());
        let bottom = bottom.min(area.bottom());
        if bottom > top {
            out.push(Rect::from_xywh(x, top, bar_w, bottom - top));
        }
    }
    out
}

pub fn waveform_line_points(area: Rect, peaks: &[MinMax]) -> Vec<Point> {
    if area.is_empty() || peaks.is_empty() {
        return Vec::new();
    }
    let n = peaks.len() as f32;
    let col_w = area.size.width / n;
    if !(col_w > 0.0) || !(area.size.height > 0.0) {
        return Vec::new();
    }
    let map_y = |v: f32| {
        let c = v.clamp(-1.0, 1.0);
        area.top() + (1.0 - c) * 0.5 * area.size.height
    };
    let mut out = Vec::with_capacity(peaks.len() * 2);
    for (i, p) in peaks.iter().enumerate() {
        let x = area.left() + (i as f32 + 0.5) * col_w;
        out.push(Point::new(x, map_y(p.min)));
        out.push(Point::new(x, map_y(p.max)));
    }
    out
}

pub fn bake_strip(peaks: &[MinMax], w: u32, h: u32, fg: Color, bg: Color) -> Vec<u8> {
    if w == 0 || h == 0 {
        return Vec::new();
    }

    const MAX_DIM: u32 = 4096;
    const MAX_BYTES: usize = 64 * 1024 * 1024;
    if w > MAX_DIM || h > MAX_DIM {
        return Vec::new();
    }
    let bytes = (w as usize)
        .checked_mul(h as usize)
        .and_then(|v| v.checked_mul(4))
        .filter(|&v| v <= MAX_BYTES);
    let Some(bytes) = bytes else {
        return Vec::new();
    };
    let (fr, fgc, fb, fa) = fg.to_rgba8();
    let (br, bgc, bb, ba) = bg.to_rgba8();
    let mut px = vec![0u8; bytes];
    for p in px.chunks_exact_mut(4) {
        p[0] = br;
        p[1] = bgc;
        p[2] = bb;
        p[3] = ba;
    }
    if peaks.is_empty() {
        return px;
    }
    let n = peaks.len();
    let hh = h as f32;
    for x in 0..(w as usize) {
        let peak = peaks[(x * n) / (w as usize)];
        let hi = peak.max.clamp(-1.0, 1.0);
        let lo = peak.min.clamp(-1.0, 1.0);
        let mut y0 = (((1.0 - hi) * 0.5 * hh).floor() as isize).clamp(0, h as isize - 1);
        let mut y1 = (((1.0 - lo) * 0.5 * hh).ceil() as isize).clamp(0, h as isize - 1);
        if y1 < y0 {
            std::mem::swap(&mut y0, &mut y1);
        }

        for y in y0..=y1 {
            let o = (y as usize * (w as usize) + x) * 4;
            px[o] = fr;
            px[o + 1] = fgc;
            px[o + 2] = fb;
            px[o + 3] = fa;
        }
    }
    px
}

#[derive(Clone, Debug, Default)]
pub struct WaveformCache {
    key: Option<(u64, usize, usize, usize, u64, u32)>,
    bars: Vec<Rect>,
}

impl WaveformCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bars(&self) -> &[Rect] {
        &self.bars
    }

    pub fn is_empty(&self) -> bool {
        self.bars.is_empty()
    }

    pub fn invalidate(&mut self) {
        self.key = None;
        self.bars.clear();
    }

    pub fn update(
        &mut self,
        area: Rect,
        peaks: &Peaks,
        start_peak: usize,
        count: usize,
        width_px: usize,
        opts: WaveformOpts,
    ) -> bool {
        let key = (
            area_key(area),
            start_peak,
            count,
            width_px,
            peaks.version(),
            opts_key(opts),
        );
        if self.key == Some(key) {
            return false;
        }
        let (_, cols) = peaks.query_for_width(start_peak, count, width_px);
        let bars = layout_bars(area, &cols, opts);
        self.bars = bars;
        self.key = Some(key);
        true
    }

    pub fn update_cols(&mut self, area: Rect, cols: &[MinMax], opts: WaveformOpts) -> bool {
        let mut hash: u64 = area_key(area) ^ opts_key(opts) as u64;
        for c in cols {
            hash = hash.wrapping_mul(0x1000_0000_01b3)
                ^ ((c.min.to_bits() as u64).wrapping_add((c.max.to_bits() as u64) << 1));
        }
        let key = (hash, 0, 0, cols.len(), 0, 0);
        if self.key == Some(key) {
            return false;
        }
        self.bars = layout_bars(area, cols, opts);
        self.key = Some(key);
        true
    }
}

fn area_key(r: Rect) -> u64 {
    let q = |v: f32| (v * 2.0).round() as i32 as u64;
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for v in [r.left(), r.top(), r.size.width, r.size.height] {
        h ^= q(v).wrapping_add(0x9e37_79b9_7f4a_7c15);
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    h
}

fn opts_key(o: WaveformOpts) -> u32 {
    o.gap_px.to_bits()
        ^ o.min_height_px.to_bits().wrapping_mul(31)
        ^ (o.mirrored as u32).wrapping_mul(0x9e37)
}

pub fn white_noise(len: usize, seed: u32, gain: f32) -> Vec<f32> {
    let mut st = seed.max(1);
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        st ^= st << 13;
        st ^= st >> 17;
        st ^= st << 5;
        let u = (st as f64) / (u32::MAX as f64);
        out.push(((u * 2.0 - 1.0) as f32) * gain);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minmax_merge_clamps_nan() {
        let a = MinMax::new(f32::NAN, 1.0);
        let b = MinMax::new(-1.0, f32::NAN);
        let m = a.merge(b);
        assert!(m.min.is_finite() && m.max.is_finite());
        assert!(m.min <= m.max);
        assert_eq!(MinMax::new(1.0, -1.0), MinMax::new(-1.0, 1.0));
    }

    #[test]
    fn pyramid_halves_and_merges() {
        let s: Vec<f32> = (0..1024)
            .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
            .collect();
        let p = Peaks::from_samples(&s, 4);
        assert_eq!(p.base_len(), 256);
        assert!(p.num_levels() >= 8);
        let top = p.level(p.num_levels() - 1);
        assert_eq!(top.len(), 1);
        assert!((top[0].min - -0.5).abs() < 1e-6);
        assert!((top[0].max - 0.5).abs() < 1e-6);
    }

    #[test]
    fn query_resamples_to_width() {
        let s: Vec<f32> = (0..1000).map(|i| (i as f32 / 1000.0) * 2.0 - 1.0).collect();
        let p = Peaks::from_samples(&s, 1);
        let cols = p.query(0, 0, p.base_len(), 100);
        assert_eq!(cols.len(), 100);
        assert!(cols[0].max < cols[99].min);
    }

    #[test]
    fn query_for_width_picks_level() {
        let p = Peaks::from_samples(&white_noise(4096, 7, 0.9), 4);
        let (lvl, cols) = p.query_for_width(0, 0, 128);
        assert_eq!(cols.len(), 128);
        assert!(lvl < p.num_levels());
    }

    #[test]
    fn layout_bars_fills_area() {
        let area = Rect::from_xywh(0.0, 0.0, 100.0, 40.0);
        let peaks = vec![MinMax::new(-1.0, 1.0); 10];
        let bars = layout_bars(area, &peaks, WaveformOpts::default());
        assert_eq!(bars.len(), 10);
        for b in &bars {
            assert!(b.top() >= area.top() - 0.01 && b.bottom() <= area.bottom() + 0.01);
        }
        let quiet = layout_bars(area, &[MinMax::SILENCE], WaveformOpts::default());
        assert_eq!(quiet.len(), 1);
        assert!(quiet[0].size.height >= 1.0);
    }

    #[test]
    fn line_points_zigzag_two_per_column() {
        let area = Rect::from_xywh(0.0, 10.0, 100.0, 40.0);
        let peaks = vec![
            MinMax::new(-1.0, 1.0),
            MinMax::new(-0.5, 0.0),
            MinMax::SILENCE,
        ];
        let pts = waveform_line_points(area, &peaks);
        assert_eq!(pts.len(), 6, "two points per column: min, then max");
        assert!(
            (pts[0].y - 50.0).abs() < 1e-4,
            "min=-1 is bottom: {:?}",
            pts[0]
        );
        assert!(
            (pts[1].y - 10.0).abs() < 1e-4,
            "max=+1 is top: {:?}",
            pts[1]
        );
        assert!((pts[2].y - 40.0).abs() < 1e-4, "{:?}", pts[2]);
        assert!((pts[3].y - 30.0).abs() < 1e-4, "{:?}", pts[3]);
        assert!((pts[4].y - 30.0).abs() < 1e-4, "{:?}", pts[4]);
        assert!((pts[5].y - 30.0).abs() < 1e-4, "{:?}", pts[5]);
        for w in pts.windows(2) {
            assert!(w[1].x >= w[0].x);
        }
        assert_eq!(pts[0].x, pts[1].x, "para dzieli x kolumny");
        assert!(pts[2].x > pts[1].x);
        assert!(waveform_line_points(Rect::ZERO, &peaks).is_empty());
        assert!(waveform_line_points(area, &[]).is_empty());
    }

    #[test]
    fn cache_skips_recompute() {
        let p = Peaks::from_samples(&white_noise(2048, 1, 0.8), 8);
        let mut c = WaveformCache::new();
        let area = Rect::from_xywh(0.0, 0.0, 200.0, 60.0);
        assert!(c.update(area, &p, 0, 0, 200, WaveformOpts::default()));
        assert!(!c.update(area, &p, 0, 0, 200, WaveformOpts::default()));
        assert!(c.update(area, &p, 10, 0, 200, WaveformOpts::default()));
    }

    #[test]
    fn bake_strip_size_and_colors() {
        let peaks = vec![MinMax::new(-0.5, 0.5); 8];
        let px = bake_strip(&peaks, 8, 4, Color::WHITE, Color::BLACK);
        assert_eq!(px.len(), 8 * 4 * 4);
        assert_eq!(&px[0..4], &[0, 0, 0, 255]);
        let row1 = 8 * 4;
        assert_eq!(&px[row1..row1 + 4], &[255, 255, 255, 255]);
        let pos = vec![MinMax::new(0.4, 0.9); 4];
        let px = bake_strip(&pos, 4, 8, Color::WHITE, Color::BLACK);
        let bottom = 7 * 4 * 4;
        assert_eq!(&px[bottom..bottom + 4], &[0, 0, 0, 255]);
        assert_eq!(&px[0..4], &[255, 255, 255, 255]);
    }

    #[test]
    fn white_noise_deterministic() {
        assert_eq!(white_noise(16, 42, 1.0), white_noise(16, 42, 1.0));
        assert_ne!(white_noise(16, 42, 1.0), white_noise(16, 43, 1.0));
    }

    #[test]
    fn extend_appends() {
        let mut p = Peaks::from_samples(&white_noise(256, 1, 0.9), 16);
        let v0 = p.version();
        let n0 = p.base_len();
        p.extend_samples(&white_noise(256, 2, 0.9));
        assert!(p.base_len() > n0);
        assert_ne!(p.version(), v0);
    }
}
