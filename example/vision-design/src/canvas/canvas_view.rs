//! Canvas view composition; gesture logic lives in `interaction`, drawing in `paint`.

use std::collections::HashSet;

use sequoia_ui::prelude::*;

use crate::canvas::interaction::CanvasSignals;
use crate::canvas::paint::{
    draw_ghost, draw_grid_screen, draw_guides, draw_layer, draw_marquee, draw_selection,
};
use crate::state::EditorState;
use crate::theme::colors;

pub(crate) fn canvas(state: &EditorState) -> impl View + 'static {
    let signals = CanvasSignals::from_state(state);
    // Gesture sessions live in the editor state: rebuilding the view tree must not
    // drop an ongoing drag or allocate a new state machine for
    // every frame.
    let interaction = state.interaction.clone();

    let track_sig = signals.clone();
    let draw_sig = signals.clone();
    let draw_inter = interaction.clone();
    let ptr_sig = signals;
    let ptr_inter = interaction;

    Canvas::new()
        .background(colors::active().bg_darker)
        .track(move || {
            track_sig.layers.get();
            track_sig.selected_layers.get();
            track_sig.tool.get();
            track_sig.repaint.get();
            track_sig.grid.get();
            track_sig.zoom.get();
            track_sig.pan_x.get();
            track_sig.pan_y.get();
        })
        .on_draw(move |p| {
            let size = p.size();
            if (draw_sig.viewport_w.get_untracked() - size.width).abs() > 0.5 {
                draw_sig.viewport_w.set(size.width);
            }
            if (draw_sig.viewport_h.get_untracked() - size.height).abs() > 0.5 {
                draw_sig.viewport_h.set(size.height);
            }

            let zoom = draw_sig.zoom.get_untracked();
            let pan_x = draw_sig.pan_x.get_untracked();
            let pan_y = draw_sig.pan_y.get_untracked();

            if draw_sig.grid.get_untracked() {
                draw_grid_screen(p, pan_x, pan_y, zoom);
            }

            let wx0 = pan_x - 100.0;
            let wy0 = pan_y - 100.0;
            let wx1 = pan_x + size.width / zoom + 100.0;
            let wy1 = pan_y + size.height / zoom + 100.0;

            let transform = Transform::translate(-pan_x, -pan_y).then(Transform::uniform(zoom));

            p.with_transform(transform, |p| {
                draw_sig.layers.with_untracked(|layers| {
                    for l in layers {
                        draw_layer(p, l);
                    }

                    if let Some(session) = draw_inter.draw.borrow().as_ref() {
                        let x = session.start_wx.min(session.cur_wx);
                        let y = session.start_wy.min(session.cur_wy);
                        let w = (session.cur_wx - session.start_wx).abs();
                        let h = (session.cur_wy - session.start_wy).abs();
                        draw_ghost(p, session.tool, x, y, w, h);
                    }

                    if let Some(session) = draw_inter.marquee.borrow().as_ref() {
                        let x = session.start_wx.min(session.cur_wx);
                        let y = session.start_wy.min(session.cur_wy);
                        let w = (session.cur_wx - session.start_wx).abs();
                        let h = (session.cur_wy - session.start_wy).abs();
                        draw_marquee(p, x, y, w, h);
                    }

                    // One item keeps its resize handles; a marquee selection
                    // outlines every item so large selections stay readable.
                    let selected_ids = draw_sig.selected_layers.get_untracked();
                    if let [id] = selected_ids.as_slice() {
                        if let Some(l) = layers.iter().find(|l| l.id == *id) {
                            draw_selection(p, l, zoom, true);
                        }
                    } else if !selected_ids.is_empty() {
                        let selected: HashSet<u64> = selected_ids.iter().copied().collect();
                        for l in layers.iter().filter(|l| selected.contains(&l.id)) {
                            draw_selection(p, l, zoom, false);
                        }
                    }

                    let guides = draw_inter.guides.borrow();
                    if !guides.v.is_empty() || !guides.h.is_empty() {
                        draw_guides(p, &guides, wx0, wx1, wy0, wy1);
                    }
                });
            });
        })
        .on_pointer(move |pos, ev, ctx| ptr_inter.pointer(&ptr_sig, pos, ev, ctx))
}
