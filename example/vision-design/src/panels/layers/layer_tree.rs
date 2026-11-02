//! Layer list with filtering (List + LayerRow).
use sequoia_ui::prelude::*;

use crate::core::layer::Layer;
use crate::state::EditorState;

use super::layer_row::LayerRow;

pub(super) fn layer_list(state: &EditorState, only_components: bool) -> impl View + 'static {
    let layers = state.layers.clone();
    let query = state.layer_query.clone();
    let st = state.clone();
    List::new({
        let layers = layers.clone();
        let query = query.clone();
        move || {
            let q = query.get().to_lowercase();
            let all = layers.get();
            all.into_iter()
                .filter(|l| l.component == only_components)
                .filter(|l| q.is_empty() || l.name.to_lowercase().contains(&q))
                .collect::<Vec<_>>()
        }
    })
    .key(|l: &Layer| l.id)
    .item({
        let st = st.clone();
        move |l: &Layer| LayerRow {
            layer: l.clone(),
            state: st.clone(),
        }
    })
}