use crate::core::layer::Layer;

pub(crate) fn apply_auto_layout_frame(frame_id: u64, ls: &mut [Layer]) {
    let Some(fi) = ls.iter().position(|l| l.id == frame_id) else {
        return;
    };
    let frame = ls[fi].clone();
    if frame.kind != "FRAME" || frame.layout == "NONE" {
        return;
    }

    let pad_l = frame.pad_l.max(0.0);
    let pad_t = frame.pad_t.max(0.0);
    let pad_r = frame.pad_r.max(0.0);
    let pad_b = frame.pad_b.max(0.0);
    let gap = frame.gap.max(0.0);
    let inner_w = (frame.w - pad_l - pad_r).max(0.0);
    let inner_h = (frame.h - pad_t - pad_b).max(0.0);

    let child_ids: Vec<u64> = ls
        .iter()
        .filter(|l| l.parent == Some(frame_id) && l.visible)
        .map(|l| l.id)
        .collect();
    if child_ids.is_empty() {
        return;
    }

    if frame.layout == "H" {
        let mut cx = frame.x + pad_l;
        for cid in child_ids {
            let Some(ci) = ls.iter().position(|l| l.id == cid) else {
                continue;
            };
            let c = &mut ls[ci];
            if c.position_mode == "ABSOLUTE" {
                continue; // keeps own x/y, doesn't take space
            }
            let cy = match frame.align_v {
                "center" => frame.y + pad_t + (inner_h - c.h) * 0.5,
                "end" => frame.y + pad_t + inner_h - c.h,
                _ => frame.y + pad_t,
            };
            c.x = cx;
            c.y = cy;
            cx += c.w + gap;
        }
    } else if frame.layout == "V" {
        let mut cy = frame.y + pad_t;
        for cid in child_ids {
            let Some(ci) = ls.iter().position(|l| l.id == cid) else {
                continue;
            };
            let c = &mut ls[ci];
            if c.position_mode == "ABSOLUTE" {
                continue;
            }
            let cx = match frame.align_h {
                "center" => frame.x + pad_l + (inner_w - c.w) * 0.5,
                "end" => frame.x + pad_l + inner_w - c.w,
                _ => frame.x + pad_l,
            };
            c.x = cx;
            c.y = cy;
            cy += c.h + gap;
        }
    } else if frame.layout == "G" {
        let mut cx = frame.x + pad_l;
        let mut cy = frame.y + pad_t;
        let mut row_h: f32 = 0.0;
        for cid in child_ids {
            let Some(ci) = ls.iter().position(|l| l.id == cid) else {
                continue;
            };
            let c = &mut ls[ci];
            if c.position_mode == "ABSOLUTE" {
                continue;
            }
            if cx > frame.x + pad_l && cx + c.w > frame.x + pad_l + inner_w {
                cx = frame.x + pad_l;
                cy += row_h + gap;
                row_h = 0.0;
            }
            c.x = cx;
            c.y = cy;
            row_h = row_h.max(c.h);
            cx += c.w + gap;
        }
    } else if frame.layout == "Z" {
        // ZStack: each child is free (keeps own x/y) unless pins are set on it.
        for cid in child_ids {
            let Some(ci) = ls.iter().position(|l| l.id == cid) else {
                continue;
            };
            let c = &mut ls[ci];
            if c.position_mode == "ABSOLUTE" {
                continue;
            }
            // Horizontal
            match (c.pin_left, c.pin_right) {
                (true, true) => {
                    c.x = frame.x + pad_l + c.self_pad_left;
                    c.w = (inner_w - c.self_pad_left - c.self_pad_right).max(4.0);
                }
                (true, false) => {
                    c.x = frame.x + pad_l + c.self_pad_left;
                }
                (false, true) => {
                    c.x = frame.x + frame.w - pad_r - c.self_pad_right - c.w;
                }
                (false, false) => {}
            }
            // Vertical
            match (c.pin_top, c.pin_bottom) {
                (true, true) => {
                    c.y = frame.y + pad_t + c.self_pad_top;
                    c.h = (inner_h - c.self_pad_top - c.self_pad_bottom).max(4.0);
                }
                (true, false) => {
                    c.y = frame.y + pad_t + c.self_pad_top;
                }
                (false, true) => {
                    c.y = frame.y + frame.h - pad_b - c.self_pad_bottom - c.h;
                }
                (false, false) => {}
            }
        }
    }
}

