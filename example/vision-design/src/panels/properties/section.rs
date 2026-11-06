use crate::theme::colors;
use sequoia_ui::prelude::*;

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
            .child(
                Container::new()
                    .fill_width()
                    .padding(Edges::xy(12.0, 8.0))
                    .on_click(move || open_cb.update(|b| *b = !*b))
                    .hover(Color::hexa(0xFFFFFF08))
                    .child(
                        Row::new()
                            .gap(6.0)
                            .align_items(Align::Center)
                            .child(
                                Text::new(chev)
                                    .size(10.0)
                                    .color(colors::active().icon_inactive),
                            )
                            .child(
                                Text::new(self.title)
                                    .size(11.0)
                                    .semibold()
                                    .font("Inter")
                                    .color(Color::hex(0xA8A8AC)),
                            ),
                    ),
            )
            .child(when(
                self.open.clone(),
                Container::new()
                    .padding(Edges::xy(12.0, 10.0))
                    .child(self.body.clone()),
            ))
    }
}

pub(super) fn section(
    title: &'static str,
    open: Signal<bool>,
    body: impl View + 'static,
) -> AnyView {
    AnyView::new(Section {
        title,
        open,
        body: AnyView::new(body),
    })
}
