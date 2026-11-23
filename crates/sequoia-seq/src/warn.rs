use std::collections::HashSet;

use crate::error::SeqError;
use crate::ir::{
    DeferredKind, SeqApp, SeqArg, SeqExpr, SeqExprKind, SeqNode, SeqStmt, SeqValue, Visibility,
};

#[derive(Default)]
struct Used {
    fns: HashSet<String>,
    components: HashSet<String>,
    styles: HashSet<String>,
    states: HashSet<String>,
}

pub(crate) fn unused_diagnostics(file: &str, app: &SeqApp) -> Vec<SeqError> {
    let used = collect_used(app);
    let mut out = Vec::new();
    for f in &app.fns {
        if !used.fns.contains(&f.name) {
            out.push(SeqError::warning(
                file,
                f.line,
                1,
                format!("function '{}' is never used", f.name),
            ));
        }
    }
    for c in &app.components {
        if c.visibility == Visibility::Priv && !used.components.contains(&c.name) {
            out.push(SeqError::warning(
                &c.file,
                c.line,
                1,
                format!("component '{}' is never used", c.name),
            ));
        }
    }
    for s in &app.styles {
        if !used.styles.contains(&s.name) {
            out.push(SeqError::warning(
                file,
                s.line,
                1,
                format!("style '{}' is never used", s.name),
            ));
        }
    }
    for s in &app.states {
        if !used.states.contains(&s.name) {
            out.push(SeqError::warning(
                file,
                s.line,
                1,
                format!("state '{}' is never used", s.name),
            ));
        }
    }
    out
}

fn collect_used(app: &SeqApp) -> Used {
    let mut used = Used::default();
    for w in &app.windows {
        collect_nodes(&w.nodes, &mut used);
    }
    for c in &app.components {
        collect_nodes(&c.body, &mut used);
        for m in &c.members {
            if let Some(init) = &m.init {
                collect_expr(init, &mut used);
            }
        }
    }
    for s in &app.styles {
        if let Some(base) = &s.base {
            used.styles.insert(base.clone());
        }
        for arg in &s.args {
            collect_arg(arg, &mut used);
        }
    }
    for f in &app.fns {
        collect_stmts(&f.body, &mut used);
    }
    for s in &app.states {
        collect_expr(&s.init, &mut used);
    }
    for d in &app.deferred {
        if d.kind == DeferredKind::On {
            used.fns.insert(d.name.clone());
        }
    }
    used
}

fn collect_nodes(nodes: &[SeqNode], used: &mut Used) {
    for node in nodes {
        used.components.insert(node.kind.clone());
        for arg in &node.args {
            collect_arg(arg, used);
        }
        collect_nodes(&node.children, used);
    }
}

fn collect_arg(arg: &SeqArg, used: &mut Used) {
    if arg.name == "style" {
        match &arg.value {
            SeqValue::Ident(name) | SeqValue::Str(name) => {
                used.styles.insert(name.clone());
            }
            SeqValue::Expr(expr) => {
                if let SeqExprKind::Str(name) = &expr.kind {
                    used.styles.insert(name.clone());
                }
            }
            _ => {}
        }
    }
    collect_value(&arg.value, used);
}

fn collect_value(value: &SeqValue, used: &mut Used) {
    match value {
        SeqValue::Callback { action, args } | SeqValue::CallSeq { name: action, args } => {
            used.fns.insert(action.clone());
            for arg in args {
                collect_value(arg, used);
            }
        }
        SeqValue::Tuple(values) | SeqValue::Array(values) => {
            for value in values {
                collect_value(value, used);
            }
        }
        SeqValue::Expr(expr) => collect_expr(expr, used),
        _ => {}
    }
}

fn collect_expr(expr: &SeqExpr, used: &mut Used) {
    match &expr.kind {
        SeqExprKind::Var(name) => {
            used.states.insert(name.clone());
        }
        SeqExprKind::Field { base, .. } => {
            used.states.insert(base.clone());
        }
        SeqExprKind::Tuple(items) | SeqExprKind::Array(items) => {
            for item in items {
                collect_expr(item, used);
            }
        }
        SeqExprKind::Unary { expr, .. } => collect_expr(expr, used),
        SeqExprKind::Binary { lhs, rhs, .. } => {
            collect_expr(lhs, used);
            collect_expr(rhs, used);
        }
        SeqExprKind::Call { name, args } => {
            used.fns.insert(name.clone());
            for arg in args {
                collect_expr(arg, used);
            }
        }
        SeqExprKind::Ternary { cond, then, els } => {
            collect_expr(cond, used);
            collect_expr(then, used);
            collect_expr(els, used);
        }
        _ => {}
    }
}

fn collect_stmts(stmts: &[SeqStmt], used: &mut Used) {
    for stmt in stmts {
        match stmt {
            SeqStmt::Let { value, .. } => collect_expr(value, used),
            SeqStmt::Assign { name, value, .. } => {
                used.states.insert(name.clone());
                collect_expr(value, used);
            }
            SeqStmt::If {
                cond,
                then,
                elifs,
                els,
                ..
            } => {
                collect_expr(cond, used);
                collect_stmts(then, used);
                for (cond, body) in elifs {
                    collect_expr(cond, used);
                    collect_stmts(body, used);
                }
                if let Some(body) = els {
                    collect_stmts(body, used);
                }
            }
            SeqStmt::For { body, .. } => collect_stmts(body, used),
            SeqStmt::Match { value, arms, .. } => {
                collect_expr(value, used);
                for arm in arms {
                    collect_stmts(&arm.body, used);
                }
            }
            SeqStmt::Return { value, .. } => {
                if let Some(value) = value {
                    collect_expr(value, used);
                }
            }
            SeqStmt::Expr(expr) => collect_expr(expr, used),
        }
    }
}
