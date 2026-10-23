#[derive(Clone, Debug)]
pub struct SeqApp {
    pub title: String,

    pub size: (f32, f32),

    pub theme: ThemeName,

    pub windows: Vec<SeqWindow>,

    pub deferred: Vec<DeferredDecl>,

    pub styles: Vec<StyleDecl>,

    pub components: Vec<ComponentDef>,

    pub states: Vec<StateDecl>,

    pub fns: Vec<SeqFn>,

    pub imports: Vec<ImportDecl>,

    pub app_line: usize,

    pub has_title: bool,

    pub has_size: bool,
}

#[derive(Clone, Debug)]
pub struct SeqFile {
    pub imports: Vec<ImportDecl>,

    pub components: Vec<ComponentDef>,

    pub app: Option<SeqApp>,
}

#[derive(Clone, Debug)]
pub struct ImportDecl {
    pub path: String,

    pub line: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    Pub,

    Priv,
}

#[derive(Clone, Debug)]
pub struct DeferredDecl {
    pub kind: DeferredKind,

    pub name: String,

    pub line: usize,

    pub sig: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeferredKind {
    Prop,

    On,

    Style,
}

#[derive(Clone, Debug)]
pub struct StyleDecl {
    pub name: String,

    pub line: usize,

    pub target: String,

    pub base: Option<String>,

    pub args: Vec<SeqArg>,
}

#[derive(Clone, Debug)]
pub struct CompParam {
    pub name: String,

    pub line: usize,

    pub ty: String,
}

#[derive(Clone, Debug)]
pub struct ComponentDef {
    pub name: String,

    pub line: usize,

    pub params: Vec<CompParam>,

    pub body: Vec<SeqNode>,

    pub visibility: Visibility,

    pub file: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThemeName {
    Dark,
    Light,

    File(String),
}

#[derive(Clone, Debug)]
pub struct SeqWindow {
    pub name: String,

    pub line: usize,

    pub size: Option<(f32, f32)>,

    pub nodes: Vec<SeqNode>,
}

#[derive(Clone, Debug)]
pub struct SeqNode {
    pub kind: String,

    pub line: usize,

    pub label: Option<String>,

    pub args: Vec<SeqArg>,

    pub children: Vec<SeqNode>,
}

#[derive(Clone, Debug)]
pub struct SeqArg {
    pub name: String,

    pub line: usize,

    pub value: SeqValue,
}

#[derive(Clone, Debug)]
pub enum SeqValue {
    Str(String),

    Num(f32),

    Bool(bool),

    Color(String),

    Theme(String),

    Length(LengthVal),

    Ident(String),

    Callback { action: String, args: Vec<SeqValue> },

    CallSeq { name: String, args: Vec<SeqValue> },

    Dynamic(String),

    Env(String),

    ItemField(String),

    Tuple(Vec<SeqValue>),

    Array(Vec<SeqValue>),
}

#[derive(Clone, Copy, Debug)]
pub enum LengthVal {
    Fill(f32),

    Px(f32),

    Fit,

    Percent(f32),
}

#[derive(Clone, Debug)]
pub struct StateDecl {
    pub name: String,

    pub line: usize,

    pub ty: String,

    pub init: SeqExpr,
}

#[derive(Clone, Debug)]
pub struct FnParam {
    pub name: String,

    pub mutable: bool,

    pub line: usize,

    pub ty: String,
}

#[derive(Clone, Debug)]
pub struct SeqFn {
    pub name: String,

    pub line: usize,

    pub params: Vec<FnParam>,

    pub ret: Option<String>,

    pub body: Vec<SeqStmt>,
}

#[derive(Clone, Debug)]
pub enum SeqStmt {
    Let {
        name: String,

        mutable: bool,

        value: SeqExpr,

        line: usize,
    },

    Assign {
        name: String,

        value: SeqExpr,

        line: usize,
    },

    If {
        cond: SeqExpr,

        then: Vec<SeqStmt>,

        elifs: Vec<(SeqExpr, Vec<SeqStmt>)>,

        els: Option<Vec<SeqStmt>>,

        line: usize,
    },

    For {
        var: String,

        source: String,

        body: Vec<SeqStmt>,

        line: usize,
    },

    Match {
        value: SeqExpr,

        arms: Vec<SeqMatchArm>,

        line: usize,
    },

    Return {
        value: Option<SeqExpr>,

        line: usize,
    },

    Expr(SeqExpr),
}

#[derive(Clone, Debug)]
pub struct SeqMatchArm {
    pub pattern: MatchPat,

    pub body: Vec<SeqStmt>,

    pub line: usize,
}

#[derive(Clone, Debug)]
pub enum MatchPat {
    Str(String),

    Int(i64),

    Bool(bool),

    Wildcard,
}

#[derive(Clone, Debug)]
pub struct SeqExpr {
    pub line: usize,

    pub kind: SeqExprKind,
}

#[derive(Clone, Debug)]
pub enum SeqExprKind {
    Int(i64),

    Float(f64),

    Str(String),

    Bool(bool),

    Var(String),

    Field {
        base: String,

        field: String,
    },

    Tuple(Vec<SeqExpr>),

    Unary {
        op: UnOp,

        expr: Box<SeqExpr>,
    },

    Binary {
        op: BinOp,

        lhs: Box<SeqExpr>,

        rhs: Box<SeqExpr>,
    },

    Call {
        name: String,

        args: Vec<SeqExpr>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    Neg,

    Not,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,

    Sub,

    Mul,

    Div,

    Rem,

    Eq,

    Ne,

    Lt,

    Le,

    Gt,

    Ge,

    And,

    Or,
}

#[must_use]
pub fn resolve_seq_calls(mut app: SeqApp) -> SeqApp {
    let fns: std::collections::HashSet<&str> = app.fns.iter().map(|f| f.name.as_str()).collect();
    if fns.is_empty() {
        return app;
    }
    for style in &mut app.styles {
        for arg in &mut style.args {
            resolve_value(&fns, &mut arg.value);
        }
    }
    for window in &mut app.windows {
        for node in &mut window.nodes {
            resolve_node(&fns, node);
        }
    }
    for comp in &mut app.components {
        for node in &mut comp.body {
            resolve_node(&fns, node);
        }
    }
    app
}

fn resolve_node(fns: &std::collections::HashSet<&str>, node: &mut SeqNode) {
    for arg in &mut node.args {
        resolve_value(fns, &mut arg.value);
    }
    for child in &mut node.children {
        resolve_node(fns, child);
    }
}

fn resolve_value(fns: &std::collections::HashSet<&str>, value: &mut SeqValue) {
    if let SeqValue::Callback { action, args } = value
        && fns.contains(action.as_str())
    {
        let name = std::mem::take(action);
        let mut taken = Vec::new();
        std::mem::swap(args, &mut taken);
        *value = SeqValue::CallSeq { name, args: taken };
    }
    match value {
        SeqValue::Callback { args, .. } | SeqValue::CallSeq { args, .. } => {
            for arg in args {
                resolve_value(fns, arg);
            }
        }
        SeqValue::Tuple(items) | SeqValue::Array(items) => {
            for item in items {
                resolve_value(fns, item);
            }
        }
        _ => {}
    }
}
