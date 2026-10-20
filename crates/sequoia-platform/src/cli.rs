use std::collections::HashMap;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CliArgs {
    pub positionals: Vec<String>,

    pub options: HashMap<String, Option<String>>,
}

impl CliArgs {
    pub fn parse() -> Self {
        Self::parse_from(std::env::args().skip(1))
    }

    pub fn parse_from(args: impl IntoIterator<Item = String>) -> Self {
        let mut out = Self::default();
        let mut pending_key: Option<String> = None;
        for arg in args {
            if let Some(key) = pending_key.take() {
                if !arg.starts_with('-') {
                    out.options.insert(key, Some(arg));
                    continue;
                }
                out.options.insert(key, None);
            }
            if let Some(rest) = arg.strip_prefix("--") {
                if let Some((k, v)) = rest.split_once('=') {
                    out.options.insert(k.to_string(), Some(v.to_string()));
                } else {
                    pending_key = Some(rest.to_string());
                }
            } else if let Some(rest) = arg.strip_prefix('-') {
                if rest.len() > 1 {
                    for c in rest.chars() {
                        out.options.insert(c.to_string(), None);
                    }
                } else if !rest.is_empty() {
                    pending_key = Some(rest.to_string());
                } else {
                    out.positionals.push(arg);
                }
            } else {
                out.positionals.push(arg);
            }
        }
        if let Some(key) = pending_key {
            out.options.insert(key, None);
        }
        out
    }

    pub fn has(&self, key: &str) -> bool {
        self.options.contains_key(key)
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.options.get(key).and_then(|v| v.as_deref())
    }

    pub fn positionals(&self) -> &[String] {
        &self.positionals
    }

    pub fn first_file(&self) -> Option<&str> {
        self.positionals.first().map(|s| s.as_str())
    }
}

pub fn help_text(program: &str, options: &[(&str, &str)]) -> String {
    let mut out = format!("Usage: {program} [options] [files...]\n\nOptions:\n");
    for (name, desc) in options {
        out.push_str(&format!("  {name:<22} {desc}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> CliArgs {
        CliArgs::parse_from(v.iter().map(|s| s.to_string()))
    }

    #[test]
    fn files_flags_and_values() {
        let a = args(&[
            "model.obj",
            "--fullscreen",
            "--theme",
            "light",
            "--jobs=4",
            "-abc",
        ]);
        assert_eq!(a.positionals(), &["model.obj".to_string()]);
        assert!(a.has("fullscreen"));
        assert_eq!(a.get("theme"), Some("light"));
        assert_eq!(a.get("jobs"), Some("4"));
        assert!(a.has("a") && a.has("b") && a.has("c"));
        assert_eq!(a.first_file(), Some("model.obj"));
    }

    #[test]
    fn dangling_key_is_flag() {
        let a = args(&["--verbose"]);
        assert!(a.has("verbose"));
        assert_eq!(a.get("verbose"), None);
    }

    #[test]
    fn lone_dash_is_positional() {
        let a = args(&["-"]);
        assert_eq!(a.positionals(), &["-".to_string()]);
    }

    #[test]
    fn help_lists_options() {
        let h = help_text("shaper", &[("--fullscreen", "full screen")]);
        assert!(h.contains("shaper") && h.contains("--fullscreen"));
    }
}
