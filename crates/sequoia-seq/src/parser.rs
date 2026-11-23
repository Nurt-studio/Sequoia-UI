use crate::error::SeqError;
use crate::ir::{
    BinOp, CompMember, CompMemberKind, CompParam, ComponentDef, DeferredDecl, DeferredKind,
    FnParam, ImportDecl, LengthVal, MatchPat, MotionSpec, SeqApp, SeqArg, SeqExpr, SeqExprKind,
    SeqFile, SeqFn, SeqMatchArm, SeqNode, SeqStmt, SeqValue, SeqWindow, SpringIr, StateDecl,
    StyleDecl, ThemeName, UnOp, Visibility,
};

pub fn parse(file: &str, src: &str) -> Result<SeqApp, SeqError> {
    let (app, errors) = parse_all(file, src);
    if let Some(error) = SeqError::aggregate(errors) {
        return Err(error);
    }
    app.ok_or_else(|| SeqError::new(file, 1, 1, "missing `app Name { }` block"))
}

pub fn parse_all(file: &str, src: &str) -> (Option<SeqApp>, Vec<SeqError>) {
    let (seq, mut errors) = parse_seq_file_all(file, src);
    let SeqFile {
        imports,
        components,
        app,
    } = seq;
    let app = app.map(|mut app| {
        app.components = components;
        app.imports = imports;
        app
    });
    if app.is_none() && errors.is_empty() {
        errors.push(SeqError::new(file, 1, 1, "missing `app Name { }` block"));
    }
    (app, errors)
}

pub fn parse_seq_file(file: &str, src: &str) -> Result<SeqFile, SeqError> {
    let (seq, errors) = parse_seq_file_all(file, src);
    match SeqError::aggregate(errors) {
        Some(error) => Err(error),
        None => Ok(seq),
    }
}

pub fn parse_seq_file_all(file: &str, src: &str) -> (SeqFile, Vec<SeqError>) {
    let mut p = Parser::new(file, src);
    p.parse_seq_file_all()
}

const MAX_NESTING_DEPTH: usize = 256;

const MAX_ERRORS: usize = 100;

fn is_app_item_keyword(kw: &str) -> bool {
    matches!(
        kw,
        "title" | "size" | "theme" | "window" | "prop" | "on" | "state" | "fn" | "style"
    )
}

fn is_single_value_expr(kind: &SeqExprKind) -> bool {
    matches!(
        kind,
        SeqExprKind::Int(_)
            | SeqExprKind::Float(_)
            | SeqExprKind::Str(_)
            | SeqExprKind::Color(_)
            | SeqExprKind::Bool(_)
            | SeqExprKind::Var(_)
            | SeqExprKind::Field { .. }
            | SeqExprKind::Unary { .. }
            | SeqExprKind::Call { .. }
            | SeqExprKind::Array(_)
    )
}

struct Parser<'a> {
    file: &'a str,
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    depth: usize,
    errors: Vec<SeqError>,
}

impl<'a> Parser<'a> {
    fn new(file: &'a str, src: &str) -> Self {
        Self {
            file,
            chars: src.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            depth: 0,
            errors: Vec::new(),
        }
    }

    fn err(&self, line: usize, col: usize, message: impl Into<String>) -> SeqError {
        SeqError::new(self.file, line, col, message)
    }

    fn push_error(&mut self, error: SeqError) {
        if self.errors.len() < MAX_ERRORS {
            self.errors.push(error);
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied()?;
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c == ' ' || c == '\t' || c == '\r' || c == '\n' => {
                    self.bump();
                }
                Some('/') if self.chars.get(self.pos + 1) == Some(&'/') => {
                    while !matches!(self.peek(), None | Some('\n')) {
                        self.bump();
                    }
                }
                _ => break,
            }
        }
    }

    fn loc(&mut self) -> (usize, usize) {
        self.skip_trivia();
        (self.line, self.col)
    }

    fn eat(&mut self, c: char) -> bool {
        self.skip_trivia();
        if self.peek() == Some(c) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, c: char, what: &str) -> Result<(), SeqError> {
        let (line, col) = self.loc();
        if self.peek() == Some(c) {
            self.bump();
            Ok(())
        } else {
            Err(self.err(line, col, format!("expected {what}")))
        }
    }

    fn ident(&mut self) -> Result<(String, usize, usize), SeqError> {
        let (line, col) = self.loc();
        let start = self.pos;
        match self.peek() {
            Some(c) if c.is_ascii_alphabetic() || c == '_' => {
                self.bump();
            }
            _ => return Err(self.err(line, col, "expected identifier")),
        }
        while matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric() || c == '_') {
            self.bump();
        }
        Ok((self.chars[start..self.pos].iter().collect(), line, col))
    }

    fn number(&mut self) -> Result<(f32, usize, usize), SeqError> {
        let (line, col) = self.loc();
        let start = self.pos;
        if self.peek() == Some('-') {
            self.bump();
        }
        let mut digits = 0;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.bump();
            digits += 1;
        }
        if self.peek() == Some('.') {
            self.bump();
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.bump();
                digits += 1;
            }
        }
        if digits == 0 {
            return Err(self.err(line, col, "expected number"));
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        text.parse::<f32>()
            .map(|v| (v, line, col))
            .map_err(|_| self.err(line, col, format!("bad number '{text}'")))
    }

    fn string(&mut self) -> Result<(String, usize, usize), SeqError> {
        let (line, col) = self.loc();
        if self.peek() != Some('"') {
            return Err(self.err(line, col, "expected string in quotation marks"));
        }
        self.bump();
        let mut out = String::new();
        loop {
            match self.bump() {
                None => return Err(self.err(line, col, "unclosed string")),
                Some('"') => break,
                Some('\\') => match self.bump() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some(c) => {
                        return Err(self.err(self.line, self.col, format!("bad escape '\\{c}'")));
                    }
                    None => return Err(self.err(line, col, "unclosed string")),
                },
                Some(c) => out.push(c),
            }
        }
        Ok((out, line, col))
    }

    fn color(&mut self) -> Result<(String, usize, usize), SeqError> {
        let (line, col) = self.loc();
        debug_assert_eq!(self.peek(), Some('#'));
        self.bump();
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_hexdigit()) {
            self.bump();
        }
        let hex: String = self.chars[start..self.pos].iter().collect();
        if hex.len() != 6 && hex.len() != 8 {
            return Err(self.err(line, col, "color must be #RRGGBB or #RRGGBBAA"));
        }
        Ok((hex, line, col))
    }

    fn at_line_start(&self) -> bool {
        let mut i = self.pos;
        while i > 0 {
            let c = self.chars[i - 1];
            if c == ' ' || c == '\t' || c == '\r' {
                i -= 1;
            } else {
                return c == '\n';
            }
        }
        true
    }

    fn skip_string_lossy(&mut self) {
        if self.peek() != Some('"') {
            return;
        }
        self.bump();
        while let Some(c) = self.bump() {
            match c {
                '"' => return,
                '\\' => {
                    self.bump();
                }
                _ => {}
            }
        }
    }

    fn skip_comment(&mut self) {
        while !matches!(self.peek(), None | Some('\n')) {
            self.bump();
        }
    }

    fn recover_top_level(&mut self) {
        let mut depth = 0i32;
        loop {
            self.skip_trivia();
            match self.peek() {
                None => return,
                Some('{') => {
                    depth += 1;
                    self.bump();
                }
                Some('}') => {
                    if depth > 0 {
                        depth -= 1;
                    }
                    self.bump();
                }
                Some('"') => self.skip_string_lossy(),
                Some('/') if self.chars.get(self.pos + 1) == Some(&'/') => self.skip_comment(),
                Some(c)
                    if depth == 0
                        && (c.is_ascii_alphabetic() || c == '_')
                        && self.at_line_start() =>
                {
                    let save = (self.pos, self.line, self.col);
                    match self.ident() {
                        Ok((kw, _, _))
                            if matches!(kw.as_str(), "app" | "component" | "pub" | "import") =>
                        {
                            (self.pos, self.line, self.col) = save;
                            return;
                        }
                        _ => {
                            (self.pos, self.line, self.col) = save;
                            self.bump();
                        }
                    }
                }
                _ => {
                    self.bump();
                }
            }
        }
    }

    fn recover_app_item(&mut self) {
        let mut depth = 0i32;
        loop {
            self.skip_trivia();
            match self.peek() {
                None => return,
                Some('{') => {
                    depth += 1;
                    self.bump();
                }
                Some('}') => {
                    if depth == 0 {
                        return;
                    }
                    depth -= 1;
                    self.bump();
                }
                Some('"') => self.skip_string_lossy(),
                Some('/') if self.chars.get(self.pos + 1) == Some(&'/') => self.skip_comment(),
                Some(c)
                    if depth == 0
                        && (c.is_ascii_alphabetic() || c == '_')
                        && self.at_line_start() =>
                {
                    let save = (self.pos, self.line, self.col);
                    match self.ident() {
                        Ok((kw, _, _)) if is_app_item_keyword(&kw) => {
                            (self.pos, self.line, self.col) = save;
                            return;
                        }
                        _ => {
                            (self.pos, self.line, self.col) = save;
                            self.bump();
                        }
                    }
                }
                _ => {
                    self.bump();
                }
            }
        }
    }

    fn recover_block_item(&mut self) {
        let mut depth = 0i32;
        loop {
            self.skip_trivia();
            match self.peek() {
                None => return,
                Some('{') => {
                    depth += 1;
                    self.bump();
                }
                Some('}') => {
                    if depth == 0 {
                        return;
                    }
                    depth -= 1;
                    self.bump();
                }
                Some('"') => self.skip_string_lossy(),
                Some('/') if self.chars.get(self.pos + 1) == Some(&'/') => self.skip_comment(),
                Some(c)
                    if depth == 0
                        && (c.is_ascii_alphabetic() || c == '_')
                        && self.at_line_start() =>
                {
                    let save = (self.pos, self.line, self.col);
                    match self.ident() {
                        Ok((kw, _, _))
                            if kw == "children"
                                || kw.starts_with(|ch: char| ch.is_ascii_uppercase()) =>
                        {
                            (self.pos, self.line, self.col) = save;
                            return;
                        }
                        _ => {
                            (self.pos, self.line, self.col) = save;
                            self.bump();
                        }
                    }
                }
                _ => {
                    self.bump();
                }
            }
        }
    }

    fn recover_stmt(&mut self) {
        let mut depth = 0i32;
        loop {
            self.skip_trivia();
            match self.peek() {
                None => return,
                Some(';') if depth == 0 => return,
                Some('}') if depth == 0 => return,
                Some('{') => {
                    depth += 1;
                    self.bump();
                }
                Some('}') => {
                    depth -= 1;
                    self.bump();
                }
                Some('"') => self.skip_string_lossy(),
                Some('/') if self.chars.get(self.pos + 1) == Some(&'/') => self.skip_comment(),
                _ => {
                    self.bump();
                }
            }
        }
    }

    fn recover_delimited(&mut self, stops: &[char]) {
        let mut parens = 0i32;
        let mut brackets = 0i32;
        let mut braces = 0i32;
        loop {
            self.skip_trivia();
            match self.peek() {
                None => return,
                Some('"') => self.skip_string_lossy(),
                Some('/') if self.chars.get(self.pos + 1) == Some(&'/') => self.skip_comment(),
                Some('(') => {
                    parens += 1;
                    self.bump();
                }
                Some('[') => {
                    brackets += 1;
                    self.bump();
                }
                Some('{') => {
                    braces += 1;
                    self.bump();
                }
                Some(')') => {
                    if parens == 0 && brackets == 0 && braces == 0 {
                        return;
                    }
                    if parens > 0 {
                        parens -= 1;
                    }
                    self.bump();
                }
                Some(']') => {
                    if parens == 0 && brackets == 0 && braces == 0 {
                        return;
                    }
                    if brackets > 0 {
                        brackets -= 1;
                    }
                    self.bump();
                }
                Some('}') => {
                    if parens == 0 && brackets == 0 && braces == 0 {
                        return;
                    }
                    if braces > 0 {
                        braces -= 1;
                    }
                    self.bump();
                }
                Some(c) if parens == 0 && brackets == 0 && braces == 0 && stops.contains(&c) => {
                    return;
                }
                _ => {
                    self.bump();
                }
            }
        }
    }

    fn parse_seq_file_all(&mut self) -> (SeqFile, Vec<SeqError>) {
        let mut app: Option<SeqApp> = None;
        let mut components: Vec<ComponentDef> = Vec::new();
        let mut imports: Vec<ImportDecl> = Vec::new();
        loop {
            self.skip_trivia();
            if self.peek().is_none() {
                break;
            }
            let before = self.pos;
            if let Err(error) = self.top_item(&mut app, &mut components, &mut imports) {
                self.push_error(error);
                self.recover_top_level();
            }
            if self.pos == before {
                self.bump();
            }
        }
        if app.is_none() && components.is_empty() && imports.is_empty() && self.errors.is_empty() {
            self.push_error(self.err(1, 1, "empty file: expected `app`, `component` or `import`"));
        }
        (
            SeqFile {
                imports,
                components,
                app,
            },
            std::mem::take(&mut self.errors),
        )
    }

    fn top_item(
        &mut self,
        app: &mut Option<SeqApp>,
        components: &mut Vec<ComponentDef>,
        imports: &mut Vec<ImportDecl>,
    ) -> Result<(), SeqError> {
        let (kw, line, col) = self.ident()?;
        match kw.as_str() {
            "app" => {
                if app.is_some() {
                    return Err(self.err(line, col, "second `app` block (only one allowed)"));
                }
                *app = Some(self.app_rest(line, col)?);
            }
            "import" => {
                let (path, _, _) = self.string()?;
                imports.push(ImportDecl { path, line });
            }
            "pub" => {
                let (kw2, l2, c2) = self.ident()?;
                if kw2 != "component" {
                    return Err(self.err(l2, c2, "`pub` only before `component`"));
                }
                components.push(self.component_rest(line, Visibility::Pub)?);
            }
            "component" => {
                components.push(self.component_rest(line, Visibility::Priv)?);
            }
            _ => {
                return Err(self.err(line, col, "expected `app`, `component` or `import`"));
            }
        }
        Ok(())
    }

    fn app_rest(&mut self, line: usize, col: usize) -> Result<SeqApp, SeqError> {
        let (name, _, _) = self.ident()?;
        let _ = name;
        self.expect('{', "'{' after `app Name`")?;
        let mut app = SeqApp {
            title: String::new(),
            size: (0.0, 0.0),
            theme: ThemeName::Dark,
            windows: Vec::new(),
            deferred: Vec::new(),
            styles: Vec::new(),
            components: Vec::new(),
            states: Vec::new(),
            fns: Vec::new(),
            imports: Vec::new(),
            app_line: line,
            has_title: false,
            has_size: false,
        };
        loop {
            self.skip_trivia();
            if self.peek() == Some('}') {
                self.bump();
                break;
            }
            if self.peek().is_none() {
                self.push_error(self.err(self.line, self.col, "unclosed `app` block"));
                break;
            }
            let before = self.pos;
            if let Err(error) = self.app_item(&mut app) {
                self.push_error(error);
                self.recover_app_item();
            }
            if self.pos == before {
                self.bump();
            }
        }
        if !app.has_title {
            self.push_error(self.err(line, col, "app requires `title: \"...\"`"));
        }
        if !app.has_size {
            self.push_error(self.err(line, col, "app requires `size: (W, H)`"));
        }
        if app.windows.is_empty() {
            self.push_error(self.err(line, col, "app requires at least 1 `window` block"));
        }
        Ok(app)
    }

    fn component_rest(
        &mut self,
        line: usize,
        visibility: Visibility,
    ) -> Result<ComponentDef, SeqError> {
        let (name, _, _) = self.ident()?;
        self.expect('(', "'(' after component name")?;
        let mut params = Vec::new();
        loop {
            self.skip_trivia();
            if self.eat(')') {
                break;
            }
            let (pname, pline, _) = self.ident()?;
            self.expect(':', "':' after parameter name")?;
            let (pty, _, _) = self.ident()?;
            params.push(CompParam {
                name: pname,
                line: pline,
                ty: pty,
            });
            if !self.eat(',') {
                self.expect(')', "')' after `component` parameters")?;
                break;
            }
        }
        self.expect('{', "'{' after `component` header")?;
        let mut members = Vec::new();
        let mut body = Vec::new();
        loop {
            self.skip_trivia();
            if self.peek() == Some('}') {
                self.bump();
                break;
            }
            if self.peek().is_none() {
                self.push_error(self.err(self.line, self.col, "unclosed `component` block"));
                break;
            }
            let save = (self.pos, self.line, self.col);
            let mut member_kw = None;
            if let Ok((word, kline, _)) = self.ident()
                && (word == "prop" || word == "state")
            {
                member_kw = Some((word, kline));
            }
            if let Some((word, kline)) = member_kw {
                members.push(self.component_member(kline, word == "prop")?);
                continue;
            }
            (self.pos, self.line, self.col) = save;
            let before = self.pos;
            match self.node() {
                Ok(node) => body.push(node),
                Err(error) => {
                    self.push_error(error);
                    self.recover_block_item();
                }
            }
            if self.pos == before {
                self.bump();
            }
        }
        if body.is_empty() {
            self.push_error(self.err(line, 1, "component requires at least 1 node"));
        }
        Ok(ComponentDef {
            name,
            line,
            params,
            members,
            body,
            visibility,
            file: self.file.to_string(),
        })
    }

    fn component_member(&mut self, line: usize, is_prop: bool) -> Result<CompMember, SeqError> {
        let (name, _, _) = self.ident()?;
        self.expect(':', "':' after component prop name")?;
        let ty = self.fn_type()?;
        let init = if is_prop {
            None
        } else {
            self.expect('=', "'=' after component state type")?;
            Some(self.state_literal()?)
        };
        self.eat(';');
        Ok(CompMember {
            kind: if is_prop {
                CompMemberKind::Prop
            } else {
                CompMemberKind::State
            },
            name,
            line,
            ty,
            init,
        })
    }

    fn app_item(&mut self, app: &mut SeqApp) -> Result<(), SeqError> {
        let (kw, line, col) = self.ident()?;
        match kw.as_str() {
            "title" => {
                self.expect(':', "':' after `title`")?;
                let (s, _, _) = self.string()?;
                app.title = s;
                app.has_title = true;
            }
            "size" => {
                self.expect(':', "':' after `size`")?;
                app.size = self.size_tuple()?;
                app.has_size = true;
            }
            "theme" => {
                self.expect(':', "':' after `theme`")?;
                app.theme = self.theme_name()?;
            }
            "window" => {
                let w = self.window(line)?;
                app.windows.push(w);
            }
            "prop" => {
                let (name, _, _) = self.ident()?;
                self.expect(':', "':' after prop name")?;
                let (ty, _, _) = self.ident()?;
                app.deferred.push(DeferredDecl {
                    kind: DeferredKind::Prop,
                    name,
                    line,
                    sig: vec![ty],
                });
            }
            "on" => {
                let (name, _, _) = self.ident()?;
                self.expect('(', "'(' after on name")?;
                let mut sig = Vec::new();
                loop {
                    self.skip_trivia();
                    if self.eat(')') {
                        break;
                    }
                    let (_, _, _) = self.ident()?;
                    self.expect(':', "':' after parameter name")?;
                    let (ty, _, _) = self.ident()?;
                    sig.push(ty);
                    if !self.eat(',') {
                        self.expect(')', "')' after `on` parameters")?;
                        break;
                    }
                }
                app.deferred.push(DeferredDecl {
                    kind: DeferredKind::On,
                    name,
                    line,
                    sig,
                });
            }
            "state" => {
                let (name, _, _) = self.ident()?;
                self.expect(':', "':' after state name")?;
                let ty = self.fn_type()?;
                self.expect('=', "'=' after state type")?;
                let init = self.state_literal()?;
                self.eat(';');
                app.states.push(StateDecl {
                    name,
                    line,
                    ty,
                    init,
                });
            }
            "fn" => {
                let f = self.fn_decl(line)?;
                app.fns.push(f);
            }
            "style" => {
                let (name, _, _) = self.ident()?;
                self.expect(':', "':' after style name")?;
                let (target, _, _) = self.ident()?;
                let mut base = None;
                let (save_pos, save_line, save_col) = (self.pos, self.line, self.col);
                if let Ok((w, _, _)) = self.ident() {
                    if w == "extends" {
                        let (b, _, _) = self.ident()?;
                        base = Some(b);
                    } else {
                        (self.pos, self.line, self.col) = (save_pos, save_line, save_col);
                    }
                } else {
                    (self.pos, self.line, self.col) = (save_pos, save_line, save_col);
                }
                self.expect('{', "'{' after `style Name: Type`")?;
                let args = self.style_args()?;
                self.expect('}', "'}' after style fields")?;
                app.styles.push(StyleDecl {
                    name,
                    line,
                    target,
                    base,
                    args,
                });
            }
            "component" => {
                return Err(self.err(
                    line,
                    col,
                    "component block outside `app` (define alongside `app`)",
                ));
            }
            "import" => {
                return Err(self.err(
                    line,
                    col,
                    "`import` only at file level (outside `app` block)",
                ));
            }
            "pub" => {
                return Err(self.err(
                    line,
                    col,
                    "`pub` only before `component` (outside `app` block)",
                ));
            }
            _ => {
                return Err(self.err(
                    line,
                    col,
                    format!(
                        "unknown app element '{kw}' (title/size/theme/window/prop/on/state/fn/style)"
                    ),
                ));
            }
        }
        Ok(())
    }

    fn state_literal(&mut self) -> Result<SeqExpr, SeqError> {
        self.skip_trivia();
        let (line, col) = (self.line, self.col);
        match self.peek() {
            Some('"') => {
                let (s, _, _) = self.string()?;
                Ok(SeqExpr {
                    line,
                    kind: SeqExprKind::Str(s),
                })
            }
            Some(c) if c == '-' || c.is_ascii_digit() => {
                let (n, int, _, _) = self.number_expr()?;
                Ok(SeqExpr {
                    line,
                    kind: if int {
                        SeqExprKind::Int(n as i64)
                    } else {
                        SeqExprKind::Float(n)
                    },
                })
            }
            _ => {
                let (id, _, _) = self.ident()?;
                match id.as_str() {
                    "true" => Ok(SeqExpr {
                        line,
                        kind: SeqExprKind::Bool(true),
                    }),
                    "false" => Ok(SeqExpr {
                        line,
                        kind: SeqExprKind::Bool(false),
                    }),
                    "None" => Ok(SeqExpr {
                        line,
                        kind: SeqExprKind::Var("None".to_string()),
                    }),
                    other => Err(self.err(
                        line,
                        col,
                        format!(
                            "state '{other}': initial value is literal (number/string/true/false)"
                        ),
                    )),
                }
            }
        }
    }

    fn fn_decl(&mut self, line: usize) -> Result<SeqFn, SeqError> {
        let (name, _, _) = self.ident()?;
        self.expect('(', "'(' po nazwie funkcji")?;
        let mut params = Vec::new();
        loop {
            self.skip_trivia();
            if self.eat(')') {
                break;
            }
            let mut mutable = false;
            let (first, first_line, _) = self.ident()?;
            let pname;
            let pline;
            if first == "mut" {
                mutable = true;
                let (n, l, _) = self.ident()?;
                pname = n;
                pline = l;
            } else {
                pname = first;
                pline = first_line;
            }
            self.expect(':', "':' po nazwie parametru funkcji")?;
            let ty = self.fn_type()?;
            params.push(FnParam {
                name: pname,
                mutable,
                line: pline,
                ty,
            });
            if !self.eat(',') {
                self.expect(')', "')' po parametrach funkcji")?;
                break;
            }
        }
        let mut ret = None;
        if self.eat_arrow() {
            ret = Some(self.fn_type()?);
        }
        let body = self.fn_block("funkcji")?;
        Ok(SeqFn {
            name,
            line,
            params,
            ret,
            body,
        })
    }

    fn fn_type(&mut self) -> Result<String, SeqError> {
        self.skip_trivia();
        if self.peek() == Some('O') {
            let save = (self.pos, self.line, self.col);
            if let Ok((id, _, _)) = self.ident()
                && id == "Option"
            {
                self.skip_trivia();
                if self.eat('<') {
                    let inner = self.fn_type()?;
                    self.skip_trivia();
                    self.expect('>', "'>' po typie Option")?;
                    return Ok(format!("Option<{}>", inner));
                }
            } else {
                (self.pos, self.line, self.col) = save;
            }
            (self.pos, self.line, self.col) = save;
        }
        if self.eat('(') {
            let mut parts = vec![self.fn_type()?];
            while self.eat(',') {
                parts.push(self.fn_type()?);
            }
            self.expect(')', "')' po typie krotki")?;
            Ok(format!("({})", parts.join(",")))
        } else {
            let (id, _, _) = self.ident()?;
            Ok(id)
        }
    }

    fn eat_arrow(&mut self) -> bool {
        let save = (self.pos, self.line, self.col);
        self.skip_trivia();
        if self.peek() == Some('-') && self.chars.get(self.pos + 1) == Some(&'>') {
            self.bump();
            self.bump();
            true
        } else {
            (self.pos, self.line, self.col) = save;
            false
        }
    }

    fn fn_block(&mut self, what: &str) -> Result<Vec<SeqStmt>, SeqError> {
        self.depth += 1;
        if self.depth > MAX_NESTING_DEPTH {
            self.depth -= 1;
            return Err(self.err(
                self.line,
                self.col,
                format!("nesting too deep (limit {MAX_NESTING_DEPTH} levels)"),
            ));
        }
        let result = self.fn_block_inner(what);
        self.depth -= 1;
        result
    }

    fn fn_block_inner(&mut self, what: &str) -> Result<Vec<SeqStmt>, SeqError> {
        self.expect('{', format!("'{{' w ciele {what}").as_str())?;
        let mut out = Vec::new();
        loop {
            self.skip_trivia();
            if self.peek() == Some('}') {
                self.bump();
                break;
            }
            if self.peek().is_none() {
                self.push_error(self.err(self.line, self.col, format!("unclosed block {what}")));
                break;
            }
            let before = self.pos;
            match self.fn_stmt() {
                Ok(Some(stmt)) => out.push(stmt),
                Ok(None) => {}
                Err(error) => {
                    self.push_error(error);
                    self.recover_stmt();
                }
            }
            if self.pos == before {
                self.bump();
            }
        }
        Ok(out)
    }

    fn fn_stmt(&mut self) -> Result<Option<SeqStmt>, SeqError> {
        self.skip_trivia();
        if self.eat(';') {
            return Ok(None);
        }
        let (kw, line, col) = self.ident()?;
        match kw.as_str() {
            "let" | "mut" => {
                let (name, _, _) = self.ident()?;
                self.expect('=', "'=' po nazwie zmiennej")?;
                let value = self.fn_expr()?;
                self.expect(';', "';' po `let`/`mut`")?;
                Ok(Some(SeqStmt::Let {
                    name,
                    mutable: kw == "mut",
                    value,
                    line,
                }))
            }
            "if" => Ok(Some(self.if_rest(line)?)),
            "for" => {
                let (var, _, _) = self.ident()?;
                let (in_kw, l2, c2) = self.ident()?;
                if in_kw != "in" {
                    return Err(self.err(l2, c2, "'in' after loop variable `for x in rows`"));
                }
                let (source, _, _) = self.ident()?;
                let body = self.fn_block("for loop")?;
                Ok(Some(SeqStmt::For {
                    var,
                    source,
                    body,
                    line,
                }))
            }
            "match" => self.match_rest(line),
            "return" => {
                self.skip_trivia();
                let value = if self.peek() == Some(';') {
                    None
                } else {
                    Some(self.fn_expr()?)
                };
                self.expect(';', "';' po `return`")?;
                Ok(Some(SeqStmt::Return { value, line }))
            }
            _ => {
                if self.eat_eq() {
                    let value = self.fn_expr()?;
                    self.expect(';', "';' po przypisaniu")?;
                    return Ok(Some(SeqStmt::Assign {
                        name: kw,
                        value,
                        line,
                    }));
                }
                if self.eat('.') {
                    let (field, _, _) = self.ident()?;
                    self.expect('=', "'=' after instance field")?;
                    let value = self.fn_expr()?;
                    self.expect(';', "';' after assignment")?;
                    return Ok(Some(SeqStmt::Assign {
                        name: format!("{kw}.{field}"),
                        value,
                        line,
                    }));
                }
                let base = SeqExpr {
                    line,
                    kind: SeqExprKind::Var(kw),
                };
                let expr = self.fn_postfix(base)?;
                if matches!(expr.kind, SeqExprKind::Var(_) | SeqExprKind::Field { .. }) {
                    return Err(self.err(
                        line,
                        col,
                        "an expression as a statement is a function call (e.g. `connect();`)",
                    ));
                }
                self.expect(';', "';' po instrukcji")?;
                Ok(Some(SeqStmt::Expr(expr)))
            }
        }
    }

    fn if_rest(&mut self, line: usize) -> Result<SeqStmt, SeqError> {
        self.expect('(', "'(' po `if`")?;
        let cond = self.fn_expr()?;
        self.expect(')', "')' po warunku `if`")?;
        let then = self.fn_block("branch `if`")?;
        let mut elifs = Vec::new();
        let mut els = None;
        loop {
            if !self.peek_keyword("else") {
                break;
            }
            let _ = self.ident();
            if self.peek_keyword("if") {
                let _ = self.ident();
                self.expect('(', "'(' po `else if`")?;
                let cond = self.fn_expr()?;
                self.expect(')', "')' po warunku `else if`")?;
                let body = self.fn_block("branch `else if`")?;
                elifs.push((cond, body));
            } else {
                els = Some(self.fn_block("branch `else`")?);
                break;
            }
        }
        Ok(SeqStmt::If {
            cond,
            then,
            elifs,
            els,
            line,
        })
    }

    fn match_rest(&mut self, line: usize) -> Result<Option<SeqStmt>, SeqError> {
        let value = self.fn_expr()?;
        self.expect('{', "'{' after `match`")?;
        let mut arms = Vec::new();
        loop {
            self.skip_trivia();
            if self.peek() == Some('}') {
                self.bump();
                break;
            }
            if self.peek().is_none() {
                return Err(self.err(self.line, self.col, "unclosed `match` block"));
            }
            let (aline, acol) = (self.line, self.col);
            let pattern = match self.peek() {
                Some('"') => MatchPat::Str(self.string()?.0),
                Some(c) if c == '-' || c.is_ascii_digit() => {
                    let (n, int, _, _) = self.number_expr()?;
                    if !int {
                        return Err(self.err(aline, acol, "match pattern is integer"));
                    }
                    #[allow(clippy::cast_possible_truncation)]
                    MatchPat::Int(n as i64)
                }
                _ => {
                    let (id, _, _) = self.ident()?;
                    match id.as_str() {
                        "true" => MatchPat::Bool(true),
                        "false" => MatchPat::Bool(false),
                        "_" => MatchPat::Wildcard,
                        other => {
                            return Err(self.err(
                                aline,
                                acol,
                                format!(
                                    "wzorzec match to \"tekst\"/liczba/true/false/_, nie '{other}'"
                                ),
                            ));
                        }
                    }
                }
            };
            self.expect('=', "'=' we wzorcu match (`wzorzec => { ... }`)")?;
            self.expect('>', "'>' we wzorcu match (`wzorzec => { ... }`)")?;
            let body = self.fn_block("branch `match`")?;
            arms.push(SeqMatchArm {
                pattern,
                body,
                line: aline,
            });
            self.eat(',');
        }
        Ok(Some(SeqStmt::Match { value, arms, line }))
    }

    fn eat_eq(&mut self) -> bool {
        let save = (self.pos, self.line, self.col);
        self.skip_trivia();
        if self.peek() == Some('=') && self.chars.get(self.pos + 1) != Some(&'=') {
            self.bump();
            true
        } else {
            (self.pos, self.line, self.col) = save;
            false
        }
    }

    fn peek_keyword(&mut self, kw: &str) -> bool {
        let save = (self.pos, self.line, self.col);
        let ok = matches!(self.ident(), Ok((w, _, _)) if w == kw);
        (self.pos, self.line, self.col) = save;
        ok
    }

    fn number_expr(&mut self) -> Result<(f64, bool, usize, usize), SeqError> {
        let (line, col) = self.loc();
        let start = self.pos;
        if self.peek() == Some('-') {
            self.bump();
        }
        let mut digits = 0;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.bump();
            digits += 1;
        }
        let mut int = true;
        if self.peek() == Some('.') {
            int = false;
            self.bump();
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.bump();
                digits += 1;
            }
        }
        if digits == 0 {
            return Err(self.err(line, col, "expected number"));
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        text.parse::<f64>()
            .map(|v| (v, int, line, col))
            .map_err(|_| self.err(line, col, format!("bad number '{text}'")))
    }

    fn peek_paren_trivia(&mut self) -> bool {
        self.skip_trivia();
        self.peek() == Some('(')
    }

    fn fn_or(&mut self) -> Result<SeqExpr, SeqError> {
        let mut lhs = self.fn_and()?;
        loop {
            if !self.eat_op("||") {
                break;
            }
            let line = self.line;
            let rhs = self.fn_and()?;
            lhs = SeqExpr {
                line,
                kind: SeqExprKind::Binary {
                    op: BinOp::Or,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
            };
        }
        Ok(lhs)
    }

    fn fn_and(&mut self) -> Result<SeqExpr, SeqError> {
        let mut lhs = self.fn_eq()?;
        loop {
            if !self.eat_op("&&") {
                break;
            }
            let line = self.line;
            let rhs = self.fn_eq()?;
            lhs = SeqExpr {
                line,
                kind: SeqExprKind::Binary {
                    op: BinOp::And,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
            };
        }
        Ok(lhs)
    }

    fn fn_eq(&mut self) -> Result<SeqExpr, SeqError> {
        let mut lhs = self.fn_cmp()?;
        loop {
            let op = if self.eat_op("==") {
                BinOp::Eq
            } else if self.eat_op("!=") {
                BinOp::Ne
            } else {
                break;
            };
            let line = self.line;
            let rhs = self.fn_cmp()?;
            lhs = SeqExpr {
                line,
                kind: SeqExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
            };
        }
        Ok(lhs)
    }

    fn fn_cmp(&mut self) -> Result<SeqExpr, SeqError> {
        let mut lhs = self.fn_add()?;
        loop {
            let op = if self.eat_op("<=") {
                BinOp::Le
            } else if self.eat_op(">=") {
                BinOp::Ge
            } else if self.eat_op("<") {
                BinOp::Lt
            } else if self.eat_op(">") {
                BinOp::Gt
            } else {
                break;
            };
            let line = self.line;
            let rhs = self.fn_add()?;
            lhs = SeqExpr {
                line,
                kind: SeqExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
            };
        }
        Ok(lhs)
    }

    fn fn_add(&mut self) -> Result<SeqExpr, SeqError> {
        let mut lhs = self.fn_mul()?;
        loop {
            let op = if self.eat_op("+") {
                BinOp::Add
            } else if self.eat_op("-") {
                BinOp::Sub
            } else {
                break;
            };
            let line = self.line;
            let rhs = self.fn_mul()?;
            lhs = SeqExpr {
                line,
                kind: SeqExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
            };
        }
        Ok(lhs)
    }

    fn fn_mul(&mut self) -> Result<SeqExpr, SeqError> {
        let mut lhs = self.fn_unary()?;
        loop {
            let op = if self.eat_op("*") {
                BinOp::Mul
            } else if self.eat_op("/") {
                BinOp::Div
            } else if self.eat_op("%") {
                BinOp::Rem
            } else {
                break;
            };
            let line = self.line;
            let rhs = self.fn_unary()?;
            lhs = SeqExpr {
                line,
                kind: SeqExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
            };
        }
        Ok(lhs)
    }

    fn fn_unary(&mut self) -> Result<SeqExpr, SeqError> {
        self.skip_trivia();
        let (line, _) = (self.line, self.col);
        if self.eat_op("!") {
            let expr = self.fn_unary()?;
            return Ok(SeqExpr {
                line,
                kind: SeqExprKind::Unary {
                    op: UnOp::Not,
                    expr: Box::new(expr),
                },
            });
        }
        if self.eat_op("-") {
            let expr = self.fn_unary()?;
            return Ok(SeqExpr {
                line,
                kind: SeqExprKind::Unary {
                    op: UnOp::Neg,
                    expr: Box::new(expr),
                },
            });
        }
        let primary = self.fn_primary()?;
        self.fn_postfix(primary)
    }

    fn fn_postfix(&mut self, mut base: SeqExpr) -> Result<SeqExpr, SeqError> {
        loop {
            if self.eat('.') {
                self.skip_trivia();
                let (name, nline) = if matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                    let nline = self.line;
                    let start = self.pos;
                    while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                        self.bump();
                    }
                    (
                        self.chars[start..self.pos].iter().collect::<String>(),
                        nline,
                    )
                } else {
                    let (name, line, _) = self.ident()?;
                    (name, line)
                };
                if self.peek_paren_trivia() {
                    self.expect('(', "'(' po metodzie")?;
                    let args = self.fn_call_args()?;
                    let mut all = vec![base];
                    all.extend(args);
                    base = SeqExpr {
                        line: nline,
                        kind: SeqExprKind::Call { name, args: all },
                    };
                } else {
                    let SeqExprKind::Var(var) = base.kind else {
                        return Err(self.err(
                            nline,
                            self.col,
                            "field only on loop variable (e.g. `x.id`)",
                        ));
                    };
                    base = SeqExpr {
                        line: nline,
                        kind: SeqExprKind::Field {
                            base: var,
                            field: name,
                        },
                    };
                }
            } else if self.peek_paren_trivia() {
                self.skip_trivia();
                let line = self.line;
                self.expect('(', "'(' po nazwie funkcji")?;
                let args = self.fn_call_args()?;
                let SeqExprKind::Var(name) = base.kind else {
                    return Err(self.err(line, self.col, "you can call a function by name"));
                };
                base = SeqExpr {
                    line,
                    kind: SeqExprKind::Call { name, args },
                };
            } else {
                break;
            }
        }
        Ok(base)
    }

    fn fn_call_args(&mut self) -> Result<Vec<SeqExpr>, SeqError> {
        let mut args = Vec::new();
        self.skip_trivia();
        if self.eat(')') {
            return Ok(args);
        }
        args.push(self.fn_expr()?);
        loop {
            if !self.eat(',') {
                break;
            }
            self.skip_trivia();
            if self.eat(')') {
                return Ok(args);
            }
            args.push(self.fn_expr()?);
        }
        self.expect(')', "')' after call arguments")?;
        Ok(args)
    }

    fn fn_primary(&mut self) -> Result<SeqExpr, SeqError> {
        self.depth += 1;
        if self.depth > MAX_NESTING_DEPTH {
            self.depth -= 1;
            return Err(self.err(
                self.line,
                self.col,
                format!("nesting too deep (limit {MAX_NESTING_DEPTH} levels)"),
            ));
        }
        let result = self.fn_primary_inner();
        self.depth -= 1;
        result
    }

    fn fn_primary_inner(&mut self) -> Result<SeqExpr, SeqError> {
        self.skip_trivia();
        let (line, col) = (self.line, self.col);
        match self.peek() {
            Some('"') => {
                let (s, _, _) = self.string()?;
                Ok(SeqExpr {
                    line,
                    kind: SeqExprKind::Str(s),
                })
            }
            Some('#') => {
                let (hex, _, _) = self.color()?;
                Ok(SeqExpr {
                    line,
                    kind: SeqExprKind::Color(hex),
                })
            }
            Some(c) if c.is_ascii_digit() => {
                let (n, int, _, _) = self.number_expr()?;
                Ok(SeqExpr {
                    line,
                    kind: if int {
                        #[allow(clippy::cast_possible_truncation)]
                        SeqExprKind::Int(n as i64)
                    } else {
                        SeqExprKind::Float(n)
                    },
                })
            }
            Some('(') => {
                self.bump();
                let first = self.fn_expr()?;
                self.skip_trivia();
                if self.eat(',') {
                    let mut items = vec![first];
                    loop {
                        self.skip_trivia();
                        if self.eat(')') {
                            break;
                        }
                        items.push(self.fn_expr()?);
                        if !self.eat(',') {
                            self.expect(')', "')' po krotce `(a, b)`")?;
                            break;
                        }
                    }
                    Ok(SeqExpr {
                        line,
                        kind: SeqExprKind::Tuple(items),
                    })
                } else {
                    self.expect(')', "')' after the expression in brackets")?;
                    Ok(first)
                }
            }
            Some('[') => {
                self.bump();
                let mut items = Vec::new();
                self.skip_trivia();
                if !self.eat(']') {
                    loop {
                        items.push(self.fn_expr()?);
                        self.skip_trivia();
                        if self.eat(']') {
                            break;
                        }
                        self.expect(',', "',' or ']' in list `[a, b]`")?;
                        self.skip_trivia();
                        if self.eat(']') {
                            break;
                        }
                    }
                }
                Ok(SeqExpr {
                    line,
                    kind: SeqExprKind::Array(items),
                })
            }
            Some('_') => Err(self.err(line, col, "`_` only as `match` pattern")),
            _ => {
                let (id, _, _) = self.ident()?;
                Ok(SeqExpr {
                    line,
                    kind: match id.as_str() {
                        "true" => SeqExprKind::Bool(true),
                        "false" => SeqExprKind::Bool(false),
                        _ => SeqExprKind::Var(id),
                    },
                })
            }
        }
    }

    fn eat_op(&mut self, s: &str) -> bool {
        self.skip_trivia();
        for (i, c) in s.chars().enumerate() {
            if self.chars.get(self.pos + i) != Some(&c) {
                return false;
            }
        }
        for _ in s.chars() {
            self.bump();
        }
        true
    }

    fn fn_expr(&mut self) -> Result<SeqExpr, SeqError> {
        self.fn_ternary()
    }

    fn fn_ternary(&mut self) -> Result<SeqExpr, SeqError> {
        let cond = self.fn_or()?;
        if !self.eat_op("?") {
            return Ok(cond);
        }
        let line = self.line;
        let then = self.fn_expr()?;
        self.expect(':', "':' in conditional expression `condition ? a : b`")?;
        let els = self.fn_ternary()?;
        Ok(SeqExpr {
            line,
            kind: SeqExprKind::Ternary {
                cond: Box::new(cond),
                then: Box::new(then),
                els: Box::new(els),
            },
        })
    }

    fn theme_name(&mut self) -> Result<ThemeName, SeqError> {
        let (line, col) = self.loc();
        match self.peek() {
            Some('"') => {
                let (s, _, _) = self.string()?;
                match s.as_str() {
                    "dark" => Ok(ThemeName::Dark),
                    "light" => Ok(ThemeName::Light),
                    _ => Ok(ThemeName::File(s)),
                }
            }
            _ => {
                let (id, _, _) = self.ident()?;
                match id.as_str() {
                    "dark" => Ok(ThemeName::Dark),
                    "light" => Ok(ThemeName::Light),
                    _ => Err(self.err(line, col, "theme to dark/light albo \"plik-tokens.json\"")),
                }
            }
        }
    }

    fn size_tuple(&mut self) -> Result<(f32, f32), SeqError> {
        self.expect('(', "'(' przed rozmiarem")?;
        let (w, _, _) = self.number()?;
        self.expect(',', "',' between W and H")?;
        let (h, _, _) = self.number()?;
        self.expect(')', "')' po rozmiarze")?;
        if !(w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0) {
            let (l, c) = (self.line, self.col);
            return Err(self.err(l, c, "size must be positive and finite"));
        }
        Ok((w, h))
    }

    fn style_args(&mut self) -> Result<Vec<SeqArg>, SeqError> {
        let mut args = Vec::new();
        loop {
            self.skip_trivia();
            if self.peek() == Some('}') {
                break;
            }
            if self.peek().is_none() {
                self.push_error(self.err(self.line, self.col, "unclosed `style` block"));
                break;
            }
            let before = self.pos;
            let field = (|| -> Result<SeqArg, SeqError> {
                let (name, line, _) = self.ident()?;
                self.expect(':', format!("':' po polu stylu '{name}'").as_str())?;
                let value = self.value()?;
                Ok(SeqArg { name, line, value })
            })();
            match field {
                Ok(arg) => args.push(arg),
                Err(error) => {
                    self.push_error(error);
                    self.recover_delimited(&[',', '}']);
                }
            }
            if self.pos == before {
                self.bump();
            }
            self.eat(',');
        }
        Ok(args)
    }

    fn window(&mut self, line: usize) -> Result<SeqWindow, SeqError> {
        let (name, _, _) = self.ident()?;
        let mut size = None;
        if self.eat('(') {
            let (kw, l, c) = self.ident()?;
            if kw != "size" {
                return Err(self.err(l, c, "w nawiasie window dozwolone tylko `size: (W, H)`"));
            }
            self.expect(':', "':' po `size`")?;
            size = Some(self.size_tuple()?);
            self.expect(')', "')' po `window Nazwa (...)`")?;
        }
        self.expect('{', "'{' after `window` header")?;
        let mut nodes = Vec::new();
        loop {
            self.skip_trivia();
            if self.peek() == Some('}') {
                self.bump();
                break;
            }
            if self.peek().is_none() {
                self.push_error(self.err(self.line, self.col, "unclosed `window` block"));
                break;
            }
            let before = self.pos;
            match self.node() {
                Ok(node) => nodes.push(node),
                Err(error) => {
                    self.push_error(error);
                    self.recover_block_item();
                }
            }
            if self.pos == before {
                self.bump();
            }
        }
        Ok(SeqWindow {
            name,
            line,
            size,
            nodes,
        })
    }

    fn node(&mut self) -> Result<SeqNode, SeqError> {
        self.depth += 1;
        if self.depth > MAX_NESTING_DEPTH {
            self.depth -= 1;
            return Err(self.err(
                self.line,
                self.col,
                format!("nesting too deep (limit {MAX_NESTING_DEPTH} levels)"),
            ));
        }
        let result = self.node_inner();
        self.depth -= 1;
        result
    }

    fn node_inner(&mut self) -> Result<SeqNode, SeqError> {
        let (kind, line, _) = self.ident()?;
        let mut label = None;
        let mut args = Vec::new();

        if self.eat('(') {
            self.skip_trivia();
            if self.peek() == Some('"') {
                let (s, _, _) = self.string()?;
                label = Some(s);
                self.skip_trivia();
                if self.eat(',') {
                    self.skip_trivia();
                    if self.peek() != Some(')') {
                        args = self.named_args()?;
                    }
                }
            } else if self.peek() != Some(')') {
                args = self.named_args()?;
            }
            self.expect(')', "')' after attributes")?;
        }
        let mut children = Vec::new();
        if self.eat('{') {
            loop {
                self.skip_trivia();
                if self.peek() == Some('}') {
                    self.bump();
                    break;
                }
                if self.peek().is_none() {
                    self.push_error(self.err(self.line, self.col, "unclosed children block `{ }`"));
                    break;
                }
                let before = self.pos;
                match self.node() {
                    Ok(node) => children.push(node),
                    Err(error) => {
                        self.push_error(error);
                        self.recover_block_item();
                    }
                }
                if self.pos == before {
                    self.bump();
                }
            }
        }
        Ok(SeqNode {
            kind,
            line,
            label,
            args,
            children,
        })
    }

    fn named_args(&mut self) -> Result<Vec<SeqArg>, SeqError> {
        let mut args = Vec::new();
        loop {
            self.skip_trivia();
            if self.peek().is_none() || self.peek() == Some(')') {
                break;
            }
            let before = self.pos;
            let arg = (|| -> Result<SeqArg, SeqError> {
                let (name, line, _) = self.ident()?;
                self.expect(':', format!("':' after attribute '{name}'").as_str())?;
                let value = match name.as_str() {
                    "enter" | "exit" => self.motion_value()?,
                    "behavior" => self.behavior_value()?,
                    _ => self.value()?,
                };
                Ok(SeqArg { name, line, value })
            })();
            match arg {
                Ok(arg) => args.push(arg),
                Err(error) => {
                    self.push_error(error);
                    self.recover_delimited(&[',', ')']);
                }
            }
            if self.pos == before {
                self.bump();
            }
            self.skip_trivia();
            if self.peek() == Some(',') {
                self.bump();
                continue;
            }
            break;
        }
        Ok(args)
    }

    /// `enter:` / `exit:` — preset (`smooth`) or motion chain:
    /// `fade().slide_y(16) at 220ms ease_out_cubic`.
    fn motion_value(&mut self) -> Result<SeqValue, SeqError> {
        self.skip_trivia();
        let save = (self.pos, self.line, self.col);
        let ident = self.ident();
        let Ok((id, _, _)) = ident else {
            (self.pos, self.line, self.col) = save;
            return self.value();
        };
        if !matches!(id.as_str(), "fade" | "slide" | "slide_x" | "slide_y") || !self.peek_paren() {
            (self.pos, self.line, self.col) = save;
            return self.value();
        }
        let mut spec = MotionSpec {
            fade: false,
            fade_from: 0.0,
            slide: (0.0, 0.0),
            duration_secs: None,
            delay_secs: 0.0,
            easing: None,
        };
        self.motion_step(&id, &mut spec)?;
        loop {
            self.skip_trivia();
            if self.peek() != Some('.') {
                break;
            }
            self.bump();
            let (method, line, col) = self.ident()?;
            if !matches!(method.as_str(), "fade" | "slide" | "slide_x" | "slide_y")
                || !self.peek_paren()
            {
                return Err(self.err(
                    line,
                    col,
                    "in motion after '.': fade()/slide(x, y)/slide_x(px)/slide_y(px)",
                ));
            }
            self.motion_step(&method, &mut spec)?;
        }
        if self.keyword_next("at") {
            self.consume_keyword("at")?;
            spec.duration_secs = Some(self.motion_time("at")?);
            self.skip_trivia();
            let easing_save = (self.pos, self.line, self.col);
            if matches!(self.peek(), Some(c) if c.is_ascii_alphabetic() || c == '_') {
                let (easing, _, _) = self.ident()?;
                if easing == "delay" {
                    (self.pos, self.line, self.col) = easing_save;
                } else {
                    spec.easing = Some(easing);
                }
            }
        }
        if self.keyword_next("delay") {
            self.consume_keyword("delay")?;
            spec.delay_secs = self.motion_time("delay")?;
        }
        if !self.value_end() {
            let (line, col) = (self.line, self.col);
            return Err(self.err(line, col, "unexpected token in enter/exit motion"));
        }
        Ok(SeqValue::Motion(spec))
    }

    fn motion_step(&mut self, method: &str, spec: &mut MotionSpec) -> Result<(), SeqError> {
        self.expect('(', &format!("'(' after `{method}` in enter/exit"))?;
        self.skip_trivia();
        match method {
            "fade" => {
                spec.fade = true;
                if self.peek() != Some(')') {
                    let (from, line, _) = self.number()?;
                    if !(0.0..=1.0).contains(&from) {
                        return Err(self.err(line, 1, "fade(from): from in range 0..=1"));
                    }
                    spec.fade_from = from;
                }
            }
            "slide_y" => {
                let (y, _, _) = self.number()?;
                spec.slide = (spec.slide.0, y);
            }
            "slide_x" => {
                let (x, _, _) = self.number()?;
                spec.slide = (x, spec.slide.1);
            }
            "slide" => {
                let (x, _, _) = self.number()?;
                self.expect(',', "',' in slide(x, y)")?;
                self.skip_trivia();
                let (y, _, _) = self.number()?;
                spec.slide = (x, y);
            }
            _ => {
                return Err(self.err(self.line, self.col, "unknown motion step"));
            }
        }
        self.expect(')', &format!("')' po `{method}`"))?;
        Ok(())
    }

    /// `behavior:` — `spring(k, c[, m])` albo preset (`snappy`, `bouncy`, `gentle`).
    fn behavior_value(&mut self) -> Result<SeqValue, SeqError> {
        self.skip_trivia();
        let save = (self.pos, self.line, self.col);
        let ident = self.ident();
        let Ok((id, _, _)) = ident else {
            (self.pos, self.line, self.col) = save;
            return self.value();
        };
        if id != "spring" || !self.peek_paren() {
            (self.pos, self.line, self.col) = save;
            return self.value();
        }
        self.bump();
        self.skip_trivia();
        let (stiffness, line, _) = self.number()?;
        self.expect(',', "',' between stiffness and damping")?;
        self.skip_trivia();
        let (damping, _, _) = self.number()?;
        self.skip_trivia();
        let mass = if self.eat(',') {
            self.skip_trivia();
            let (mass, _, _) = self.number()?;
            mass
        } else {
            1.0
        };
        self.expect(')', "')' after spring(stiffness, damping[, mass])")?;
        if !(stiffness.is_finite()
            && damping.is_finite()
            && mass.is_finite()
            && stiffness > 0.0
            && damping > 0.0
            && mass > 0.0)
        {
            return Err(self.err(line, 1, "spring: stiffness/damping/mass must be > 0"));
        }
        Ok(SeqValue::Spring(SpringIr {
            stiffness,
            damping,
            mass,
        }))
    }

    fn motion_time(&mut self, what: &str) -> Result<f32, SeqError> {
        self.skip_trivia();
        let (n, line, _) = self.number()?;
        let secs = if self.keyword_next("ms") {
            self.consume_keyword("ms")?;
            n / 1000.0
        } else if self.keyword_next("s") {
            self.consume_keyword("s")?;
            n
        } else {
            n
        };
        if !secs.is_finite() || secs < 0.0 {
            return Err(self.err(line, 1, format!("{what}: time must be >= 0")));
        }
        Ok(secs)
    }

    fn keyword_next(&mut self, kw: &str) -> bool {
        self.skip_trivia();
        let start = self.pos;
        for (i, ch) in kw.chars().enumerate() {
            if self.chars.get(start + i) != Some(&ch) {
                return false;
            }
        }
        !matches!(
            self.chars.get(start + kw.chars().count()),
            Some(c) if c.is_ascii_alphanumeric() || *c == '_'
        )
    }

    fn consume_keyword(&mut self, kw: &str) -> Result<(), SeqError> {
        let (got, line, col) = self.ident()?;
        if got != kw {
            return Err(self.err(line, col, format!("expected `{kw}`, got `{got}`")));
        }
        Ok(())
    }

    fn peek_paren(&self) -> bool {
        let mut i = self.pos;
        while let Some(&c) = self.chars.get(i) {
            if c == ' ' || c == '\t' || c == '\r' || c == '\n' {
                i += 1;
            } else {
                return c == '(';
            }
        }
        false
    }

    fn value(&mut self) -> Result<SeqValue, SeqError> {
        self.depth += 1;
        if self.depth > MAX_NESTING_DEPTH {
            self.depth -= 1;
            return Err(self.err(
                self.line,
                self.col,
                format!("nesting too deep (limit {MAX_NESTING_DEPTH} levels)"),
            ));
        }
        let result = self.value_inner();
        self.depth -= 1;
        result.map(SeqValue::fold_exprs)
    }

    fn value_inner(&mut self) -> Result<SeqValue, SeqError> {
        self.skip_trivia();
        match self.peek() {
            Some('#') => {
                let (hex, _, _) = self.color()?;
                return Ok(SeqValue::Color(hex));
            }
            Some('{') => return self.callback(),
            _ => {}
        }
        let save = (self.pos, self.line, self.col);
        let starts_paren = self.peek() == Some('(');
        if let Ok(expr) = self.fn_expr()
            && self.value_end()
        {
            let expr = if starts_paren && is_single_value_expr(&expr.kind) {
                SeqExpr {
                    line: expr.line,
                    kind: SeqExprKind::Tuple(vec![expr]),
                }
            } else {
                expr
            };
            return Ok(SeqValue::Expr(expr));
        }
        (self.pos, self.line, self.col) = save;
        self.value_literal()
    }

    fn value_end(&mut self) -> bool {
        self.skip_trivia();
        matches!(
            self.peek(),
            None | Some(',') | Some(')') | Some(']') | Some('}')
        )
    }

    fn value_literal(&mut self) -> Result<SeqValue, SeqError> {
        self.skip_trivia();
        Ok(match self.peek() {
            Some('"') => {
                let (s, _, _) = self.string()?;
                SeqValue::Str(s)
            }
            Some(c) if c == '-' || c.is_ascii_digit() => {
                let (n, _, _) = self.number()?;
                SeqValue::Num(n)
            }
            Some('#') => {
                let (hex, _, _) = self.color()?;
                SeqValue::Color(hex)
            }
            Some('{') => return self.callback(),
            Some('(') => {
                self.bump();
                let mut items = vec![self.value()?];
                while self.eat(',') {
                    self.skip_trivia();
                    if self.peek() == Some(')') {
                        break;
                    }
                    items.push(self.value()?);
                }
                self.expect(')', "')' po krotce `(a, b)`")?;
                SeqValue::Tuple(items)
            }
            Some('[') => {
                self.bump();
                let mut items = vec![self.value()?];
                while self.eat(',') {
                    self.skip_trivia();
                    if self.peek() == Some(']') {
                        break;
                    }
                    items.push(self.value()?);
                }
                self.expect(']', "']' after list `[a, b]`")?;
                SeqValue::Array(items)
            }
            _ => {
                let (id, _, _) = self.ident()?;
                match id.as_str() {
                    "true" => SeqValue::Bool(true),
                    "false" => SeqValue::Bool(false),
                    "fit" => SeqValue::Length(LengthVal::Fit),

                    "fill" if self.peek_paren() => {
                        return self.length_paren(LengthVal::Fill, "fill(waga)");
                    }
                    "px" if self.peek_paren() => {
                        return self.length_paren(LengthVal::Px, "px(liczba)");
                    }
                    "percent" if self.peek_paren() => {
                        return self.length_paren(LengthVal::Percent, "percent(liczba)");
                    }
                    "dynamic" => {
                        self.expect('(', "'(' po `dynamic`")?;
                        let (name, _, _) = self.ident()?;
                        self.expect(')', "')' po `dynamic(nazwa)`")?;
                        SeqValue::Dynamic(name)
                    }
                    "theme" => {
                        self.expect('.', "'.' po `theme` (theme.pole)")?;
                        let (field, _, _) = self.ident()?;
                        SeqValue::Theme(field)
                    }
                    "env" => {
                        self.expect('.', "'.' po `env` (env.nazwa)")?;
                        let (field, _, _) = self.ident()?;
                        SeqValue::Env(field)
                    }
                    "item" => {
                        self.expect('.', "'.' po `item` (item.pole)")?;
                        let (field, _, _) = self.ident()?;
                        SeqValue::ItemField(field)
                    }
                    _ => SeqValue::Ident(id),
                }
            }
        })
    }

    fn length_paren(&mut self, f: fn(f32) -> LengthVal, what: &str) -> Result<SeqValue, SeqError> {
        self.expect('(', format!("'(' — {what}").as_str())?;
        let (n, _, _) = self.number()?;
        self.expect(')', format!("')' — {what}").as_str())?;
        Ok(SeqValue::Length(f(n)))
    }

    fn callback(&mut self) -> Result<SeqValue, SeqError> {
        debug_assert_eq!(self.peek(), Some('{'));
        self.bump();
        self.skip_trivia();
        let (action, _, _) = self.ident()?;
        self.expect('(', "'(' po nazwie callbacku")?;
        let mut args = Vec::new();
        self.skip_trivia();
        if self.peek() != Some(')') {
            args.push(self.value()?);
            while self.eat(',') {
                self.skip_trivia();
                if self.peek() == Some(')') {
                    break;
                }
                args.push(self.value()?);
            }
        }
        self.expect(')', "')' po argumencie callbacku")?;
        self.skip_trivia();
        if self.peek() == Some('}') {
            self.bump();
        } else {
            let (l, c) = (self.line, self.col);
            return Err(self.err(l, c, "'}' po callbacku `{ akcja() }`"));
        }
        Ok(SeqValue::Callback { action, args })
    }
}
