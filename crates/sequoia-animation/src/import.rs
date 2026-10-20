use std::collections::HashMap;

use sequoia_core::Color;

use crate::Easing;
use crate::animation::{Animation, Interpolation, Keyframe, Track};
use crate::bezier::CubicBezier;
use crate::value::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct ImportError(pub String);

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "animation import: {}", self.0)
    }
}

impl std::error::Error for ImportError {}

fn err(msg: impl Into<String>) -> ImportError {
    ImportError(msg.into())
}

fn get_f64(v: &serde_json::Value) -> Option<f64> {
    v.as_f64()
}

fn parse_color(v: &serde_json::Value) -> Result<Color, ImportError> {
    if let Some(s) = v.as_str() {
        return Color::parse_hex(s).ok_or_else(|| err(format!("bad color {s:?}")));
    }
    if let Some(arr) = v.as_array() {
        let n: Vec<f32> = arr
            .iter()
            .filter_map(|x| x.as_f64().map(|f| f as f32))
            .collect();
        if n.len() == 4 {
            return Ok(Color::rgba(n[0], n[1], n[2], n[3]));
        }
        if n.len() == 3 {
            return Ok(Color::rgb(n[0], n[1], n[2]));
        }
    }
    Err(err(format!("bad color {v}")))
}

fn parse_value(v: &serde_json::Value) -> Result<Value, ImportError> {
    if let Some(n) = v.as_f64() {
        return Ok(Value::f32(n as f32));
    }
    let o = v.as_object().ok_or_else(|| err(format!("bad value {v}")))?;
    if let Some(n) = o.get("px").and_then(get_f64) {
        return Ok(Value::px(n as f32));
    }
    if let Some(n) = o.get("f32").and_then(get_f64) {
        return Ok(Value::f32(n as f32));
    }
    if let Some(c) = o.get("color") {
        return Ok(Value::color(parse_color(c)?));
    }
    if let Some(p) = o.get("point").and_then(|x| x.as_array())
        && p.len() == 2
    {
        let x = p[0].as_f64().ok_or_else(|| err("bad point.x"))? as f32;
        let y = p[1].as_f64().ok_or_else(|| err("bad point.y"))? as f32;
        return Ok(Value::point(x, y));
    }
    if let Some(s) = o.get("size").and_then(|x| x.as_array())
        && s.len() == 2
    {
        let w = s[0].as_f64().ok_or_else(|| err("bad size.w"))? as f32;
        let h = s[1].as_f64().ok_or_else(|| err("bad size.h"))? as f32;
        return Ok(Value::size(w, h));
    }
    Err(err(format!("bad value {v}")))
}

fn named_easing(name: &str) -> Option<Easing> {
    let n: String = name
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase();
    Some(match n.as_str() {
        "linear" => Easing::Linear,
        "easein" => Easing::EaseIn,
        "easeout" => Easing::EaseOut,
        "easeinout" => Easing::EaseInOut,
        "easeinquad" => Easing::EaseInQuad,
        "easeoutquad" => Easing::EaseOutQuad,
        "easeinoutquad" => Easing::EaseInOutQuad,
        "easeincubic" => Easing::EaseInCubic,
        "easeoutcubic" => Easing::EaseOutCubic,
        "easeinoutcubic" => Easing::EaseInOutCubic,
        "easeinquart" => Easing::EaseInQuart,
        "easeoutquart" => Easing::EaseOutQuart,
        "easeinoutquart" => Easing::EaseInOutQuart,
        "easeinquint" => Easing::EaseInQuint,
        "easeoutquint" => Easing::EaseOutQuint,
        "easeinoutquint" => Easing::EaseInOutQuint,
        "easeinexpo" => Easing::EaseInExpo,
        "easeoutexpo" => Easing::EaseOutExpo,
        "easeinoutexpo" => Easing::EaseInOutExpo,
        "easeincirc" => Easing::EaseInCirc,
        "easeoutcirc" => Easing::EaseOutCirc,
        "easeinoutcirc" => Easing::EaseInOutCirc,
        "easeinsine" => Easing::EaseInSine,
        "easeoutsine" => Easing::EaseOutSine,
        "easeinoutsine" => Easing::EaseInOutSine,
        "easeinback" => Easing::EaseInBack,
        "easeoutback" => Easing::EaseOutBack,
        "easeinoutback" => Easing::EaseInOutBack,
        "bouncein" => Easing::BounceIn,
        "bounceout" => Easing::BounceOut,
        "elasticout" => Easing::ElasticOut,
        _ => return None,
    })
}

fn parse_interpolation(v: Option<&serde_json::Value>) -> Result<Interpolation, ImportError> {
    let Some(v) = v else {
        return Ok(Interpolation::Linear);
    };
    if let Some(s) = v.as_str() {
        let low = s.to_lowercase();
        if low == "hold" {
            return Ok(Interpolation::Hold);
        }
        if low == "linear" {
            return Ok(Interpolation::Linear);
        }
        if let Some(e) = named_easing(s) {
            return Ok(Interpolation::from_easing(e));
        }
        return Err(err(format!("bad interp {s:?}")));
    }
    if let Some(o) = v.as_object()
        && let Some(b) = o.get("bezier").and_then(|x| x.as_array())
    {
        if b.len() == 4 {
            let f: Vec<f32> = b
                .iter()
                .filter_map(|x| x.as_f64().map(|f| f as f32))
                .collect();
            if f.len() == 4 {
                return Ok(Interpolation::Bezier(CubicBezier::new(
                    f[0], f[1], f[2], f[3],
                )));
            }
        }
        return Err(err(format!("bad bezier {v}")));
    }
    Err(err(format!("bad interp {v}")))
}

fn parse_keyframe(v: &serde_json::Value) -> Result<Keyframe, ImportError> {
    let o = v
        .as_object()
        .ok_or_else(|| err(format!("bad keyframe {v}")))?;
    let t = o
        .get("t")
        .and_then(get_f64)
        .ok_or_else(|| err(format!("keyframe without t: {v}")))?;
    let value = o
        .get("v")
        .ok_or_else(|| err(format!("keyframe without v: {v}")))?;
    let interp = parse_interpolation(o.get("ease").or_else(|| o.get("interp")))?;
    Ok(Keyframe::new(t, parse_value(value)?).with_interpolation(interp))
}

fn parse_track(v: &serde_json::Value) -> Result<Track, ImportError> {
    let arr = v.as_array().ok_or_else(|| err(format!("bad track {v}")))?;
    let mut kfs = Vec::with_capacity(arr.len());
    for k in arr {
        kfs.push(parse_keyframe(k)?);
    }
    Track::try_new(kfs).ok_or_else(|| err(format!("invalid track {v}")))
}

pub fn animation_from_json(name: &str, v: &serde_json::Value) -> Result<Animation, ImportError> {
    let o = v
        .as_object()
        .ok_or_else(|| err("animation must be an object"))?;
    let duration = o
        .get("duration")
        .and_then(get_f64)
        .ok_or_else(|| err("animation without duration"))?;
    if !duration.is_finite() || duration < 0.0 {
        return Err(err("bad duration"));
    }
    let tracks = o
        .get("tracks")
        .and_then(|t| t.as_object())
        .ok_or_else(|| err("animation without tracks"))?;
    let mut anim = Animation::new(name, duration);
    for (track_name, track_v) in tracks {
        anim = anim.with_track(track_name, parse_track(track_v)?);
    }
    Ok(anim)
}

pub fn bundle_from_json(json: &str) -> Result<HashMap<String, Animation>, ImportError> {
    let root: serde_json::Value =
        serde_json::from_str(json).map_err(|e| err(format!("bad json: {e}")))?;
    let map = root
        .get("animations")
        .unwrap_or(&root)
        .as_object()
        .ok_or_else(|| err("bundle must be an object"))?;
    let mut out = HashMap::new();
    for (name, v) in map {
        out.insert(name.clone(), animation_from_json(name, v)?);
    }
    if out.is_empty() {
        return Err(err("empty bundle"));
    }
    Ok(out)
}

pub fn import_lottie(json: &str) -> Result<HashMap<String, Animation>, ImportError> {
    let root: serde_json::Value =
        serde_json::from_str(json).map_err(|e| err(format!("bad lottie json: {e}")))?;
    let fr = root.get("fr").and_then(get_f64).unwrap_or(60.0).max(1.0);
    let layers = root
        .get("layers")
        .and_then(|l| l.as_array())
        .ok_or_else(|| err("lottie without layers"))?;
    let layer = layers
        .iter()
        .find(|l| l.get("ks").is_some())
        .ok_or_else(|| err("lottie without ks"))?;
    let ks = &layer["ks"];

    fn lottie_track(prop: Option<&serde_json::Value>, fr: f64, scale: f32) -> Option<(Track, f64)> {
        let prop = prop?;
        let k = prop.get("k")?;

        if let Some(n) = k.as_f64() {
            let t = Track::f32(&[(0.0, (n as f32 * scale))])?;
            return Some((t, 0.0));
        }
        if let Some(arr) = k.as_array() {
            if arr.iter().all(|x| x.as_f64().is_some()) && arr.len() <= 3 {
                let n = arr[0].as_f64()? as f32 * scale;
                let t = Track::f32(&[(0.0, n)])?;
                return Some((t, 0.0));
            }

            let mut pts: Vec<(f64, f32)> = Vec::new();
            for kf in arr {
                let t = kf.get("t")?.as_f64()? / fr;
                let s = kf.get("s")?.as_array()?;
                let v = s.first()?.as_f64()? as f32 * scale;
                pts.push((t, v));
            }
            if pts.is_empty() {
                return None;
            }
            let max_t = pts.iter().map(|&(t, _)| t).fold(0.0f64, f64::max);

            if let Some(&(lt, lv)) = pts.last() {
                pts.push((lt + 1.0 / fr, lv));
            }
            let track = Track::f32(&pts.iter().map(|&(t, v)| (t, v)).collect::<Vec<_>>())?;
            return Some((track, max_t + 1.0 / fr));
        }
        None
    }

    fn lottie_pos(ks: &serde_json::Value, fr: f64, axis: usize) -> Option<(Track, f64)> {
        let p = ks.get("p")?;
        let k = p.get("k")?;
        if let Some(arr) = k.as_array() {
            if arr.iter().all(|x| x.as_f64().is_some()) && arr.len() >= 2 {
                let v = arr.get(axis)?.as_f64()? as f32;
                let pts = vec![(0.0f64, v)];
                let t = Track::px(&pts)?;
                return Some((t, 0.0));
            }
            let mut pts: Vec<(f64, f32)> = Vec::new();
            for kf in arr {
                let t = kf.get("t")?.as_f64()? / fr;
                let s = kf.get("s")?.as_array()?;
                let v = s.get(axis)?.as_f64()? as f32;
                pts.push((t, v));
            }
            if pts.is_empty() {
                return None;
            }
            let max_t = pts.iter().map(|&(t, _)| t).fold(0.0f64, f64::max);
            if let Some(&(lt, lv)) = pts.last() {
                pts.push((lt + 1.0 / fr, lv));
            }
            let track = Track::px(&pts.iter().map(|&(t, v)| (t, v)).collect::<Vec<_>>())?;
            return Some((track, max_t + 1.0 / fr));
        }
        None
    }

    let mut tracks: Vec<(&str, Track)> = Vec::new();
    let mut max_t = 0.5f64;
    if let Some((t, e)) = lottie_track(ks.get("o"), fr, 0.01) {
        max_t = max_t.max(e);
        tracks.push(("opacity", t));
    }
    if let Some((t, e)) = lottie_pos(ks, fr, 0) {
        max_t = max_t.max(e);
        tracks.push(("x", t));
    }
    if let Some((t, e)) = lottie_pos(ks, fr, 1) {
        max_t = max_t.max(e);
        tracks.push(("y", t));
    }
    if tracks.is_empty() {
        return Err(err("lottie without animatable ks.o/ks.p"));
    }
    let mut anim = Animation::new("lottie", max_t.max(0.05));
    for (name, t) in tracks {
        anim = anim.with_track(name, t);
    }
    let mut out = HashMap::new();
    out.insert("lottie".to_string(), anim);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUNDLE: &str = r#"{
        "animations": {
            "fade": {
                "duration": 0.4,
                "tracks": {
                    "opacity": [
                        {"t": 0.0, "v": 0.0},
                        {"t": 0.4, "v": 1.0, "ease": "easeOut"}
                    ],
                    "x": [
                        {"t": 0.0, "v": {"px": -8.0}, "interp": "hold"},
                        {"t": 0.4, "v": {"px": 0.0}}
                    ]
                }
            }
        }
    }"#;

    #[test]
    fn bundle_parses_tracks() {
        let b = bundle_from_json(BUNDLE).expect("bundle");
        let a = &b["fade"];
        let f0 = a.sample(0.0);
        assert_eq!(f0.get("opacity").and_then(|v| v.as_f32()), Some(0.0));
        let f1 = a.sample(1.0);
        assert_eq!(f1.get("opacity").and_then(|v| v.as_f32()), Some(1.0));
        assert_eq!(f1.get("x").and_then(|v| v.as_px()), Some(0.0));
    }

    #[test]
    fn bezier_and_color_values() {
        let json = r##"{"animations": {"c": {"duration": 1.0, "tracks": {
            "background": [
                {"t": 0.0, "v": {"color": "#ff0000"}},
                {"t": 1.0, "v": {"color": [0.0, 0.0, 1.0, 1.0]}, "ease": {"bezier": [0.25, 0.1, 0.25, 1.0]}}
            ]}}}}"##;
        let b = bundle_from_json(json).expect("bundle");
        let f = b["c"].sample(1.0);
        let c = f.get("background").and_then(|v| v.as_color()).unwrap();
        assert!((c.b - 1.0).abs() < 0.01, "{c:?}");
    }

    #[test]
    fn rejects_garbage() {
        assert!(bundle_from_json("{}").is_err());
        assert!(bundle_from_json("not json").is_err());
        assert!(bundle_from_json(r#"{"animations":{"x":{"duration":-1.0,"tracks":{}}}}"#).is_err());
    }

    #[test]
    fn lottie_subset_maps_opacity_and_pos() {
        let lottie = r#"{
            "fr": 60.0,
            "layers": [{"ks": {
                "o": {"k": [{"t": 0.0, "s": [0.0]}, {"t": 30.0, "s": [100.0]}]},
                "p": {"k": [10.0, 20.0]}
            }}]
        }"#;
        let b = import_lottie(lottie).expect("lottie");
        let a = &b["lottie"];
        let f0 = a.sample(0.0);
        assert_eq!(f0.get("opacity").and_then(|v| v.as_f32()), Some(0.0));
        assert_eq!(
            a.sample(10.0).get("opacity").and_then(|v| v.as_f32()),
            Some(1.0)
        );
        assert_eq!(a.sample(0.0).get("x").and_then(|v| v.as_px()), Some(10.0));
    }
}
