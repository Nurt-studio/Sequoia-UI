use sequoia_ui::prelude::*;

use crate::state::{EditorState, ToastEntry, ToastKind};
use crate::theme::colors;

fn toast_item(t: ToastEntry, state: &EditorState) -> impl View + 'static {
    let dot = match t.kind {
        ToastKind::Info => colors::active().ok,
        ToastKind::Warn => colors::active().warn,
        ToastKind::Error => colors::active().danger,
    };
    let st = state.clone();
    let id = t.id;
    Container::new()
        .padding(Edges::xy(12.0, 7.0))
        .radius(7.0)
        .background(Color::hex(0x2C2C2E))
        .border(1.0, colors::active().stroke)
        .on_click(move || st.dismiss_toast(id))
        .child(
            Row::new()
                .gap(8.0)
                .align_items(Align::Center)
                .child(
                    Container::new()
                        .width(7.0)
                        .height(7.0)
                        .radius(3.5)
                        .background(dot),
                )
                .child(Text::new(t.msg).size(11.0).color(Color::hex(0xF2F2F2))),
        )
        .shadow(ShadowSpec::new(Color::hexa(0x00000080), 30.0).offset(0.0, 10.0))
}

#[derive(Clone)]
struct ToastsView {
    state: EditorState,
}

impl View for ToastsView {
    fn body(&self) -> impl View + 'static {
        let toasts = self.state.toasts.get();
        let vw = self.state.viewport_w.get();
        let vh = self.state.viewport_h.get();
        let st = self.state.clone();
        let open: Signal<bool> = Signal::new(true);
        let items: Vec<AnyView> = toasts
            .into_iter()
            .map(|t| AnyView::new(toast_item(t, &st)))
            .collect();
        Popper::new(open)
            .anchor_rect(Rect::from_xywh(vw / 2.0, vh - 8.0, 0.0, 0.0))
            .placement(Placement::Top)
            .offset(0.0, 8.0)
            .child(
                Column::new()
                    .gap(6.0)
                    .align_items(Align::Center)
                    .children(items),
            )
    }
}

// -------------------------- inline text editor --------------------------

pub(crate) fn toasts(state: &EditorState) -> impl View + 'static {
    ToastsView {
        state: state.clone(),
    }
}
