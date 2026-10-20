use sequoia_core::Color;

use crate::Theme;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokensError {
    pub message: String,
}

impl TokensError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for TokensError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "tokens: {}", self.message)
    }
}

impl std::error::Error for TokensError {}

impl Theme {
    pub fn from_tokens_str(src: &str) -> Result<Self, TokensError> {
        let v: serde_json::Value =
            serde_json::from_str(src).map_err(|e| TokensError::new(format!("bad JSON: {e}")))?;
        let mut theme = Self::dark();
        if let Some(color) = v.get("color") {
            let obj = color
                .as_object()
                .ok_or_else(|| TokensError::new("'color' to obiekt { nazwa: hex }"))?;
            for (key, val) in obj {
                let hex = val.as_str().ok_or_else(|| {
                    TokensError::new(format!("color.{key} is hex in quotes, not {val}"))
                })?;
                let c = Color::parse_hex(hex).ok_or_else(|| {
                    TokensError::new(format!("color.{key} to hex #RRGGBB[AA], nie '{hex}'"))
                })?;
                match key.as_str() {
                    "bg" | "background" => theme.background = c,
                    "surface" => theme.surface = c,
                    "border" => theme.border = c,
                    "ink" | "text" => theme.text = c,
                    "muted" | "text_secondary" => theme.text_secondary = c,
                    "accent" => theme.accent = c,
                    "accent_hover" => theme.accent_hover = c,
                    "accent_pressed" => theme.accent_pressed = c,
                    "disabled" => theme.disabled = c,
                    _ => {}
                }
            }
        }
        if let Some(radius) = section_num(&v, "radius", &["md", "sm", "lg"])? {
            theme.radius = radius;
        }
        if let Some(size) = section_num(&v, "font", &["md", "sm", "lg"])? {
            theme.font_size = size;
        }
        Ok(theme)
    }

    pub fn from_tokens_file(path: &str) -> Result<Self, TokensError> {
        let src = std::fs::read_to_string(path)
            .map_err(|e| TokensError::new(format!("unable to read '{path}': {e}")))?;
        Self::from_tokens_str(&src)
    }
}

fn section_num(
    v: &serde_json::Value,
    section: &str,
    keys: &[&str],
) -> Result<Option<f32>, TokensError> {
    let Some(obj) = v.get(section) else {
        return Ok(None);
    };
    let obj = obj
        .as_object()
        .ok_or_else(|| TokensError::new(format!("'{section}' to obiekt {{ nazwa: liczba }}")))?;
    for key in keys {
        if let Some(n) = obj.get(*key) {
            let n = n
                .as_f64()
                .ok_or_else(|| TokensError::new(format!("{section}.{key} to liczba, nie {n}")))?;
            let n = n as f32;
            if n.is_finite() {
                return Ok(Some(n));
            }
            return Err(TokensError::new(format!(
                "{section}.{key} must be finished"
            )));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL: &str = r##"{
        "color": {
            "bg": "#141417", "surface": "#33363D", "surface2": "#232529",
            "border": "#3C404A", "ink": "#E6E6E6", "muted": "#9A9AA0",
            "accent": "#356FD1", "danger": "#B3261E", "ok": "#6AA84F", "warn": "#F0A63A"
        },
        "radius": { "sm": 6, "md": 8, "lg": 12 },
        "gap": { "sm": 8 },
        "font": { "sm": 12, "md": 14, "lg": 20 }
    }"##;

    #[test]
    fn full_palette_maps_known_keys() {
        let t = Theme::from_tokens_str(FULL).unwrap();
        assert_eq!(t.background, Color::parse_hex("#141417").unwrap());
        assert_eq!(t.surface, Color::parse_hex("#33363D").unwrap());
        assert_eq!(t.text, Color::parse_hex("#E6E6E6").unwrap());
        assert_eq!(t.text_secondary, Color::parse_hex("#9A9AA0").unwrap());
        assert_eq!(t.accent, Color::parse_hex("#356FD1").unwrap());
        assert_eq!(t.radius, 8.0);
        assert_eq!(t.font_size, 14.0);

        assert_eq!(t.accent_hover, Theme::dark().accent_hover);
    }

    #[test]
    fn missing_keys_fall_back_to_dark() {
        let t = Theme::from_tokens_str(r##"{"color": {"bg": "#000000"}}"##).unwrap();
        let d = Theme::dark();
        assert_eq!(t.background, Color::parse_hex("#000000").unwrap());
        assert_eq!(t.surface, d.surface);
        assert_eq!(t.radius, d.radius);
        assert_eq!(t.font_size, d.font_size);
    }

    #[test]
    fn empty_object_is_dark() {
        assert_eq!(Theme::from_tokens_str("{}").unwrap(), Theme::dark());
    }

    #[test]
    fn bad_hex_names_the_key() {
        let err = Theme::from_tokens_str(r#"{"color": {"bg": "nie-kolor"}}"#).unwrap_err();
        assert!(err.message.contains("color.bg"), "{err}");
    }

    #[test]
    fn bad_json_is_rejected() {
        assert!(Theme::from_tokens_str("{nie json").is_err());
    }

    #[test]
    fn missing_file_names_the_path() {
        let err = Theme::from_tokens_file("nie/ma/takiego.tokens.json").unwrap_err();
        assert!(err.message.contains("nie/ma/takiego.tokens.json"), "{err}");
    }
}
