use std::rc::Rc;

use sequoia_core::{Color, Length};
use sequoia_input::{CursorIcon, Event, EventPhase, PointerButton, PointerEvent};
use sequoia_reactive::Signal;
use sequoia_style::VisualState;
use sequoia_view::prelude::*;

use crate::Container;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TabTear(pub usize);

const HEIGHT: f32 = 34.0;
const PAD_X: f32 = 12.0;
const GAP: f32 = 8.0;
const FONT_SIZE: f32 = 11.0;
const ACCENT_W: f32 = 1.0;

#[derive(Clone)]
pub struct TabStrip {
    active: Signal<usize>,
    labels: Vec<String>,
    closable: Vec<bool>,
    tearable: Vec<bool>,
    on_select: Option<Rc<dyn Fn(usize)>>,
    on_close: Option<Rc<dyn Fn(usize)>>,
    on_tear_end: Option<Rc<dyn Fn(usize, DragEnd)>>,
    tear_preview_for: Option<Rc<dyn Fn(usize) -> AnyView>>,
    on_tear_start: Option<Rc<dyn Fn(usize)>>,
    height: f32,
    equal_width: bool,
    pad_x: f32,
    gap: f32,
    font_size: f32,
    active_bg: Color,
    accent: Color,
    label_color: Color,
    active_label_color: Color,
    close_color: Color,
}

impl TabStrip {
    pub fn new(active: Signal<usize>, labels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let labels: Vec<String> = labels.into_iter().map(Into::into).collect();
        Self {
            active,
            closable: vec![false; labels.len()],
            tearable: vec![false; labels.len()],
            labels,
            on_select: None,
            on_close: None,
            on_tear_end: None,
            tear_preview_for: None,
            on_tear_start: None,
            height: HEIGHT,
            equal_width: true,
            pad_x: PAD_X,
            gap: GAP,
            font_size: FONT_SIZE,

            active_bg: Color::hex(0x333333),
            accent: Color::hex(0x1C80E3),
            label_color: Color::hex(0x9A9A9A),
            active_label_color: Color::hex(0xD4D4D4),
            close_color: Color::hex(0x9A9A9A),
        }
    }

    pub fn on_select(mut self, f: impl Fn(usize) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }

    pub fn on_close(mut self, f: impl Fn(usize) + 'static) -> Self {
        self.on_close = Some(Rc::new(f));
        self
    }

    pub fn on_tear_end(mut self, f: impl Fn(usize, DragEnd) + 'static) -> Self {
        self.on_tear_end = Some(Rc::new(f));
        self
    }

    pub fn on_tear_start(mut self, f: impl Fn(usize) + 'static) -> Self {
        self.on_tear_start = Some(Rc::new(f));
        self
    }

    pub fn closable(mut self, flags: impl IntoIterator<Item = bool>) -> Self {
        self.closable = flags.into_iter().collect();
        self.closable.resize(self.labels.len(), false);
        self
    }

    pub fn tearable(mut self, flags: impl IntoIterator<Item = bool>) -> Self {
        self.tearable = flags.into_iter().collect();
        self.tearable.resize(self.labels.len(), false);
        self
    }

    pub fn tear_preview_for(mut self, f: impl Fn(usize) -> AnyView + 'static) -> Self {
        self.tear_preview_for = Some(Rc::new(f));
        self
    }

    pub fn height(mut self, v: f32) -> Self {
        self.height = v.max(0.0);
        self
    }

    pub fn equal_width(mut self, on: bool) -> Self {
        self.equal_width = on;
        self
    }

    pub fn padding_x(mut self, v: f32) -> Self {
        self.pad_x = v.max(0.0);
        self
    }

    pub fn gap(mut self, v: f32) -> Self {
        self.gap = v.max(0.0);
        self
    }

    pub fn font_size(mut self, v: f32) -> Self {
        self.font_size = v.max(0.0);
        self
    }

    pub fn active_bg(mut self, color: Color) -> Self {
        self.active_bg = color;
        self
    }

    pub fn accent(mut self, color: Color) -> Self {
        self.accent = color;
        self
    }

    pub fn label_color(mut self, color: Color) -> Self {
        self.label_color = color;
        self
    }

    pub fn active_label_color(mut self, color: Color) -> Self {
        self.active_label_color = color;
        self
    }

    pub fn close_color(mut self, color: Color) -> Self {
        self.close_color = color;
        self
    }

    fn tab(&self, i: usize, label: &str) -> impl View + use<> {
        let active = self.active.get_untracked() == i;

        let mut box_ = Container::new().height(self.height).background(if active {
            self.active_bg
        } else {
            Color::TRANSPARENT
        });
        if active {
            box_ = box_.border_top(ACCENT_W, self.accent);
        }
        let mut row = Row::new()
            .height(self.height)
            .gap(self.gap)
            .padding_x(self.pad_x)
            .align_items(Align::Center)
            .child(
                Text::new(label.to_string())
                    .size(self.font_size)
                    .bold()
                    .color(if active {
                        self.active_label_color
                    } else {
                        self.label_color
                    }),
            );
        if self.closable.get(i).copied().unwrap_or(false) {
            let close_color = self.close_color;
            let on_close = self.on_close.clone();
            row = row.child(
                Container::new()
                    .child(Text::new("×").size(self.font_size).color(close_color))
                    .padding_xy(4.0, 2.0)
                    .on_click(move || {
                        if let Some(f) = &on_close {
                            f(i);
                        }
                    }),
            );
        }

        let on_select = self.on_select.clone();
        let click = Container::new()
            .child(row)
            .width(Length::Fill(1.0))
            .height(Length::Fill(1.0))
            .on_click(move || {
                if let Some(f) = &on_select {
                    f(i);
                }
            });
        if self.tearable.get(i).copied().unwrap_or(false) {
            let on_tear_end = self.on_tear_end.clone();
            let on_tear_start = self.on_tear_start.clone();
            let preview = match &self.tear_preview_for {
                Some(f) => f(i),
                None => AnyView::new(tear_preview(
                    label,
                    self.font_size,
                    self.active_bg,
                    self.active_label_color,
                )),
            };
            box_.child(
                TearGrab::new(click, i)
                    .on_tear_start(move |idx| {
                        if let Some(f) = &on_tear_start {
                            f(idx);
                        }
                    })
                    .on_tear_end(move |idx, end| {
                        if let Some(f) = &on_tear_end {
                            f(idx, end);
                        }
                    })
                    .preview(preview),
            )
        } else {
            box_.child(click)
        }
    }
}

impl View for TabStrip {
    fn body(&self) -> impl View + 'static {
        let _ = self.active.get();
        let mut row = Row::new()
            .fill_width()
            .height(self.height)
            .align_items(Align::Stretch);
        for (i, label) in self.labels.iter().enumerate() {
            let tab = self.tab(i, label);
            row = if self.equal_width {
                row.child_flex(tab, 1.0)
            } else {
                row.child(tab)
            };
        }
        row
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Px(self.height),
            ..LayoutStyle::default()
        }
    }

    fn has_visual_state(&self) -> bool {
        true
    }
}

fn tear_preview(label: &str, font_size: f32, bg: Color, fg: Color) -> impl View + use<> {
    Container::new()
        .background(bg)
        .radius(4.0)
        .padding_xy(12.0, 8.0)
        .child(
            Text::new(label.to_string())
                .size(font_size)
                .bold()
                .color(fg),
        )
}

#[derive(Clone)]
pub(crate) struct TearGrab {
    child: AnyView,
    index: usize,
    preview: Option<AnyView>,
    on_tear_start: Option<Rc<dyn Fn(usize)>>,
    on_tear_end: Option<Rc<dyn Fn(usize, DragEnd)>>,
}

impl TearGrab {
    pub(crate) fn new<V: View + 'static>(child: V, index: usize) -> Self {
        Self {
            child: AnyView::new(child),
            index,
            preview: None,
            on_tear_start: None,
            on_tear_end: None,
        }
    }

    pub(crate) fn preview<V: View + 'static>(mut self, preview: V) -> Self {
        self.preview = Some(AnyView::new(preview));
        self
    }

    pub(crate) fn on_tear_start(mut self, f: impl Fn(usize) + 'static) -> Self {
        self.on_tear_start = Some(Rc::new(f));
        self
    }

    pub(crate) fn on_tear_end(mut self, f: impl Fn(usize, DragEnd) + 'static) -> Self {
        self.on_tear_end = Some(Rc::new(f));
        self
    }
}

impl View for TearGrab {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        Some(vec![self.child.clone()])
    }

    fn layout_style(&self) -> LayoutStyle {
        LayoutStyle {
            width: Length::Fit,
            height: Length::Fit,
            ..LayoutStyle::default()
        }
    }

    fn has_visual_state(&self) -> bool {
        true
    }

    fn cursor_icon(&self, state: VisualState) -> CursorIcon {
        if state.pressed {
            CursorIcon::Grabbing
        } else {
            CursorIcon::Grab
        }
    }

    fn on_event(&self, event: &Event, ctx: &mut EventCtx) {
        if ctx.phase == EventPhase::Bubble {
            return;
        }
        match event {
            Event::Pointer(PointerEvent::Down {
                button: PointerButton::Left,
                pos,
                ..
            }) if ctx.rect.contains(*pos) => {
                let mut opts = DragOptions::new().threshold(DRAG_THRESHOLD);
                if let Some(pv) = self.preview.clone() {
                    opts = opts.preview(pv);
                }
                ctx.begin_drag(TabTear(self.index), opts);
            }
            _ => {}
        }
    }

    fn on_drag_end(&self, end: DragEnd, _ctx: &mut EventCtx) {
        if let Some(f) = &self.on_tear_end {
            f(self.index, end);
        }
    }

    fn on_drag_start(&self, _ctx: &mut EventCtx) {
        if let Some(f) = &self.on_tear_start {
            f(self.index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tearable_defaults_to_none_and_resizes() {
        let s = TabStrip::new(Signal::new(0usize), ["A", "B", "C"]);
        assert!(s.tearable.iter().all(|c| !c));
        let s = s.tearable([false, true]);
        assert!(!s.tearable[0] && s.tearable[1] && !s.tearable[2]);
    }

    #[test]
    fn tear_payload_carries_tab_index() {
        use sequoia_view::DragPayload;
        let p = DragPayload::new(TabTear(1));
        assert!(p.is::<TabTear>());
        assert_eq!(p.get::<TabTear>(), Some(&TabTear(1)));
        assert!(!p.is::<String>());
    }

    #[test]
    fn closable_defaults_to_none_and_resizes() {
        let s = TabStrip::new(Signal::new(0usize), ["A", "B", "C"]);
        assert!(s.closable.iter().all(|c| !c));

        let s = s.closable([true]);
        assert!(s.closable[0] && !s.closable[1] && !s.closable[2]);
    }

    #[test]
    fn strip_is_fill_width_with_fixed_height() {
        let s = TabStrip::new(Signal::new(0usize), ["A"]).height(28.0);
        let style = s.layout_style();
        assert_eq!(style.width, Length::Fill(1.0));
        assert_eq!(style.height, Length::Px(28.0));
    }
}
