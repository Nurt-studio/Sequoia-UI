use sequoia_core::{BrushStop, Color, Fill, Rect};
use sequoia_export::{ExportOptions, render_to_image};
use sequoia_view::{Canvas, Column, Painter, View};

fn gray_stops() -> Vec<BrushStop> {
    (0..=6)
        .map(|i| {
            let v = (i as f32 / 6.0 * 255.0).round() as u8;
            BrushStop::new(i as f32 / 6.0, Color::rgb8(v, v, v))
        })
        .collect()
}

fn bar_view(pad: f32) -> impl View {
    let stops = gray_stops();
    Column::new()
        .padding_xy(pad, pad)
        .child(
            Canvas::new()
                .width(14.0)
                .height(198.0)
                .on_draw(move |g: &mut Painter| {
                    g.fill(
                        Rect::from_xywh(0.0, 0.0, 14.0, 198.0),
                        Fill::vertical(stops.clone()),
                    );
                }),
        )
}

fn center_luma(img: &image::RgbaImage, x: u32, y0: u32, y1: u32) -> Vec<f32> {
    (y0..y1)
        .map(|y| {
            let p = img.get_pixel(x, y).0;
            0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32
        })
        .collect()
}

fn max_notch(luma: &[f32]) -> f32 {
    luma.windows(3)
        .map(|w| (2.0 * w[1] - w[0] - w[2]).abs())
        .fold(0.0, f32::max)
}

#[test]
fn hue_strip_has_no_seams_on_fractional_origin() {
    for pad in [0.0f32, 10.5] {
        let opts = ExportOptions::new(120.0, 260.0).scale(1.0);
        let img = render_to_image(bar_view(pad), &opts).unwrap();

        let cx = (pad + 7.0).floor() as u32;
        let y0 = (pad + 2.0).ceil() as u32;
        let y1 = (pad + 196.0).floor() as u32;
        let luma = center_luma(&img, cx, y0, y1);
        let range = luma.iter().fold(0.0f32, |a, &b| a.max(b))
            - luma.iter().fold(255.0f32, |a, &b| a.min(b));
        assert!(
            range > 200.0,
            "pad {pad}: pasek nie wyrenderowany? zakres luminancji {range:.1}"
        );

        let notch = max_notch(&luma);
        if notch >= 12.0 {
            let mut spikes: Vec<(usize, f32)> = luma
                .windows(3)
                .enumerate()
                .map(|(i, w)| (i + 1, (2.0 * w[1] - w[0] - w[2]).abs()))
                .collect();
            spikes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            for (i, s) in spikes.iter().take(5) {
                let lo = i.saturating_sub(2);
                let hi = (i + 3).min(luma.len());
                eprintln!(
                    "pad {pad} spike row {i} ({s:.1}): {luma:?}",
                    luma = &luma[lo..hi]
                );
            }
        }
        assert!(notch < 4.0, "pad {pad}: szew gradientu (kolec {notch:.1})");
    }
}

#[test]
fn two_stop_gradient_is_exported() {
    let stops = vec![
        BrushStop::new(0.0, Color::BLACK),
        BrushStop::new(1.0, Color::WHITE),
    ];
    let view = Column::new().child(Canvas::new().width(200.0).height(8.0).on_draw(
        move |g: &mut Painter| {
            g.fill(
                Rect::from_xywh(0.0, 0.0, 200.0, 8.0),
                Fill::horizontal(stops.clone()),
            );
        },
    ));
    let opts = ExportOptions::new(220.0, 28.0).scale(1.0);
    let img = render_to_image(view, &opts).unwrap();
    let luma = |x: u32| {
        let p = img.get_pixel(x, 4).0;
        0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32
    };

    assert!(luma(0) < 15.0, "lewy koniec czarny, jest {}", luma(0));
    assert!(luma(199) > 245.0, "right end white, is {}", luma(199));
    let mid = luma(100);
    assert!(
        (170.0..=205.0).contains(&mid),
        "center of wedge ~188, is {mid:.1}"
    );
}
