use sequoia_core::Pixels;

pub const MAX_PHOTO_DIM: u32 = 8192;

pub const MAX_PHOTO_BYTES: usize = 64 << 20;

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PhotoError {
    Io(String),

    TooLarge { width: u32, height: u32 },

    Decode(String),
}

impl std::fmt::Display for PhotoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PhotoError::Io(path) => write!(f, "cannot read photo `{path}`"),
            PhotoError::TooLarge { width, height } => write!(
                f,
                "photo past the limit ({width}x{height}, max {MAX_PHOTO_DIM}px / {} MiB)",
                MAX_PHOTO_BYTES / (1 << 20)
            ),
            PhotoError::Decode(why) => write!(f, "cannot decode photo: {why}"),
        }
    }
}

impl std::error::Error for PhotoError {}

pub fn load_pixels(path: &str) -> Result<Pixels, PhotoError> {
    if let Ok((w, h)) = image::image_dimensions(path)
        && !dims_allowed(w, h)
    {
        log::warn!(
            "sequoia-assets: photo {path} past the limit ({w}x{h}, max {MAX_PHOTO_DIM}px / {} MiB) — refused",
            MAX_PHOTO_BYTES / (1 << 20)
        );
        return Err(PhotoError::TooLarge {
            width: w,
            height: h,
        });
    }
    let img = image::open(path)
        .map_err(|_| PhotoError::Io(path.to_string()))?
        .to_rgba8();
    let (w, h) = img.dimensions();
    if !dims_allowed(w, h) {
        log::warn!(
            "sequoia-assets: photo {path} after decoding past the limit ({w}x{h}) — refused"
        );
        return Err(PhotoError::TooLarge {
            width: w,
            height: h,
        });
    }
    Pixels::from_rgba(w, h, img.into_raw()).ok_or_else(|| PhotoError::Decode("bad buffer".into()))
}

pub fn decode_pixels(bytes: &[u8]) -> Result<Pixels, PhotoError> {
    precheck_bytes_dims(bytes)?;
    let img = image::load_from_memory(bytes)
        .map_err(|e| PhotoError::Decode(e.to_string()))?
        .to_rgba8();
    let (w, h) = img.dimensions();
    if !dims_allowed(w, h) {
        log::warn!("sequoia-assets: decoded image past the limit ({w}x{h}) — refused");
        return Err(PhotoError::TooLarge {
            width: w,
            height: h,
        });
    }
    Pixels::from_rgba(w, h, img.into_raw()).ok_or_else(|| PhotoError::Decode("bad buffer".into()))
}

pub fn load_pixels_thumbnail(path: &str, max_dim: u32) -> Result<Pixels, PhotoError> {
    let img = open_checked(path)?;
    to_thumbnail(img, max_dim)
}

pub fn decode_pixels_thumbnail(bytes: &[u8], max_dim: u32) -> Result<Pixels, PhotoError> {
    precheck_bytes_dims(bytes)?;
    let img = image::load_from_memory(bytes)
        .map_err(|e| PhotoError::Decode(e.to_string()))?
        .to_rgba8();
    let (w, h) = img.dimensions();
    if !dims_allowed(w, h) {
        log::warn!("sequoia-assets: decoded image past the limit ({w}x{h}) — refused");
        return Err(PhotoError::TooLarge {
            width: w,
            height: h,
        });
    }
    to_thumbnail(img, max_dim)
}

fn precheck_bytes_dims(bytes: &[u8]) -> Result<(), PhotoError> {
    let dims = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()
        .and_then(|r| r.into_dimensions().ok());
    if let Some((w, h)) = dims
        && !dims_allowed(w, h)
    {
        log::warn!(
            "sequoia-assets: image bytes past the limit ({w}x{h}, max {MAX_PHOTO_DIM}px / {} MiB) — refused",
            MAX_PHOTO_BYTES / (1 << 20)
        );
        return Err(PhotoError::TooLarge {
            width: w,
            height: h,
        });
    }
    Ok(())
}

fn open_checked(path: &str) -> Result<image::RgbaImage, PhotoError> {
    if let Ok((w, h)) = image::image_dimensions(path)
        && !dims_allowed(w, h)
    {
        log::warn!("sequoia-assets: photo {path} past the limit ({w}x{h}) — refused");
        return Err(PhotoError::TooLarge {
            width: w,
            height: h,
        });
    }
    let img = image::open(path)
        .map_err(|_| PhotoError::Io(path.to_string()))?
        .to_rgba8();
    let (w, h) = img.dimensions();
    if !dims_allowed(w, h) {
        log::warn!(
            "sequoia-assets: photo {path} after decoding past the limit ({w}x{h}) — refused"
        );
        return Err(PhotoError::TooLarge {
            width: w,
            height: h,
        });
    }
    Ok(img)
}

fn to_thumbnail(img: image::RgbaImage, max_dim: u32) -> Result<Pixels, PhotoError> {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return Err(PhotoError::Decode("empty image".into()));
    }
    let img = if max_dim > 0 && w.max(h) > max_dim {
        let scale = max_dim as f32 / w.max(h) as f32;
        let tw = ((w as f32 * scale).round() as u32).max(1);
        let th = ((h as f32 * scale).round() as u32).max(1);
        image::imageops::thumbnail(&img, tw, th)
    } else {
        img
    };
    let (w, h) = img.dimensions();
    Pixels::from_rgba(w, h, img.into_raw()).ok_or_else(|| PhotoError::Decode("bad buffer".into()))
}

fn dims_allowed(width: u32, height: u32) -> bool {
    if width == 0 || height == 0 || width > MAX_PHOTO_DIM || height > MAX_PHOTO_DIM {
        return false;
    }
    match (width as usize)
        .checked_mul(height as usize)
        .and_then(|px| px.checked_mul(4))
    {
        Some(bytes) => bytes <= MAX_PHOTO_BYTES,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_png() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("sequoia-pixels");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("photo.png");
        let pixels: Vec<u8> = (0..8 * 4).flat_map(|_| [10u8, 20, 30, 255]).collect();
        image::save_buffer(&path, &pixels, 8, 4, image::ColorType::Rgba8).unwrap();
        path
    }

    #[test]
    fn load_roundtrips_dimensions_and_color() {
        let path = temp_png();
        let px = load_pixels(path.to_str().unwrap()).expect("loads png");
        assert_eq!((px.width(), px.height()), (8, 4));
        let c = px.get(0, 0).unwrap();
        assert_eq!(c.to_rgb8(), (10, 20, 30));
        assert_eq!(px.get(0, 0).unwrap().a, 1.0);
    }

    #[test]
    fn thumbnail_downscales_keeping_aspect() {
        let path = temp_png();
        let px = load_pixels_thumbnail(path.to_str().unwrap(), 4).expect("loads");
        assert_eq!((px.width(), px.height()), (4, 2));
        let px = load_pixels_thumbnail(path.to_str().unwrap(), 64).expect("loads");
        assert_eq!((px.width(), px.height()), (8, 4), "small image untouched");
    }

    #[test]
    fn decode_from_memory_works() {
        let path = temp_png();
        let bytes = std::fs::read(&path).unwrap();
        let px = decode_pixels(&bytes).expect("decodes");
        assert_eq!((px.width(), px.height()), (8, 4));
        let thumb = decode_pixels_thumbnail(&bytes, 4).unwrap();
        assert_eq!((thumb.width(), thumb.height()), (4, 2));
    }

    #[test]
    fn decode_from_memory_rejects_bomb_before_alloc() {
        fn crc32(data: &[u8]) -> u32 {
            let mut crc = 0xFFFF_FFFFu32;
            for &b in data {
                crc ^= u32::from(b);
                for _ in 0..8 {
                    let mask = (crc & 1).wrapping_neg();
                    crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
                }
            }
            !crc
        }
        use image::ImageEncoder as _;
        let pixels = vec![0u8; 8 * 4 * 4];
        let mut bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut bytes)
            .write_image(&pixels, 8, 4, image::ExtendedColorType::Rgba8)
            .unwrap();
        bytes[16..20].copy_from_slice(&100_000u32.to_be_bytes());
        bytes[20..24].copy_from_slice(&100_000u32.to_be_bytes());
        let crc = crc32(&bytes[12..29]);
        bytes[29..33].copy_from_slice(&crc.to_be_bytes());
        assert!(matches!(
            decode_pixels(&bytes),
            Err(PhotoError::TooLarge { .. })
        ));
        assert!(matches!(
            decode_pixels_thumbnail(&bytes, 64),
            Err(PhotoError::TooLarge { .. })
        ));
    }

    #[test]
    fn missing_or_garbage_is_err() {
        assert!(load_pixels("no/such/photo.png").is_err());
        assert!(decode_pixels(b"not an image").is_err());
    }

    #[test]
    fn dims_guard_rejects_huge_and_empty() {
        assert!(!dims_allowed(0, 10));
        assert!(!dims_allowed(MAX_PHOTO_DIM + 1, 1));
        assert!(!dims_allowed(MAX_PHOTO_DIM, MAX_PHOTO_DIM), "bytes cap");
        assert!(dims_allowed(64, 64));
    }
}
