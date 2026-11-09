//! Layer list with filtering (List + LayerRow).
use sequoia_ui::prelude::*;

use crate::core::layer::Layer;
use crate::state::EditorState;

use super::layer_row::LayerRow;

/// The rows of one section after every filter: section, search query and the
/// "hide hidden layers" toggle (eye in the panel header). Shared by the list
/// itself and by the drag & drop row math, so indexes always agree.
pub(super) fn filtered_layers(state: &EditorState, only_components: bool) -> Vec<Layer> {
    let q = state.layer_query.get().to_lowercase();
    let show_hidden = state.show_hidden_layers.get();
    state
        .layers
        .get()
        .into_iter()
        .filter(|l| l.component == only_components)
        .filter(|l| l.visible || show_hidden)
        .filter(|l| q.is_empty() || l.name.to_lowercase().contains(&q))
        .collect()
}

pub(super) fn layer_list(state: &EditorState, only_components: bool) -> impl View + 'static {
    let st = state.clone();
    VirtualList::new({
        let st = st.clone();
        move || filtered_layers(&st, only_components)
    })
    .key(|l: &Layer| l.id)
    .item_height(28.0)
    .item({
        let st = st.clone();
        move |l: &Layer| LayerRow {
            layer: l.clone(),
            state: st.clone(),
            only_components,
        }
    })
}
