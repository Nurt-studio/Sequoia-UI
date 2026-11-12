//! Motion JPEG AVI encoding for continuous exports.
//!
//! The writer builds a plain RIFF AVI with one JPEG frame per animation frame.
//! No external encoder is required, which keeps headless export self-contained.

use std::path::Path;

use image::RgbaImage;

use crate::ExportError;

const DEFAULT_JPEG_QUALITY: u8 = 88;
const AVIF_HASINDEX: u32 = 0x10;
const AVIIF_KEYFRAME: u32 = 0x10;

/// Writes `frames` as a Motion JPEG AVI at `fps`.
pub(crate) fn write_mjpeg_avi(
    frames: &[RgbaImage],
    fps: u32,
    path: &Path,
) -> Result<(), ExportError> {
    let Some(first) = frames.first() else {
        return Err(ExportError::Io("video needs at least one frame".into()));
    };
    let (width, height) = (first.width(), first.height());
    if frames
        .iter()
        .any(|frame| frame.width() != width || frame.height() != height)
    {
        return Err(ExportError::Io(
            "video frames must all have the same size".into(),
        ));
    }

    let fps = fps.max(1);
    let mut jpegs = Vec::with_capacity(frames.len());
    for frame in frames {
        jpegs.push(encode_frame(frame)?);
    }
    let max_frame = jpegs.iter().map(Vec::len).max().unwrap_or(0) as u32;
    let max_bytes_per_sec =
        ((max_frame as u64).saturating_mul(fps as u64)).min(u64::from(u32::MAX)) as u32;
    let micros_per_frame = 1_000_000u32 / fps;

    let mut avih = Vec::with_capacity(56);
    push_u32(&mut avih, micros_per_frame);
    push_u32(&mut avih, max_bytes_per_sec);
    push_u32(&mut avih, 0);
    push_u32(&mut avih, AVIF_HASINDEX);
    push_u32(&mut avih, jpegs.len() as u32);
    push_u32(&mut avih, 0);
    push_u32(&mut avih, 1);
    push_u32(&mut avih, max_frame);
    push_u32(&mut avih, width);
    push_u32(&mut avih, height);
    for _ in 0..4 {
        push_u32(&mut avih, 0);
    }

    let mut strh = Vec::with_capacity(56);
    strh.extend_from_slice(b"vids");
    strh.extend_from_slice(b"MJPG");
    push_u32(&mut strh, 0);
    push_u16(&mut strh, 0);
    push_u16(&mut strh, 0);
    push_u32(&mut strh, 0);
    push_u32(&mut strh, 1);
    push_u32(&mut strh, fps);
    push_u32(&mut strh, 0);
    push_u32(&mut strh, jpegs.len() as u32);
    push_u32(&mut strh, max_frame);
    push_u32(&mut strh, u32::MAX);
    push_u32(&mut strh, 0);
    push_u16(&mut strh, 0);
    push_u16(&mut strh, 0);
    push_u16(&mut strh, width.min(u32::from(u16::MAX)) as u16);
    push_u16(&mut strh, height.min(u32::from(u16::MAX)) as u16);

    let mut strf = Vec::with_capacity(40);
    push_u32(&mut strf, 40);
    push_u32(&mut strf, width);
    push_u32(&mut strf, height);
    push_u16(&mut strf, 1);
    push_u16(&mut strf, 24);
    strf.extend_from_slice(b"MJPG");
    push_u32(&mut strf, width.saturating_mul(height).saturating_mul(3));
    for _ in 0..4 {
        push_u32(&mut strf, 0);
    }

    let strl = list(
        b"strl",
        &[chunk(b"strh", &strh), chunk(b"strf", &strf)].concat(),
    );
    let hdrl = list(b"hdrl", &[chunk(b"avih", &avih), strl].concat());

    let mut movi_payload = Vec::new();
    let mut index = Vec::with_capacity(jpegs.len() * 16);
    for jpeg in &jpegs {
        // Index offsets count from the `movi` fourcc that starts the list body.
        let offset = 4 + movi_payload.len() as u32;
        write_chunk(&mut movi_payload, b"00dc", jpeg);
        index.extend_from_slice(b"00dc");
        push_u32(&mut index, AVIIF_KEYFRAME);
        push_u32(&mut index, offset);
        push_u32(&mut index, jpeg.len() as u32);
    }
    let movi = list(b"movi", &movi_payload);

    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    push_u32(&mut out, 0);
    out.extend_from_slice(b"AVI ");
    out.extend_from_slice(&hdrl);
    out.extend_from_slice(&movi);
    out.extend_from_slice(&chunk(b"idx1", &index));

    let riff_size = (out.len() - 8) as u32;
    out[4..8].copy_from_slice(&riff_size.to_le_bytes());
    std::fs::write(path, &out)
        .map_err(|e| ExportError::Io(format!("write video {}: {e}", path.display())))
}

fn encode_frame(frame: &RgbaImage) -> Result<Vec<u8>, ExportError> {
    let (width, height) = (frame.width(), frame.height());
    let mut rgb = Vec::with_capacity(width as usize * height as usize * 3);
    for pixel in frame.pixels() {
        let alpha = f32::from(pixel[3]) / 255.0;
        rgb.push((f32::from(pixel[0]) * alpha).round() as u8);
        rgb.push((f32::from(pixel[1]) * alpha).round() as u8);
        rgb.push((f32::from(pixel[2]) * alpha).round() as u8);
    }
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, DEFAULT_JPEG_QUALITY)
        .encode(&rgb, width, height, image::ExtendedColorType::Rgb8)
        .map_err(|e| ExportError::Io(format!("encode mjpeg frame: {e}")))?;
    Ok(out)
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_chunk(out: &mut Vec<u8>, id: &[u8; 4], payload: &[u8]) {
    out.extend_from_slice(id);
    push_u32(out, payload.len() as u32);
    out.extend_from_slice(payload);
    if payload.len() % 2 == 1 {
        out.push(0);
    }
}

fn chunk(id: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 9);
    write_chunk(&mut out, id, payload);
    out
}

fn list(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(payload.len() + 4);
    body.extend_from_slice(kind);
    body.extend_from_slice(payload);
    let mut out = Vec::with_capacity(body.len() + 8);
    out.extend_from_slice(b"LIST");
    push_u32(&mut out, body.len() as u32);
    out.extend_from_slice(&body);
    out
}
