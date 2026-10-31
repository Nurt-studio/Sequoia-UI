use sequoia_ui::prelude::*;

use crate::theme::colors;

#[derive(Clone)]
struct Section {
    title: &'static str,
    open: Signal<bool>,
    body: AnyView,
}

impl View for Section {
    fn body(&self) -> impl View + 'static {
        let open_now = self.open.get();
        let open_cb = self.open.clone();
        let chev = if open_now { "▾" } else { "▸" };
        Column::new()
            .border_bottom(1.0, colors::active().stroke_soft)
            .padding(Edges::xy(0.0, 4.0))
            .child(
                Container::new()
                    .fill_width()
                    .padding(Edges::xy(12.0, 6.0))
                    .on_click(move || open_cb.update(|b| *b = !*b))
                    .child(
                        Row::new()
                            .gap(8.0)
                            .align_items(Align::Center)
                            .child(Text::new(chev).size(14.0).color(colors::active().icon_inactive))
                            .child(
                                Text::new(self.title)
                                    .size(13.0)
                                    .semibold()
                                    .color(Color::hex(0xC9C9CD)),
                            ),
                    ),
            )
            .child(when(
                self.open.clone(),
                Container::new()
                    .padding(Edges::xy(12.0, 4.0))
                    .child(self.body.clone()),
            ))
    }
}

pub(super) fn section(title: &'static str, open: Signal<bool>, body: impl View + 'static) -> AnyView {
    AnyView::new(Section {
        title,
        open,
        body: AnyView::new(body),
    })
}

// ---------------------------- alignment ----------------------------

