use crate::dock::sanitize_gap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    pub const fn is_horizontal(self) -> bool {
        matches!(self, Self::Horizontal)
    }

    pub const fn is_vertical(self) -> bool {
        matches!(self, Self::Vertical)
    }

    pub fn split_main_cross(self, width: f32, height: f32) -> (f32, f32) {
        match self {
            Self::Horizontal => (width, height),
            Self::Vertical => (height, width),
        }
    }

    pub fn merge_main_cross(self, main: f32, cross: f32) -> (f32, f32) {
        match self {
            Self::Horizontal => (main, cross),
            Self::Vertical => (cross, main),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Justify {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

impl Justify {
    pub fn distribute(self, count: usize, _base_gap: f32, extra: f32) -> (f32, f32) {
        let extra = if extra.is_finite() {
            extra.max(0.0)
        } else {
            0.0
        };
        match self {
            Self::Start => (0.0, 0.0),
            Self::Center => (extra * 0.5, 0.0),
            Self::End => (extra, 0.0),
            Self::SpaceBetween => {
                if count > 1 {
                    (0.0, extra / (count - 1) as f32)
                } else if count == 1 {
                    (extra * 0.5, 0.0)
                } else {
                    (0.0, 0.0)
                }
            }
            Self::SpaceAround => {
                if count > 0 {
                    let step = extra / count as f32;
                    (step * 0.5, step)
                } else {
                    (0.0, 0.0)
                }
            }
            Self::SpaceEvenly => {
                if count > 0 {
                    let step = extra / (count + 1) as f32;
                    (step, step)
                } else {
                    (0.0, 0.0)
                }
            }
        }
    }

    pub fn start_offset(self, extra: f32) -> f32 {
        let (start, _) = self.distribute(0, 0.0, extra);
        match self {
            Self::Start | Self::Center | Self::End => start,
            _ => start,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
    Stretch,
}

impl Align {
    pub const fn is_stretch(self) -> bool {
        matches!(self, Self::Stretch)
    }

    pub fn offset(self, avail: f32, child: f32) -> f32 {
        if !avail.is_finite() {
            return 0.0;
        }
        let slack = (avail - child).max(0.0);
        match self {
            Self::Start | Self::Stretch => 0.0,
            Self::Center => slack * 0.5,
            Self::End => slack,
        }
    }

    pub fn resolve(child: Option<Align>, container: Align) -> Align {
        child.unwrap_or(container)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContainerStyle {
    pub axis: Axis,

    pub gap: f32,

    pub cross_gap: f32,
    pub align: Align,
    pub justify: Justify,

    pub wrap: bool,

    pub align_content: Align,
}

impl Default for ContainerStyle {
    fn default() -> Self {
        Self {
            axis: Axis::Horizontal,
            gap: 0.0,
            cross_gap: 0.0,
            align: Align::Start,
            justify: Justify::Start,
            wrap: false,
            align_content: Align::Start,
        }
    }
}

impl ContainerStyle {
    pub fn new(axis: Axis, gap: f32) -> Self {
        let gap = sanitize_gap(gap);
        Self {
            axis,
            gap,
            cross_gap: gap,
            ..Self::default()
        }
    }

    pub fn gap(mut self, gap: f32) -> Self {
        let gap = sanitize_gap(gap);
        self.gap = gap;
        self.cross_gap = gap;
        self
    }

    pub fn main_gap(mut self, gap: f32) -> Self {
        self.gap = sanitize_gap(gap);
        self
    }

    pub fn cross_gap(mut self, gap: f32) -> Self {
        self.cross_gap = sanitize_gap(gap);
        self
    }

    pub fn gaps(mut self, main: f32, cross: f32) -> Self {
        self.gap = sanitize_gap(main);
        self.cross_gap = sanitize_gap(cross);
        self
    }

    pub fn align_items(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    pub fn justify_content(mut self, justify: Justify) -> Self {
        self.justify = justify;
        self
    }

    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn wrapped(mut self) -> Self {
        self.wrap = true;
        self
    }

    pub fn align_content(mut self, align: Align) -> Self {
        self.align_content = align;
        self
    }

    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    pub fn main_gaps_total(self, count: usize) -> f32 {
        if count > 1 {
            self.gap * (count - 1) as f32
        } else {
            0.0
        }
    }
}
