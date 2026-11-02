use crate::core::layer::Layer;

/// Serializes visible layers into a simple SVG document.
pub(crate) fn to_svg(layers: &[Layer]) -> String {
    let mut svg =
        String::from("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 800 600\">\n");
    for l in layers.iter().filter(|l| l.visible) {
        let op = l.opacity / 100.0;
        match l.kind {
            "RECTANGLE" | "FRAME" => {
                svg.push_str(&format!(
                    "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\" fill-opacity=\"{}\" />\n",
                    l.x, l.y, l.w, l.h, l.radius, l.fill_hex, op
                ));
            }
            "ELLIPSE" => {
                svg.push_str(&format!(
                    "  <ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" fill-opacity=\"{}\" />\n",
                    l.x + l.w / 2.0,
                    l.y + l.h / 2.0,
                    l.w / 2.0,
                    l.h / 2.0,
                    l.fill_hex,
                    op
                ));
            }
            "TEXT" => {
                svg.push_str(&format!(
                    "  <text x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"{}\" font-family=\"sans-serif\" fill-opacity=\"{}\">{}</text>\n",
                    l.x,
                    l.y + l.font_size,
                    l.font_size,
                    l.fill_hex,
                    op,
                    l.text
                ));
            }
            _ => {}
        }
    }
    svg.push_str("</svg>\n");
    svg
}