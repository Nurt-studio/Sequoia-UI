use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use crate::error::SeqError;
use crate::ir::{ImportDecl, SeqApp, SeqFile};
use crate::parse_seq_file_all;

const MAX_IMPORT_DEPTH: usize = 32;

pub trait SeqSource {
    fn read(&self, parent: &Path, rel: &str) -> Result<(PathBuf, String), String>;

    fn root(&self, file: &str) -> PathBuf {
        PathBuf::from(normalize_display(Path::new(file)))
    }
}

pub struct FsSource;

impl SeqSource for FsSource {
    fn read(&self, parent: &Path, rel: &str) -> Result<(PathBuf, String), String> {
        let full = join_lexical(parent, rel);
        let canonical = std::fs::canonicalize(&full)
            .map_err(|e| format!("unable to read '{}': {e}", full.display()))?;
        let src = std::fs::read_to_string(&canonical)
            .map_err(|e| format!("unable to read '{}': {e}", full.display()))?;
        Ok((canonical, src))
    }

    fn root(&self, file: &str) -> PathBuf {
        std::fs::canonicalize(file)
            .unwrap_or_else(|_| PathBuf::from(normalize_display(Path::new(file))))
    }
}

pub struct MapSource {
    files: HashMap<String, String>,
}

impl MapSource {
    pub fn new(files: &[(&str, &str)]) -> Self {
        let files = files
            .iter()
            .map(|(p, s)| (normalize_display(Path::new(p)), (*s).to_string()))
            .collect();
        Self { files }
    }
}

impl SeqSource for MapSource {
    fn read(&self, parent: &Path, rel: &str) -> Result<(PathBuf, String), String> {
        let joined = join_lexical(parent, rel);
        let key = normalize_display(&joined);
        match self.files.get(&key) {
            Some(src) => Ok((PathBuf::from(&key), src.clone())),
            None => Err(format!("file not found '{key}'")),
        }
    }
}

pub struct ResolvedImports {
    pub app: SeqApp,

    pub files: Vec<PathBuf>,
}

struct Resolver<'a> {
    source: &'a dyn SeqSource,
    seen: HashMap<PathBuf, ()>,
    stack: Vec<(PathBuf, String)>,
    components: Vec<crate::ir::ComponentDef>,
    files: Vec<PathBuf>,
}

pub fn resolve_imports(
    file: &str,
    root: SeqFile,
    source: &dyn SeqSource,
) -> Result<ResolvedImports, SeqError> {
    let Some(mut app) = root.app else {
        return Err(SeqError::new(file, 1, 1, "missing `app Name { }` block"));
    };
    let root_id = source.root(file);
    let mut r = Resolver {
        source,
        seen: HashMap::new(),
        stack: vec![(root_id.clone(), file.to_string())],
        components: root.components,
        files: vec![root_id],
    };
    r.seen.insert(r.files[0].clone(), ());
    let mut errors = Vec::new();
    r.collect(file, &root.imports, 1, &mut errors);
    if let Some(error) = SeqError::aggregate(errors) {
        return Err(error);
    }
    app.components = r.components;
    app.imports.clear();
    Ok(ResolvedImports {
        app,
        files: r.files,
    })
}

impl Resolver<'_> {
    fn collect(
        &mut self,
        parent_display: &str,
        imports: &[ImportDecl],
        depth: usize,
        errors: &mut Vec<SeqError>,
    ) {
        for imp in imports {
            if depth > MAX_IMPORT_DEPTH {
                errors.push(SeqError::new(
                    parent_display,
                    imp.line,
                    1,
                    format!("import: nesting limit exceeded ({MAX_IMPORT_DEPTH})"),
                ));
                continue;
            }
            let parent = Path::new(parent_display);
            let (id, src) = match self.source.read(parent, &imp.path) {
                Ok(read) => read,
                Err(e) => {
                    errors.push(SeqError::new(
                        parent_display,
                        imp.line,
                        1,
                        format!("import \"{}\": {e}", imp.path),
                    ));
                    continue;
                }
            };
            let display = normalize_display(&join_lexical(parent, &imp.path));
            if let Some(pos) = self.stack.iter().position(|(i, _)| i == &id) {
                let mut names: Vec<String> =
                    self.stack[pos..].iter().map(|(_, d)| d.clone()).collect();
                names.push(display.clone());
                errors.push(SeqError::new(
                    parent_display,
                    imp.line,
                    1,
                    format!("cyclic import: {}", names.join(" -> ")),
                ));
                continue;
            }
            if self.seen.contains_key(&id) {
                continue;
            }
            self.seen.insert(id.clone(), ());
            let (seq, parse_errors) = parse_seq_file_all(&display, &src);
            errors.extend(parse_errors);
            if seq.app.is_some() {
                errors.push(SeqError::new(
                    parent_display,
                    imp.line,
                    1,
                    format!("imported file '{display}' cannot contain `app`"),
                ));
            }
            self.files.push(id.clone());
            self.components.extend(seq.components);
            self.stack.push((id, display.clone()));
            self.collect(&display, &seq.imports, depth + 1, errors);
            self.stack.pop();
        }
    }
}

fn join_lexical(parent: &Path, rel: &str) -> PathBuf {
    let requested = Path::new(rel);
    if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        parent
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(requested)
    }
}

fn normalize_display(path: &Path) -> String {
    let mut out = String::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if out.is_empty() || out.ends_with("..") {
                    push_display(&mut out, "..");
                } else if let Some(pos) = out.rfind('/') {
                    out.truncate(pos);
                } else {
                    out.clear();
                }
            }
            Component::RootDir => out.push('/'),
            Component::Prefix(p) => out.push_str(&p.as_os_str().to_string_lossy()),
            Component::Normal(s) => push_display(&mut out, &s.to_string_lossy()),
        }
    }
    out
}

fn push_display(out: &mut String, segment: &str) {
    if !out.is_empty() && !out.ends_with('/') {
        out.push('/');
    }
    out.push_str(segment);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_relative_and_parent() {
        assert_eq!(normalize_display(Path::new("./ui/card.seq")), "ui/card.seq");
        assert_eq!(normalize_display(Path::new("a/../b.seq")), "b.seq");
        assert_eq!(normalize_display(Path::new("../x.seq")), "../x.seq");
        assert_eq!(normalize_display(Path::new("ui/./card.seq")), "ui/card.seq");
    }

    #[test]
    fn join_relative_to_file_dir() {
        let j = join_lexical(Path::new("dir/Main.seq"), "./ui/card.seq");
        assert_eq!(normalize_display(&j), "dir/ui/card.seq");
    }
}
