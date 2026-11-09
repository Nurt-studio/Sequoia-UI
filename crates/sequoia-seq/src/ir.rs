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

    Expr(SeqExpr),
}

impl SeqValue {
    pub(crate) fn fold_exprs(self) -> Self {
        match self {
            Self::Expr(expr) => match evaluate_seq_expr(&expr) {
                Ok(value) => value.fold_exprs(),
                Err(_) => Self::Expr(expr),
            },
            Self::Tuple(items) => Self::Tuple(items.into_iter().map(Self::fold_exprs).collect()),
            Self::Array(items) => Self::Array(items.into_iter().map(Self::fold_exprs).collect()),
            Self::Callback { action, args } => Self::Callback {
                action,
                args: args.into_iter().map(Self::fold_exprs).collect(),
            },
            Self::CallSeq { name, args } => Self::CallSeq {
                name,
                args: args.into_iter().map(Self::fold_exprs).collect(),
            },
            other => other,
        }
    }
}

pub(crate) fn evaluate_seq_expr(expr: &SeqExpr) -> Result<SeqValue, String> {
    match &expr.kind {
        SeqExprKind::Int(i) => Ok(SeqValue::Num(*i as f32)),
        SeqExprKind::Float(f) => {
            #[allow(clippy::cast_possible_truncation)]
            let value = *f as f32;
            if value.is_finite() {
                Ok(SeqValue::Num(value))
            } else {
                Err("non-finite float".to_string())
            }
        }
        SeqExprKind::Str(s) => Ok(SeqValue::Str(s.clone())),
        SeqExprKind::Bool(b) => Ok(SeqValue::Bool(*b)),
        SeqExprKind::Tuple(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(evaluate_seq_expr(item)?);
            }
            Ok(SeqValue::Tuple(out))
        }
        SeqExprKind::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(evaluate_seq_expr(item)?);
            }
            Ok(SeqValue::Array(out))
        }
        SeqExprKind::Unary { op, expr } => {
            let v = evaluate_seq_expr(expr)?;
            match (op, &v) {
                (UnOp::Neg, SeqValue::Num(n)) => Ok(SeqValue::Num(-n)),
                (UnOp::Not, SeqValue::Bool(b)) => Ok(SeqValue::Bool(!b)),
                _ => Err("invalid unary operation".to_string()),
            }
        }
        SeqExprKind::Binary { op, lhs, rhs } => {
            let l = evaluate_seq_expr(lhs)?;
            let r = evaluate_seq_expr(rhs)?;
            match (op, &l, &r) {
                (BinOp::Add, SeqValue::Str(a), SeqValue::Str(b)) => {
                    Ok(SeqValue::Str(format!("{a}{b}")))
                }
                (BinOp::Add, SeqValue::Num(a), SeqValue::Num(b)) => Ok(SeqValue::Num(a + b)),
                (BinOp::Sub, SeqValue::Num(a), SeqValue::Num(b)) => Ok(SeqValue::Num(a - b)),
                (BinOp::Mul, SeqValue::Num(a), SeqValue::Num(b)) => Ok(SeqValue::Num(a * b)),
                (BinOp::Div, SeqValue::Num(a), SeqValue::Num(b)) => {
                    if *b == 0.0 {
                        Err("division by zero".to_string())
                    } else {
                        Ok(SeqValue::Num(a / b))
                    }
                }
                (BinOp::Rem, SeqValue::Num(a), SeqValue::Num(b)) => {
                    if *b == 0.0 {
                        Err("remainder by zero".to_string())
                    } else {
                        Ok(SeqValue::Num(a % b))
                    }
                }
                (BinOp::Eq, _, _) => Ok(SeqValue::Bool(values_eq(&l, &r))),
                (BinOp::Ne, _, _) => Ok(SeqValue::Bool(!values_eq(&l, &r))),
                (BinOp::Lt, SeqValue::Num(a), SeqValue::Num(b)) => Ok(SeqValue::Bool(a < b)),
                (BinOp::Le, SeqValue::Num(a), SeqValue::Num(b)) => Ok(SeqValue::Bool(a <= b)),
                (BinOp::Gt, SeqValue::Num(a), SeqValue::Num(b)) => Ok(SeqValue::Bool(a > b)),
                (BinOp::Ge, SeqValue::Num(a), SeqValue::Num(b)) => Ok(SeqValue::Bool(a >= b)),
                (BinOp::And, SeqValue::Bool(a), SeqValue::Bool(b)) => Ok(SeqValue::Bool(*a && *b)),
                (BinOp::Or, SeqValue::Bool(a), SeqValue::Bool(b)) => Ok(SeqValue::Bool(*a || *b)),
                _ => Err("invalid binary operation".to_string()),
            }
        }
        SeqExprKind::Ternary { cond, then, els } => {
            let c = evaluate_seq_expr(cond)?;
            match c {
                SeqValue::Bool(true) => evaluate_seq_expr(then),
                SeqValue::Bool(false) => evaluate_seq_expr(els),
                _ => Err("ternary condition must be Bool".to_string()),
            }
        }
        SeqExprKind::Var(name) => match name.as_str() {
            "true" => Ok(SeqValue::Bool(true)),
            "false" => Ok(SeqValue::Bool(false)),
            "fit" => Ok(SeqValue::Length(LengthVal::Fit)),
            _ => Ok(SeqValue::Ident(name.clone())),
        },
        SeqExprKind::Field { base, field } => match base.as_str() {
            "env" => Ok(SeqValue::Env(field.clone())),
            "theme" => Ok(SeqValue::Theme(field.clone())),
            "item" => Ok(SeqValue::ItemField(field.clone())),
            _ => Ok(SeqValue::Ident(format!("{base}.{field}"))),
        },
        SeqExprKind::Call { name, args } => match name.as_str() {
            "fill" if args.len() == 1 => {
                if let SeqValue::Num(n) = evaluate_seq_expr(&args[0])? {
                    Ok(SeqValue::Length(LengthVal::Fill(n)))
                } else {
                    Err("fill requires number".to_string())
                }
            }
            "px" if args.len() == 1 => {
                if let SeqValue::Num(n) = evaluate_seq_expr(&args[0])? {
                    Ok(SeqValue::Length(LengthVal::Px(n)))
                } else {
                    Err("px requires number".to_string())
                }
            }
            "percent" if args.len() == 1 => {
                if let SeqValue::Num(n) = evaluate_seq_expr(&args[0])? {
                    Ok(SeqValue::Length(LengthVal::Percent(n)))
                } else {
                    Err("percent requires number".to_string())
                }
            }
            "dynamic" if args.len() == 1 => match evaluate_seq_expr(&args[0])? {
                SeqValue::Ident(s) | SeqValue::Str(s) => Ok(SeqValue::Dynamic(s)),
                _ => Err("dynamic requires string or ident".to_string()),
            },
            "min" | "max" if args.len() == 2 => {
                let a = evaluate_seq_expr(&args[0])?;
                let b = evaluate_seq_expr(&args[1])?;
                match (&a, &b) {
                    (SeqValue::Num(x), SeqValue::Num(y)) => {
                        let val = if name == "max" { x.max(*y) } else { x.min(*y) };
                        Ok(SeqValue::Num(val))
                    }
                    _ => Err(format!("{name} requires numbers")),
                }
            }
            "clamp" if args.len() == 3 => {
                let v = evaluate_seq_expr(&args[0])?;
                let lo = evaluate_seq_expr(&args[1])?;
                let hi = evaluate_seq_expr(&args[2])?;
                match (&v, &lo, &hi) {
                    (SeqValue::Num(v), SeqValue::Num(lo), SeqValue::Num(hi)) => {
                        Ok(SeqValue::Num((*v).max(*lo).min(*hi)))
                    }
                    _ => Err("clamp requires numbers".to_string()),
                }
            }
            "powf" if args.len() == 2 => {
                let a = evaluate_seq_expr(&args[0])?;
                let b = evaluate_seq_expr(&args[1])?;
                match (&a, &b) {
                    (SeqValue::Num(x), SeqValue::Num(y)) => Ok(SeqValue::Num(x.powf(*y))),
                    _ => Err("powf requires numbers".to_string()),
                }
            }
            "len" if args.len() == 1 => {
                let a = evaluate_seq_expr(&args[0])?;
                match a {
                    SeqValue::Str(s) => Ok(SeqValue::Num(s.chars().count() as f32)),
                    _ => Err("len requires string".to_string()),
                }
            }
            _ => {
                let mut evaluated_args = Vec::with_capacity(args.len());
                for arg in args {
                    evaluated_args.push(evaluate_seq_expr(arg)?);
                }
                Ok(SeqValue::CallSeq {
                    name: name.clone(),
                    args: evaluated_args,
                })
            }
        },
    }
}

fn values_eq(a: &SeqValue, b: &SeqValue) -> bool {
    match (a, b) {
        (SeqValue::Num(x), SeqValue::Num(y)) => x == y,
        (SeqValue::Str(x), SeqValue::Str(y)) => x == y,
        (SeqValue::Bool(x), SeqValue::Bool(y)) => x == y,
        _ => false,
    }
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

    Array(Vec<SeqExpr>),

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

    Ternary {
        cond: Box<SeqExpr>,

        then: Box<SeqExpr>,

        els: Box<SeqExpr>,
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
        SeqValue::Expr(_) => {}
        _ => {}
    }
}
