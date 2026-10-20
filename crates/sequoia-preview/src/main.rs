#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, SystemTime};

use sequoia_platform::App;
use sequoia_seq::SeqLoader;

const CHILD_FLAG: &str = "--__child";

const WATCH_POLL: Duration = Duration::from_millis(250);

struct Config {
    file: PathBuf,
    watch: bool,
    is_child: bool,
}

fn parse_args(argv: &[String]) -> Result<Config, String> {
    let mut file = None;
    let mut watch = false;
    let mut is_child = false;
    for a in argv {
        match a.as_str() {
            "--watch" => watch = true,
            CHILD_FLAG => is_child = true,
            s if s.starts_with("--") => return Err(format!("nieznana flaga '{s}'")),
            _ => {
                if file.is_some() {
                    return Err("podaj jeden plik .seq".to_string());
                }
                file = Some(PathBuf::from(a));
            }
        }
    }
    match file {
        Some(f) => Ok(Config {
            file: f,
            watch,
            is_child,
        }),
        None => Err("usage: sequoia-preview FILE.seq [--watch]".to_string()),
    }
}

fn display_name(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn run_once(path: &Path) -> i32 {
    let file = display_name(path);
    let loaded = match SeqLoader::load_file(&file) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    let (w, h) = loaded.meta.size;
    let result = App::new()
        .title(loaded.meta.title)
        .size(w, h)
        .theme(loaded.meta.theme)
        .root(loaded.view)
        .run();
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{file}: runtime error: {e}");
            1
        }
    }
}

fn mtime_of(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn supervise(path: &Path) -> i32 {
    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("cannot find binary: {e}");
            return 1;
        }
    };
    loop {
        let mut child = match spawn_child(&exe, path) {
            Some(c) => c,
            None => return 1,
        };
        let deps = deps_of(path);
        let mut stamps = stamps(&deps);
        let (code, changed) = watch_child(&mut child, &deps, &mut stamps);

        if !changed {
            return code;
        }
    }
}

fn deps_of(path: &Path) -> Vec<PathBuf> {
    match SeqLoader::load_file(&display_name(path)) {
        Ok(loaded) if !loaded.files.is_empty() => loaded.files,
        _ => vec![path.to_path_buf()],
    }
}

fn stamps(deps: &[PathBuf]) -> Vec<Option<SystemTime>> {
    deps.iter().map(|p| mtime_of(p)).collect()
}

fn spawn_child(exe: &Path, path: &Path) -> Option<Child> {
    match Command::new(exe).arg(path).arg(CHILD_FLAG).spawn() {
        Ok(c) => Some(c),
        Err(e) => {
            eprintln!("cannot start preview: {e}");
            None
        }
    }
}

fn watch_child(
    child: &mut Child,
    deps: &[PathBuf],
    stamps: &mut [Option<SystemTime>],
) -> (i32, bool) {
    loop {
        std::thread::sleep(WATCH_POLL);
        for (p, stamp) in deps.iter().zip(stamps.iter_mut()) {
            let now = mtime_of(p);
            if now != *stamp {
                if now.is_some() {
                    eprintln!("{}: change - preview restart", display_name(p));
                }
                *stamp = now;
                let _ = child.kill();
                let _ = child.wait();
                return (0, true);
            }
        }
        match child.try_wait() {
            Ok(Some(status)) => return (status.code().unwrap_or(1), false),
            Ok(None) => {}
            Err(e) => {
                eprintln!("watching error: {e}");
                return (1, false);
            }
        }
    }
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let cfg = match parse_args(&argv) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    if cfg.is_child || !cfg.watch {
        std::process::exit(run_once(&cfg.file));
    }
    std::process::exit(supervise(&cfg.file));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn parses_file_and_watch() {
        let c = parse_args(&args(&["Main.seq", "--watch"])).unwrap();
        assert_eq!(c.file, PathBuf::from("Main.seq"));
        assert!(c.watch);
        assert!(!c.is_child);
    }

    #[test]
    fn rejects_two_files_and_bad_flags() {
        assert!(parse_args(&args(&["a.seq", "b.seq"])).is_err());
        assert!(parse_args(&args(&["a.seq", "--frobnicate"])).is_err());
        assert!(parse_args(&args(&[])).is_err());
    }

    #[test]
    fn bad_seq_file_exits_nonzero_without_window() {
        let dir = std::env::temp_dir();
        let path = dir.join("seq_preview_bad_test.seq");
        std::fs::write(&path, "app A {\n broken\n}\n").unwrap();
        let code = run_once(&path);
        assert_eq!(code, 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn mtime_missing_file_is_none() {
        assert!(mtime_of(Path::new("na-pewno-nie-istnieje.seq")).is_none());
    }

    #[test]
    fn deps_include_transitive_imports() {
        let dir = std::env::temp_dir().join(format!("seq_preview_deps_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("ui")).unwrap();
        std::fs::write(
            dir.join("Main.seq"),
            "import \"./ui/a.seq\"\napp A {\n  title: \"A\"\n  size: (10, 10)\n  window M {\n    Card(t: \"x\")\n  }\n}\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("ui/a.seq"),
            "import \"./card.seq\"\npub component Card(t: String) {\n  Helper(t: t)\n}\ncomponent Helper(t: String) {\n  Text(text: t)\n}\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("ui/card.seq"),
            "pub component Card2(t: String) {\n  Text(text: t)\n}\n",
        )
        .unwrap();
        let deps = deps_of(&dir.join("Main.seq"));
        assert_eq!(deps.len(), 3, "main + a.seq + card.seq");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn deps_fall_back_to_root_on_broken_file() {
        let dir = std::env::temp_dir().join(format!("seq_preview_broken_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("Main.seq");
        std::fs::write(&path, "app A {\n broken\n}\n").unwrap();
        let deps = deps_of(&path);
        assert_eq!(deps, vec![path.clone()]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
