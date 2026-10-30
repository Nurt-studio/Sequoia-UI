use std::rc::Rc;

use sequoia_core::Length;
use sequoia_reactive::Signal;
use sequoia_style::{FORM_ERROR, FormStyle};
use sequoia_view::{AnyView, Column, Row, Text, Unit, View};

use crate::button::Button;
use crate::container::Container;
use crate::drag_value::DragValue;
use crate::floating_menu::FloatingMenu;
use crate::text_field::TextField;

pub trait ValidField {
    fn is_valid(&self) -> bool;
    fn touch(&self);

    fn error_text(&self) -> Option<String> {
        None
    }

    fn track(&self) {}
}

type RuleFn = Rc<dyn Fn(&str) -> bool>;

#[derive(Clone)]
struct Rule {
    message: String,
    check: RuleFn,
}

#[derive(Clone)]
pub struct Field {
    value: Signal<String>,
    label: String,
    placeholder: String,
    rules: Vec<Rule>,
    touched: Signal<bool>,
    editing: Signal<bool>,
    style: Option<FormStyle>,
}

impl Field {
    pub fn new(value: Signal<String>, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
            placeholder: String::new(),
            rules: Vec::new(),
            touched: Signal::new(false),
            editing: Signal::new(false),
            style: None,
        }
    }

    pub fn style(mut self, style: FormStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn rule(
        mut self,
        message: impl Into<String>,
        check: impl Fn(&str) -> bool + 'static,
    ) -> Self {
        self.rules.push(Rule {
            message: message.into(),
            check: Rc::new(check),
        });
        self
    }

    pub fn error(&self) -> Option<String> {
        let v = self.value.get_untracked();
        self.rules
            .iter()
            .find(|r| !(r.check)(&v))
            .map(|r| r.message.clone())
    }

    pub fn is_valid(&self) -> bool {
        let v = self.value.get_untracked();
        self.rules.iter().all(|r| (r.check)(&v))
    }

    pub fn touch(&self) {
        self.touched.set(true);
    }

    pub fn value_signal(&self) -> Signal<String> {
        self.value.clone()
    }

    pub fn editing_signal(&self) -> Signal<bool> {
        self.editing.clone()
    }

    pub fn label_text(&self) -> &str {
        &self.label
    }
}

impl ValidField for Field {
    fn is_valid(&self) -> bool {
        Field::is_valid(self)
    }

    fn touch(&self) {
        Field::touch(self)
    }

    fn error_text(&self) -> Option<String> {
        if self.touched.get_untracked() {
            self.error()
        } else {
            None
        }
    }

    fn track(&self) {
        let _ = self.value.get();
        let _ = self.touched.get();
    }
}

impl View for Field {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let f = self.clone();

        let _ = f.value.get();
        let touched = f.touched.get();
        let error = if touched { f.error() } else { None };
        let label = Text::new(f.label.clone())
            .size(label_size_of(f.style))
            .bold();
        let editing = f.editing.clone();
        let touched_sig = f.touched.clone();
        let editing_end = f.editing.clone();
        let touched_submit = f.touched.clone();
        let editing_submit = f.editing.clone();
        let input = TextField::new(f.value.clone())
            .placeholder(f.placeholder.clone())
            .on_edit_start(move || editing.set(true))
            .on_edit_end(move |_| {
                editing_end.set(false);
                touched_sig.set(true);
            })
            .on_submit(move |_| {
                editing_submit.set(false);
                touched_submit.set(true);
            });
        let mut col = Column::new().gap(4.0).child(label).child(input);
        if let Some(msg) = error {
            col = col.child(
                Text::new(msg)
                    .size(error_size_of(f.style))
                    .color(error_color_of(f.style)),
            );
        }
        Some(vec![AnyView::new(col)])
    }

    fn layout_style(&self) -> sequoia_layout::LayoutStyle {
        sequoia_layout::LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Fit,
            ..sequoia_layout::LayoutStyle::default()
        }
    }
}

const ERROR_RED: sequoia_core::Color = FORM_ERROR;

fn label_size_of(style: Option<FormStyle>) -> f32 {
    style.and_then(|s| s.label_size).unwrap_or(12.0)
}

fn error_size_of(style: Option<FormStyle>) -> f32 {
    style.and_then(|s| s.error_size).unwrap_or(11.0)
}

fn error_color_of(style: Option<FormStyle>) -> sequoia_core::Color {
    style.and_then(|s| s.error).unwrap_or(ERROR_RED)
}

fn styled_button(button: Button, style: Option<FormStyle>) -> Button {
    match style.and_then(|s| s.button) {
        Some(button_style) => button.style(button_style),
        None => button,
    }
}

#[derive(Clone, Default)]
pub struct Form {
    fields: Rc<std::cell::RefCell<Vec<Rc<dyn ValidField>>>>,
}

impl Form {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<F: ValidField + Clone + 'static>(&self, field: &F) {
        struct Wrap<F>(F);
        impl<F: ValidField> ValidField for Wrap<F> {
            fn is_valid(&self) -> bool {
                self.0.is_valid()
            }
            fn touch(&self) {
                self.0.touch();
            }
            fn error_text(&self) -> Option<String> {
                self.0.error_text()
            }
            fn track(&self) {
                self.0.track();
            }
        }
        self.fields.borrow_mut().push(Rc::new(Wrap(field.clone())));
    }

    pub fn is_valid(&self) -> bool {
        self.fields.borrow().iter().all(|f| f.is_valid())
    }

    pub fn touch_all(&self) {
        for f in self.fields.borrow().iter() {
            f.touch();
        }
    }

    pub fn len(&self) -> usize {
        self.fields.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.fields.borrow().is_empty()
    }

    pub fn summary(&self) -> AnyView {
        let fields = self.fields.clone();
        AnyView::new(Column::new().child(Text::with(move || {
            let mut first: Option<String> = None;
            for f in fields.borrow().iter() {
                f.track();
                if first.is_none() {
                    first = f.error_text();
                }
            }
            first.unwrap_or_default()
        })))
    }
}

#[derive(Clone)]
pub struct NumberField {
    value: Signal<f32>,
    label: String,
    min: f32,
    max: f32,
    step: f32,
    precision: usize,
    rules: Vec<NumRule>,
    touched: Signal<bool>,
    editing: Signal<bool>,
    style: Option<FormStyle>,
}

type NumRuleFn = Rc<dyn Fn(f32) -> bool>;

#[derive(Clone)]
struct NumRule {
    message: String,
    check: NumRuleFn,
}

impl NumberField {
    pub fn new(value: Signal<f32>, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
            min: f32::NEG_INFINITY,
            max: f32::INFINITY,
            step: 1.0,
            precision: 0,
            rules: Vec::new(),
            touched: Signal::new(false),
            editing: Signal::new(false),
            style: None,
        }
    }

    pub fn style(mut self, style: FormStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn min(mut self, min: f32) -> Self {
        self.min = min;
        self
    }

    pub fn max(mut self, max: f32) -> Self {
        self.max = max;
        self
    }

    pub fn step(mut self, step: f32) -> Self {
        if step.is_finite() && step > 0.0 {
            self.step = step;
        }
        self
    }

    pub fn precision(mut self, precision: usize) -> Self {
        self.precision = precision;
        self
    }

    pub fn rule(
        mut self,
        message: impl Into<String>,
        check: impl Fn(f32) -> bool + 'static,
    ) -> Self {
        self.rules.push(NumRule {
            message: message.into(),
            check: Rc::new(check),
        });
        self
    }

    pub fn error(&self) -> Option<String> {
        let v = self.value.get_untracked();
        self.rules
            .iter()
            .find(|r| !(r.check)(v))
            .map(|r| r.message.clone())
    }

    pub fn is_valid(&self) -> bool {
        let v = self.value.get_untracked();
        self.rules.iter().all(|r| (r.check)(v))
    }

    pub fn touch(&self) {
        self.touched.set(true);
    }

    pub fn value_signal(&self) -> Signal<f32> {
        self.value.clone()
    }

    pub fn editing_signal(&self) -> Signal<bool> {
        self.editing.clone()
    }

    fn clamp(&self, v: f32) -> f32 {
        v.clamp(self.min, self.max)
    }

    fn nudge(&self, dir: f32) {
        let v = self.value.get_untracked();
        self.value.set(self.clamp(v + dir * self.step));
        self.touch();
    }
}

impl ValidField for NumberField {
    fn is_valid(&self) -> bool {
        NumberField::is_valid(self)
    }

    fn touch(&self) {
        NumberField::touch(self)
    }

    fn error_text(&self) -> Option<String> {
        if self.touched.get_untracked() {
            self.error()
        } else {
            None
        }
    }

    fn track(&self) {
        let _ = self.value.get();
        let _ = self.touched.get();
    }
}

impl View for NumberField {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let f = self.clone();
        let _ = f.value.get();
        let touched = f.touched.get();
        let error = if touched { f.error() } else { None };
        let editing = f.editing.clone();
        let editing_end = f.editing.clone();
        let touched_end = f.touched.clone();
        let drag = DragValue::new(f.value.clone())
            .min(f.min)
            .max(f.max)
            .precision(f.precision)
            .on_change_start(move || editing.set(true))
            .on_commit(move |_| {
                editing_end.set(false);
                touched_end.set(true);
            });
        let minus = f.clone();
        let plus = f.clone();
        let mut col = Column::new()
            .gap(4.0)
            .child(
                Text::new(f.label.clone())
                    .size(label_size_of(f.style))
                    .bold(),
            )
            .child(
                Row::new()
                    .gap(4.0)
                    .child(styled_button(
                        Button::new("-").on_click(move || minus.nudge(-1.0)),
                        f.style,
                    ))
                    .child_flex(Container::new().fill_size().center().child(drag), 1.0)
                    .child(styled_button(
                        Button::new("+").on_click(move || plus.nudge(1.0)),
                        f.style,
                    )),
            );
        if let Some(msg) = error {
            col = col.child(
                Text::new(msg)
                    .size(error_size_of(f.style))
                    .color(error_color_of(f.style)),
            );
        }
        Some(vec![AnyView::new(col)])
    }

    fn layout_style(&self) -> sequoia_layout::LayoutStyle {
        sequoia_layout::LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Fit,
            ..sequoia_layout::LayoutStyle::default()
        }
    }
}

#[derive(Clone)]
pub struct DateField {
    value: Signal<String>,
    label: String,
    touched: Signal<bool>,
    editing: Signal<bool>,
    style: Option<FormStyle>,
}

impl DateField {
    pub fn new(value: Signal<String>, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
            touched: Signal::new(false),
            editing: Signal::new(false),
            style: None,
        }
    }

    pub fn style(mut self, style: FormStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn error(&self) -> Option<String> {
        match parse_iso_date(&self.value.get_untracked()) {
            Some(_) => None,
            None => Some("data w formacie RRRR-MM-DD".to_string()),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.error().is_none()
    }

    pub fn touch(&self) {
        self.touched.set(true);
    }

    pub fn value_signal(&self) -> Signal<String> {
        self.value.clone()
    }

    pub fn editing_signal(&self) -> Signal<bool> {
        self.editing.clone()
    }

    fn shift_days(&self, delta: i64) {
        let cur = self.value.get_untracked();
        let shifted = parse_iso_date(&cur)
            .map(|d| days_to_iso(date_to_days(d) + delta))
            .unwrap_or_else(today_iso_fallback);
        self.value.set(shifted);
        self.touch();
    }
}

impl ValidField for DateField {
    fn is_valid(&self) -> bool {
        DateField::is_valid(self)
    }

    fn touch(&self) {
        DateField::touch(self)
    }

    fn error_text(&self) -> Option<String> {
        if self.touched.get_untracked() {
            self.error()
        } else {
            None
        }
    }

    fn track(&self) {
        let _ = self.value.get();
        let _ = self.touched.get();
    }
}

impl View for DateField {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let f = self.clone();
        let _ = f.value.get();
        let touched = f.touched.get();
        let error = if touched { f.error() } else { None };
        let editing = f.editing.clone();
        let editing_end = f.editing.clone();
        let touched_end = f.touched.clone();
        let editing_submit = f.editing.clone();
        let touched_submit = f.touched.clone();
        let input = TextField::new(f.value.clone())
            .placeholder("RRRR-MM-DD")
            .max_length(10)
            .on_edit_start(move || editing.set(true))
            .on_edit_end(move |_| {
                editing_end.set(false);
                touched_end.set(true);
            })
            .on_submit(move |_| {
                editing_submit.set(false);
                touched_submit.set(true);
            });
        let prev = f.clone();
        let next = f.clone();
        let mut col = Column::new()
            .gap(4.0)
            .child(
                Text::new(f.label.clone())
                    .size(label_size_of(f.style))
                    .bold(),
            )
            .child(
                Row::new()
                    .gap(4.0)
                    .child(styled_button(
                        Button::new("<").on_click(move || prev.shift_days(-1)),
                        f.style,
                    ))
                    .child_flex(input, 1.0)
                    .child(styled_button(
                        Button::new(">").on_click(move || next.shift_days(1)),
                        f.style,
                    )),
            );
        if let Some(msg) = error {
            col = col.child(
                Text::new(msg)
                    .size(error_size_of(f.style))
                    .color(error_color_of(f.style)),
            );
        }
        Some(vec![AnyView::new(col)])
    }

    fn layout_style(&self) -> sequoia_layout::LayoutStyle {
        sequoia_layout::LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Fit,
            ..sequoia_layout::LayoutStyle::default()
        }
    }
}

type IsoDate = (i32, u32, u32);

pub fn parse_iso_date(s: &str) -> Option<IsoDate> {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let y: i32 = s[0..4].parse().ok()?;
    let m: u32 = s[5..7].parse().ok()?;
    let d: u32 = s[8..10].parse().ok()?;
    if !(1..=12).contains(&m) {
        return None;
    }
    if d < 1 || d > days_in_month(y, m) {
        return None;
    }
    Some((y, m, d))
}

fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(y) => 29,
        2 => 28,
        _ => 0,
    }
}

fn date_to_days((y, m, d): IsoDate) -> i64 {
    let y = if m <= 2 { y as i64 - 1 } else { y as i64 };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (m as i64 + 9).rem_euclid(12);
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn days_to_iso(z: i64) -> String {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    format!("{:04}-{:02}-{:02}", if m <= 2 { y + 1 } else { y }, m, d)
}

fn today_iso_fallback() -> String {
    "1970-01-01".to_string()
}

#[derive(Clone)]
pub struct MultiSelect {
    options: Vec<String>,
    selected: Signal<Vec<usize>>,
    label: String,
    min_selected: usize,
    touched: Signal<bool>,
    style: Option<FormStyle>,
}

impl MultiSelect {
    pub fn new(options: Vec<String>, label: impl Into<String>) -> Self {
        Self {
            options,
            selected: Signal::new(Vec::new()),
            label: label.into(),
            min_selected: 0,
            touched: Signal::new(false),
            style: None,
        }
    }

    pub fn style(mut self, style: FormStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn min_selected(mut self, n: usize) -> Self {
        self.min_selected = n;
        self
    }

    pub fn toggle(&self, index: usize) {
        if index >= self.options.len() {
            return;
        }
        self.selected.update(|sel| {
            if let Some(pos) = sel.iter().position(|i| *i == index) {
                sel.remove(pos);
            } else {
                sel.push(index);
                sel.sort_unstable();
            }
        });
        self.touch();
    }

    pub fn selected_indices(&self) -> Vec<usize> {
        self.selected.get_untracked()
    }

    pub fn selected_values(&self) -> Vec<String> {
        let sel = self.selected.get_untracked();
        sel.into_iter()
            .filter_map(|i| self.options.get(i).cloned())
            .collect()
    }

    pub fn summary_text(&self) -> String {
        let values = self.selected_values();
        if values.is_empty() {
            "—".to_string()
        } else if values.len() <= 2 {
            values.join(", ")
        } else {
            format!("{} wybrane", values.len())
        }
    }

    pub fn error(&self) -> Option<String> {
        if self.selected.get_untracked().len() < self.min_selected {
            Some(format!("wybierz co najmniej {}", self.min_selected))
        } else {
            None
        }
    }

    pub fn is_valid(&self) -> bool {
        self.error().is_none()
    }

    pub fn touch(&self) {
        self.touched.set(true);
    }

    pub fn selected_signal(&self) -> Signal<Vec<usize>> {
        self.selected.clone()
    }
}

impl ValidField for MultiSelect {
    fn is_valid(&self) -> bool {
        MultiSelect::is_valid(self)
    }

    fn touch(&self) {
        MultiSelect::touch(self)
    }

    fn error_text(&self) -> Option<String> {
        if self.touched.get_untracked() {
            self.error()
        } else {
            None
        }
    }

    fn track(&self) {
        let _ = self.selected.get();
        let _ = self.touched.get();
    }
}

impl View for MultiSelect {
    fn body(&self) -> impl View + 'static {
        Unit
    }

    fn children_override(&self) -> Option<Vec<AnyView>> {
        let me = self.clone();
        let _ = me.selected.get();
        let touched = me.touched.get();
        let error = if touched { me.error() } else { None };
        let summary = me.summary_text();
        let mut menu = FloatingMenu::wrap(styled_button(Button::new(summary), me.style))
            .below_invoker()
            .open_on_click()
            .popup_width(220.0);
        for (i, opt) in me.options.iter().enumerate() {
            let on = me.selected.get_untracked().contains(&i);
            let toggler = me.clone();
            let mark = if on { "✓ " } else { "○ " };
            menu = menu.row("", format!("{mark}{opt}"), move || toggler.toggle(i));
        }
        let mut col = Column::new()
            .gap(4.0)
            .child(Text::new(me.label.clone()).size(12.0).bold())
            .child(menu);
        if let Some(msg) = error {
            col = col.child(
                Text::new(msg)
                    .size(error_size_of(me.style))
                    .color(error_color_of(me.style)),
            );
        }
        Some(vec![AnyView::new(col)])
    }

    fn layout_style(&self) -> sequoia_layout::LayoutStyle {
        sequoia_layout::LayoutStyle {
            width: Length::Fill(1.0),
            height: Length::Fit,
            ..sequoia_layout::LayoutStyle::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_until_touched_and_filled() {
        let form = Form::new();
        let v = Signal::new(String::new());
        let f = Field::new(v.clone(), "Nazwa")
            .rule("cannot be empty", |s: &str| !s.trim().is_empty())
            .rule("min. 3 znaki", |s: &str| s.chars().count() >= 3);
        form.register(&f);
        assert!(!form.is_valid());
        assert_eq!(f.error().as_deref(), Some("cannot be empty"));
        v.set("ab".to_string());
        assert_eq!(f.error().as_deref(), Some("min. 3 znaki"));
        v.set("abc".to_string());
        assert!(form.is_valid());
        assert!(f.error().is_none());
    }

    #[test]
    fn number_clamps_and_validates() {
        let v = Signal::new(5.0f32);
        let n = NumberField::new(v.clone(), "Quantity")
            .min(0.0)
            .max(10.0)
            .step(2.0)
            .rule("max 8", |x| x <= 8.0);
        n.nudge(1.0);
        assert_eq!(v.get_untracked(), 7.0);
        n.nudge(1.0);
        assert_eq!(v.get_untracked(), 9.0, "step 2 od 7");
        assert!(!n.is_valid());
        assert_eq!(n.error().as_deref(), Some("max 8"));
        v.set(4.0);
        assert!(n.is_valid());
    }

    #[test]
    fn iso_date_rejects_bad_calendar_dates() {
        assert!(parse_iso_date("2026-02-29").is_none(), "2026 unleap");
        assert!(parse_iso_date("2024-02-29").is_some(), "2024 Leap");
        assert!(parse_iso_date("2026-13-01").is_none());
        assert!(parse_iso_date("2026-04-31").is_none());
        assert!(parse_iso_date("26-04-01").is_none());
        assert!(parse_iso_date("2026/04/01").is_none());

        let d = parse_iso_date("2026-03-01").unwrap();
        assert_eq!(days_to_iso(date_to_days(d) - 1), "2026-02-28");
        assert_eq!(days_to_iso(date_to_days(d) + 365), "2027-03-01");
        let f = DateField::new(Signal::new("2026-02-30".to_string()), "D");
        assert!(!f.is_valid());
        f.shift_days(0);
        assert_eq!(f.value_signal().get_untracked(), "1970-01-01");
        let g = DateField::new(Signal::new("2026-03-01".to_string()), "D");
        g.shift_days(-1);
        assert_eq!(g.value_signal().get_untracked(), "2026-02-28");
        assert!(g.is_valid());
    }

    #[test]
    fn multi_select_toggles_sorted_and_enforces_min() {
        let m = MultiSelect::new(vec!["a".into(), "b".into(), "c".into()], "Tagi").min_selected(1);
        assert!(!m.is_valid());
        m.toggle(2);
        m.toggle(0);
        assert_eq!(m.selected_indices(), vec![0, 2], "posortowane");
        assert_eq!(m.selected_values(), vec!["a".to_string(), "c".to_string()]);
        assert!(m.is_valid());
        m.toggle(0);
        m.toggle(2);
        assert!(m.selected_indices().is_empty());
        assert!(!m.is_valid());
        m.toggle(99);
        assert!(m.selected_indices().is_empty());

        let form = Form::new();
        form.register(&m);
        assert_eq!(form.len(), 1);
        assert!(!form.is_valid());
    }
}
