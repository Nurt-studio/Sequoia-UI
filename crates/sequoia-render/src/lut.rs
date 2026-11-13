//! Parser for Adobe/IRIDAS `.cube` 3D LUTs.
//!
//! Only the 3D flavour is supported. The parsed table is stored as RGBA floats
//! so the renderer can upload it as a `Rgba16Float` `texture_3d`; sampling it
//! with a linear, clamp-to-edge sampler reproduces the trilinear interpolation
//! used by photo editors.

/// Largest accepted LUT edge. 33³ is the common size; 128³ already needs 16 MiB
/// as RGBA16F and anything past that is almost certainly not a colour table.
pub const MAX_LUT_SIZE: u32 = 128;

/// A parsed 3D LUT.
#[derive(Clone, Debug)]
pub struct Lut3D {
    /// Samples per axis.
    pub size: u32,
    /// `size³` RGBA texels, red varying fastest.
    pub data: Vec<f32>,
}

/// Reads and parses a `.cube` file.
pub fn load_cube(path: &str) -> Result<Lut3D, String> {
    let source = std::fs::read_to_string(path).map_err(|e| format!("cannot read `{path}`: {e}"))?;
    parse_cube(&source)
}

/// Parses the text of a `.cube` file.
pub fn parse_cube(source: &str) -> Result<Lut3D, String> {
    let mut size: Option<u32> = None;
    let mut domain_min = [0.0f32; 3];
    let mut domain_max = [1.0f32; 3];
    let mut rows: Vec<[f32; 3]> = Vec::new();

    for (index, raw) in source.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(head) = parts.next() else {
            continue;
        };
        match head {
            "TITLE" => {}
            "LUT_3D_SIZE" => {
                let value = parts
                    .next()
                    .ok_or_else(|| format!("line {}: LUT_3D_SIZE needs a number", index + 1))?;
                let parsed = value
                    .parse::<u32>()
                    .map_err(|_| format!("line {}: `{value}` is not a LUT size", index + 1))?;
                if parsed < 2 || parsed > MAX_LUT_SIZE {
                    return Err(format!(
                        "line {}: LUT size {parsed} is outside 2..={MAX_LUT_SIZE}",
                        index + 1
                    ));
                }
                if let Some(existing) = size
                    && existing != parsed
                {
                    return Err(format!(
                        "line {}: conflicting LUT_3D_SIZE {parsed} (already {existing})",
                        index + 1
                    ));
                }
                size = Some(parsed);
            }
            "LUT_1D_SIZE" | "LUT_2D_SIZE" => {
                return Err(format!(
                    "line {}: only 3D LUTs are supported, found `{head}`",
                    index + 1
                ));
            }
            "DOMAIN_MIN" => {
                domain_min = parse_triplet(head, &mut parts, index + 1)?;
            }
            "DOMAIN_MAX" => {
                domain_max = parse_triplet(head, &mut parts, index + 1)?;
            }
            _ => {
                let mut rgb = [0.0f32; 3];
                rgb[0] = parse_float(head, index + 1)?;
                for value in rgb.iter_mut().skip(1) {
                    let part = parts.next().ok_or_else(|| {
                        format!("line {}: expected three numbers per LUT row", index + 1)
                    })?;
                    *value = parse_float(part, index + 1)?;
                }
                if parts.next().is_some() {
                    return Err(format!(
                        "line {}: expected exactly three numbers per LUT row",
                        index + 1
                    ));
                }
                rows.push(rgb);
            }
        }
    }

    if domain_min != [0.0, 0.0, 0.0] || domain_max != [1.0, 1.0, 1.0] {
        return Err(format!(
            "unsupported input domain {:?}..{:?}; only 0..1 LUTs are supported",
            domain_min, domain_max
        ));
    }
    let size = size.ok_or_else(|| "missing LUT_3D_SIZE".to_string())?;
    let expected = (size as usize).pow(3);
    if rows.len() != expected {
        return Err(format!(
            "expected {expected} LUT rows for size {size}, found {}",
            rows.len()
        ));
    }

    let mut data = Vec::with_capacity(expected * 4);
    for rgb in rows {
        data.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 1.0]);
    }
    Ok(Lut3D { size, data })
}

fn parse_float(value: &str, line: usize) -> Result<f32, String> {
    let parsed = value
        .parse::<f32>()
        .map_err(|_| format!("line {line}: `{value}` is not a number"))?;
    if !parsed.is_finite() {
        return Err(format!("line {line}: `{value}` is not finite"));
    }
    Ok(parsed)
}

fn parse_triplet<'a>(
    head: &str,
    parts: &mut impl Iterator<Item = &'a str>,
    line: usize,
) -> Result<[f32; 3], String> {
    let mut out = [0.0f32; 3];
    for value in &mut out {
        let part = parts
            .next()
            .ok_or_else(|| format!("line {line}: {head} needs three numbers"))?;
        *value = parse_float(part, line)?;
    }
    if parts.next().is_some() {
        return Err(format!(
            "line {line}: {head} takes exactly three numbers"
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity_cube(size: u32) -> String {
        let mut out = format!("TITLE \"identity\"\n# comment\nLUT_3D_SIZE {size}\n");
        let last = size - 1;
        for b in 0..size {
            for g in 0..size {
                for r in 0..size {
                    out.push_str(&format!(
                        "{} {} {}\n",
                        r as f32 / last as f32,
                        g as f32 / last as f32,
                        b as f32 / last as f32
                    ));
                }
            }
        }
        out
    }

    #[test]
    fn parses_a_3d_lut_in_red_major_order() {
        let lut = parse_cube(&identity_cube(4)).expect("valid LUT");
        assert_eq!(lut.size, 4);
        assert_eq!(lut.data.len(), 4 * 4 * 4 * 4);
        assert_eq!(&lut.data[0..4], &[0.0, 0.0, 0.0, 1.0]);
        // Red varies fastest, then green, then blue.
        assert_eq!(&lut.data[4..8], &[1.0 / 3.0, 0.0, 0.0, 1.0]);
        assert_eq!(
            &lut.data[lut.data.len() - 4..],
            &[1.0, 1.0, 1.0, 1.0]
        );
    }

    #[test]
    fn rejects_1d_luts_and_non_default_domains() {
        let error = parse_cube("LUT_1D_SIZE 16\n").expect_err("1D must be rejected");
        assert!(error.contains("3D"), "{error}");

        let error = parse_cube("DOMAIN_MIN 0 0 0\nDOMAIN_MAX 0 0 1\nLUT_3D_SIZE 2\n")
            .expect_err("non-default domain must be rejected");
        assert!(error.contains("domain"), "{error}");
    }

    #[test]
    fn rejects_wrong_row_counts_and_sizes() {
        let error = parse_cube("LUT_3D_SIZE 2\n0 0 0\n").expect_err("missing rows");
        assert!(error.contains("expected 8 LUT rows"), "{error}");

        let error = parse_cube("LUT_3D_SIZE 1\n").expect_err("too small");
        assert!(error.contains("outside 2..="), "{error}");

        let error = parse_cube(&identity_cube(2).replace("0 0 0\n", "0 0\n"))
            .expect_err("short row");
        assert!(error.contains("three numbers"), "{error}");
    }
}
