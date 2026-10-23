use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::bindings::{FieldSignal, HostBindings};
use crate::error::SeqError;
use crate::ir::{BinOp, MatchPat, SeqExpr, SeqExprKind, SeqFn, SeqStmt, StateDecl, UnOp};
use crate::value::Value;

const MAX_STEPS: u32 = 100_000;

const MAX_CALL_DEPTH: usize = 64;

pub(crate) struct SeqRuntime {
    file: String,
    fns: RefCell<HashMap<String, SeqFn>>,
    states: RefCell<HashMap<String, Value>>,
}

impl SeqRuntime {
    pub(crate) fn new(file: &str, fns: &[SeqFn], states: &[StateDecl]) -> Self {
        let mut table = HashMap::new();
        for f in fns {
            table.insert(f.name.clone(), f.clone());
        }
        let mut vars = HashMap::new();
        for s in states {
            if let Some(v) = literal_value(&s.init) {
                vars.insert(s.name.clone(), v);
            }
        }
        Self {
            file: file.to_string(),
            fns: RefCell::new(table),
            states: RefCell::new(vars),
        }
    }

    pub(crate) fn file(&self) -> &str {
        &self.file
    }

    pub(crate) fn rebind(&self, fns: &[SeqFn]) {
        let mut table = self.fns.borrow_mut();
        table.clear();
        for f in fns {
            table.insert(f.name.clone(), f.clone());
        }
    }

    pub(crate) fn has_fn(&self, name: &str) -> bool {
        self.fns.borrow().contains_key(name)
    }

    pub(crate) fn call(
        &self,
        b: &HostBindings,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value, SeqError> {
        let mut ev = Eval {
            rt: self,
            b,
            frames: Vec::new(),
            stack: Vec::new(),
            steps: MAX_STEPS,
        };
        ev.call_fn(name, args, 1)
    }
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Tuple(x), Value::Tuple(y)) => {
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(a, b)| values_equal(a, b))
        }
        (Value::Option(x), Value::Option(y)) => match (x, y) {
            (None, None) => true,
            (Some(a), Some(b)) => values_equal(a, b),
            _ => false,
        },
        (Value::Null, Value::Null) => true,
        _ => false,
    }
}
fn literal_value(init: &SeqExpr) -> Option<Value> {
    match &init.kind {
        SeqExprKind::Int(i) => Some(Value::Int(*i)),
        SeqExprKind::Float(f) => Some(Value::Float(*f)),
        SeqExprKind::Str(s) => Some(Value::Str(Rc::new(s.clone()))),
        SeqExprKind::Bool(b) => Some(Value::Bool(*b)),
        SeqExprKind::Var(name) if name == "None" => Some(Value::Option(None)),
        _ => None,
    }
}

struct Local {
    value: Value,
    mutable: bool,
}

enum Flow {
    Continue,
    Return(Value),
}

struct Eval<'a> {
    rt: &'a SeqRuntime,
    b: &'a HostBindings,
    frames: Vec<HashMap<String, Local>>,
    stack: Vec<String>,
    steps: u32,
}

impl<'a> Eval<'a> {
    fn err(&self, line: usize, message: impl Into<String>) -> SeqError {
        SeqError::new(&self.rt.file, line, 1, message)
    }

    fn step(&mut self, line: usize) -> Result<(), SeqError> {
        if self.steps == 0 {
            return Err(self.err(line, "step limit exceeded (endless loop?)"));
        }
        self.steps -= 1;
        Ok(())
    }

    fn call_fn(&mut self, name: &str, args: Vec<Value>, line: usize) -> Result<Value, SeqError> {
        if self.stack.len() >= MAX_CALL_DEPTH {
            return Err(self.err(line, format!("calls too deep (limit {MAX_CALL_DEPTH})")));
        }
        let fns = self.rt.fns.borrow();
        let Some(f) = fns.get(name) else {
            return Err(self.err(line, format!("nieznana funkcja '{name}'")));
        };
        if f.params.len() != args.len() {
            return Err(self.err(
                line,
                format!(
                    "fn {}: expected {} arguments, given {}",
                    f.name,
                    f.params.len(),
                    args.len()
                ),
            ));
        }
        let mut frame = HashMap::new();
        for (p, a) in f.params.iter().zip(args) {
            let v = coerce_arg(self, p, a)?;
            frame.insert(
                p.name.clone(),
                Local {
                    value: v,
                    mutable: p.mutable,
                },
            );
        }
        let body = f.body.clone();
        let ret = f.ret.clone();
        let line_fn = f.line;
        self.frames.push(frame);
        self.stack.push(name.to_string());
        let flow = self.block(&body);
        self.stack.pop();
        self.frames.pop();
        match flow? {
            Flow::Return(v) => Ok(v),
            Flow::Continue => {
                if ret.is_some() {
                    Err(self.err(
                        line_fn,
                        format!("fn {name}: missing `return` at end of path"),
                    ))
                } else {
                    Ok(Value::Null)
                }
            }
        }
    }

    fn block(&mut self, stmts: &[SeqStmt]) -> Result<Flow, SeqError> {
        for stmt in stmts {
            if let Flow::Return(v) = self.stmt(stmt)? {
                return Ok(Flow::Return(v));
            }
        }
        Ok(Flow::Continue)
    }

    fn stmt(&mut self, stmt: &SeqStmt) -> Result<Flow, SeqError> {
        match stmt {
            SeqStmt::Let {
                name,
                mutable,
                value,
                line,
            } => {
                self.step(*line)?;
                let v = self.expr(value)?;
                let frame = self.frames.last_mut().expect("frame");
                frame.insert(
                    name.clone(),
                    Local {
                        value: v,
                        mutable: *mutable,
                    },
                );
                Ok(Flow::Continue)
            }
            SeqStmt::Assign { name, value, line } => {
                self.step(*line)?;
                let v = self.expr(value)?;
                self.assign(name, v, *line)?;
                Ok(Flow::Continue)
            }
            SeqStmt::If {
                cond,
                then,
                elifs,
                els,
                line,
            } => {
                self.step(*line)?;
                let c = self.expr_bool(cond)?;
                if c {
                    return self.block(then);
                }
                for (cond, body) in elifs {
                    if self.expr_bool(cond)? {
                        return self.block(body);
                    }
                }
                match els {
                    Some(body) => self.block(body),
                    None => Ok(Flow::Continue),
                }
            }
            SeqStmt::For {
                var,
                source,
                body,
                line,
            } => {
                self.step(*line)?;
                let Some(rows) = self.b.get_rows(source) else {
                    return Err(self.err(
                        *line,
                        format!("for: unknown row source '{source}' (HostBindings::rows)"),
                    ));
                };
                for row in rows() {
                    let frame = self.frames.last_mut().expect("frame");
                    frame.insert(
                        var.clone(),
                        Local {
                            value: Value::Row(row),
                            mutable: false,
                        },
                    );
                    if let Flow::Return(v) = self.block(body)? {
                        return Ok(Flow::Return(v));
                    }
                }
                Ok(Flow::Continue)
            }
            SeqStmt::Match { value, arms, line } => {
                self.step(*line)?;
                let v = self.expr(value)?;
                for arm in arms {
                    if pattern_matches(&arm.pattern, &v) {
                        return self.block(&arm.body);
                    }
                }
                Err(self.err(*line, format!("match: no branch matches {}", v.type_name())))
            }
            SeqStmt::Return { value, line } => {
                self.step(*line)?;
                let v = match value {
                    Some(e) => self.expr(e)?,
                    None => Value::Null,
                };
                Ok(Flow::Return(v))
            }
            SeqStmt::Expr(e) => {
                self.step(e.line)?;
                self.expr(e)?;
                Ok(Flow::Continue)
            }
        }
    }

    fn assign(&mut self, name: &str, v: Value, line: usize) -> Result<(), SeqError> {
        if let Some(i) = self.frames.iter().rposition(|f| f.contains_key(name)) {
            if !self.frames[i][name].mutable {
                return Err(self.err(
                    line,
                    format!("'{name}' is immutable (use `mut {name} = ...`)"),
                ));
            }
            if !same_kind(&self.frames[i][name].value, &v) {
                return Err(self.err(
                    line,
                    format!(
                        "wrong write type to '{name}' (is {}, given {})",
                        self.frames[i][name].value.type_name(),
                        v.type_name()
                    ),
                ));
            }
            self.frames[i].get_mut(name).expect("local").value = v;
            return Ok(());
        }
        {
            let mut states = self.rt.states.borrow_mut();
            if let Some(slot) = states.get_mut(name) {
                if !same_kind(slot, &v) {
                    return Err(self.err(
                        line,
                        format!(
                            "wrong type writing to state '{name}' (is {}, given {})",
                            slot.type_name(),
                            v.type_name()
                        ),
                    ));
                }
                *slot = v;
                return Ok(());
            }
        }
        if let Some(sig) = self.b.get_str(name) {
            return match &v {
                Value::Str(s) => {
                    sig.set(s.as_ref().clone());
                    Ok(())
                }
                _ => Err(self.err(line, format!("wrong write type to '{name}' (is String)"))),
            };
        }
        if let Some(sig) = self.b.get_bool(name) {
            return match &v {
                Value::Bool(b) => {
                    sig.set(*b);
                    Ok(())
                }
                _ => Err(self.err(line, format!("wrong write type to '{name}' (is Bool)"))),
            };
        }
        if let Some(sig) = self.b.get_int(name) {
            return match &v {
                Value::Int(i) => {
                    sig.set(*i);
                    Ok(())
                }
                _ => Err(self.err(line, format!("wrong write type to '{name}' (it is Int)"))),
            };
        }
        if let Some(sig) = self.b.get_float(name) {
            return match v.as_f64() {
                Some(f) => {
                    sig.set(f);
                    Ok(())
                }
                None => Err(self.err(line, format!("wrong write type to '{name}' (is Float)"))),
            };
        }
        if self.b.get_memo_str(name).is_some()
            || self.b.get_memo_bool(name).is_some()
            || self.b.get_memo_int(name).is_some()
            || self.b.get_memo_float(name).is_some()
        {
            return Err(self.err(
                line,
                format!("'{name}' is memo (derived value) - cannot be written"),
            ));
        }
        Err(self.err(line, format!("nieznana zmienna '{name}'")))
    }

    fn expr_bool(&mut self, e: &SeqExpr) -> Result<bool, SeqError> {
        let v = self.expr(e)?;
        v.as_bool().ok_or_else(|| {
            self.err(
                e.line,
                format!("condition must be Bool (is {})", v.type_name()),
            )
        })
    }

    fn expr(&mut self, e: &SeqExpr) -> Result<Value, SeqError> {
        self.step(e.line)?;
        match &e.kind {
            SeqExprKind::Int(i) => Ok(Value::Int(*i)),
            SeqExprKind::Float(f) => Ok(Value::Float(*f)),
            SeqExprKind::Str(s) => Ok(Value::Str(Rc::new(s.clone()))),
            SeqExprKind::Bool(b) => Ok(Value::Bool(*b)),
            SeqExprKind::Var(name) => {
                if name == "None" {
                    return Ok(Value::Option(None));
                }
                self.read_var(name, e.line)
            }
            SeqExprKind::Field { base, field } => self.read_field(base, field, e.line),
            SeqExprKind::Tuple(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(self.expr(item)?);
                }
                Ok(Value::Tuple(Rc::new(out)))
            }
            SeqExprKind::Unary { op, expr } => {
                let v = self.expr(expr)?;
                match (op, &v) {
                    (UnOp::Neg, Value::Int(i)) => Ok(Value::Int(-i)),
                    (UnOp::Neg, Value::Float(f)) => Ok(Value::Float(-f)),
                    (UnOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
                    (UnOp::Neg, _) => Err(self.err(
                        e.line,
                        format!("`-` wymaga liczby (jest {})", v.type_name()),
                    )),
                    (UnOp::Not, _) => {
                        Err(self.err(e.line, format!("`!` wymaga Bool (jest {})", v.type_name())))
                    }
                }
            }
            SeqExprKind::Binary { op, lhs, rhs } => {
                if matches!(op, BinOp::And) {
                    let l = self.expr_bool(lhs)?;
                    return if l {
                        Ok(Value::Bool(self.expr_bool(rhs)?))
                    } else {
                        Ok(Value::Bool(false))
                    };
                }
                if matches!(op, BinOp::Or) {
                    let l = self.expr_bool(lhs)?;
                    return if l {
                        Ok(Value::Bool(true))
                    } else {
                        Ok(Value::Bool(self.expr_bool(rhs)?))
                    };
                }
                let l = self.expr(lhs)?;
                let r = self.expr(rhs)?;
                self.binary(*op, l, r, e.line)
            }
            SeqExprKind::Call { name, args } => {
                if name == "Some" {
                    if args.len() != 1 {
                        return Err(self.err(e.line, "Some: oczekiwano 1 argumentu"));
                    }
                    let v = self.expr(&args[0])?;
                    return Ok(Value::Option(Some(Box::new(v))));
                }
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(self.expr(a)?);
                }
                self.call_builtin(name, vals, e.line)
            }
        }
    }

    fn read_var(&self, name: &str, line: usize) -> Result<Value, SeqError> {
        for frame in self.frames.iter().rev() {
            if let Some(local) = frame.get(name) {
                return Ok(local.value.clone());
            }
        }
        if let Some(v) = self.rt.states.borrow().get(name) {
            return Ok(v.clone());
        }
        if let Some(sig) = self.b.get_str(name) {
            return Ok(Value::Str(Rc::new(sig.get_untracked())));
        }
        if let Some(sig) = self.b.get_bool(name) {
            return Ok(Value::Bool(sig.get_untracked()));
        }
        if let Some(sig) = self.b.get_int(name) {
            return Ok(Value::Int(sig.get_untracked()));
        }
        if let Some(sig) = self.b.get_float(name) {
            return Ok(Value::Float(sig.get_untracked()));
        }
        if let Some(m) = self.b.get_memo_str(name) {
            return Ok(Value::Str(Rc::new(m.get_untracked())));
        }
        if let Some(m) = self.b.get_memo_bool(name) {
            return Ok(Value::Bool(m.get_untracked()));
        }
        if let Some(m) = self.b.get_memo_int(name) {
            return Ok(Value::Int(m.get_untracked()));
        }
        if let Some(m) = self.b.get_memo_float(name) {
            return Ok(Value::Float(m.get_untracked()));
        }
        Err(self.err(line, format!("nieznana zmienna '{name}'")))
    }

    fn read_field(&self, base: &str, field: &str, line: usize) -> Result<Value, SeqError> {
        for frame in self.frames.iter().rev() {
            if let Some(local) = frame.get(base) {
                match &local.value {
                    Value::Row(row) => {
                        if field == "id" {
                            return Ok(Value::Int(row.id));
                        }
                        let Some(sig) = row.fields.get(field) else {
                            return Err(
                                self.err(line, format!("wiersz '{base}' nie ma pola '{field}'"))
                            );
                        };
                        return Ok(match sig {
                            FieldSignal::Str(s) => Value::Str(Rc::new(s.get_untracked())),
                            FieldSignal::Bool(b) => Value::Bool(b.get_untracked()),
                            FieldSignal::Int(i) => Value::Int(i.get_untracked()),
                            FieldSignal::Float(f) => Value::Float(f.get_untracked()),
                        });
                    }
                    Value::Tuple(items) => {
                        let Ok(i) = field.parse::<usize>() else {
                            return Err(self.err(
                                line,
                                format!("pole krotki '{base}' to `.0`/`.1` (jest '.{field}')"),
                            ));
                        };
                        return items.get(i).cloned().ok_or_else(|| {
                            self.err(
                                line,
                                format!("tuple '{base}' has {} elements (index {i})", items.len()),
                            )
                        });
                    }
                    other => {
                        return Err(self.err(
                            line,
                            format!("'{base}' to {}, nie wiersz", other.type_name()),
                        ));
                    }
                }
            }
        }
        Err(self.err(line, format!("nieznana zmienna '{base}'")))
    }

    #[allow(clippy::too_many_lines)]
    fn binary(&self, op: BinOp, l: Value, r: Value, line: usize) -> Result<Value, SeqError> {
        match op {
            BinOp::Add => match (&l, &r) {
                (Value::Str(a), Value::Str(b)) => Ok(Value::Str(Rc::new(format!("{a}{b}")))),
                _ => self.num_op(op, l, r, line, |a, b| a + b, |a, b| a + b),
            },
            BinOp::Sub => self.num_op(op, l, r, line, |a, b| a - b, |a, b| a - b),
            BinOp::Mul => self.num_op(op, l, r, line, |a, b| a * b, |a, b| a * b),
            _ => self.binary_rest(op, l, r, line),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn binary_rest(&self, op: BinOp, l: Value, r: Value, line: usize) -> Result<Value, SeqError> {
        match op {
            BinOp::Div => {
                if let (Value::Int(a), Value::Int(b)) = (&l, &r) {
                    if *b == 0 {
                        return Err(self.err(line, "dzielenie przez zero"));
                    }
                    return Ok(Value::Int(a / b));
                }
                match (l.as_f64(), r.as_f64()) {
                    (Some(a), Some(b)) => Ok(Value::Float(a / b)),
                    _ => Err(self.err(
                        line,
                        format!(
                            "`/` wymaga liczb (jest {} i {})",
                            l.type_name(),
                            r.type_name()
                        ),
                    )),
                }
            }
            BinOp::Rem => {
                if let (Value::Int(a), Value::Int(b)) = (&l, &r) {
                    if *b == 0 {
                        return Err(self.err(line, "reszta z dzielenia przez zero"));
                    }
                    return Ok(Value::Int(a % b));
                }
                match (l.as_f64(), r.as_f64()) {
                    (Some(a), Some(b)) => Ok(Value::Float(a % b)),
                    _ => Err(self.err(
                        line,
                        format!(
                            "`%` wymaga liczb (jest {} i {})",
                            l.type_name(),
                            r.type_name()
                        ),
                    )),
                }
            }
            BinOp::Eq | BinOp::Ne => {
                let eq = match (&l, &r) {
                    (Value::Str(a), Value::Str(b)) => a == b,
                    (Value::Bool(a), Value::Bool(b)) => a == b,
                    (Value::Option(a), Value::Option(b)) => match (a, b) {
                        (None, None) => true,
                        (Some(x), Some(y)) => values_equal(x, y),
                        _ => false,
                    },
                    _ => match (l.as_f64(), r.as_f64()) {
                        (Some(a), Some(b)) => a == b,
                        _ => false,
                    },
                };
                Ok(Value::Bool(if matches!(op, BinOp::Eq) { eq } else { !eq }))
            }
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                let (a, b) = match (l.as_f64(), r.as_f64()) {
                    (Some(a), Some(b)) => (a, b),
                    _ => {
                        if let (Value::Str(a), Value::Str(b)) = (&l, &r) {
                            let ord = a.cmp(b);
                            return Ok(Value::Bool(match op {
                                BinOp::Lt => ord.is_lt(),
                                BinOp::Le => ord.is_le(),
                                BinOp::Gt => ord.is_gt(),
                                _ => ord.is_ge(),
                            }));
                        }
                        return Err(self.err(
                            line,
                            format!(
                                "comparison requires numbers (there are {} and {})",
                                l.type_name(),
                                r.type_name()
                            ),
                        ));
                    }
                };
                Ok(Value::Bool(match op {
                    BinOp::Lt => a < b,
                    BinOp::Le => a <= b,
                    BinOp::Gt => a > b,
                    _ => a >= b,
                }))
            }
            BinOp::And | BinOp::Or => Err(self.err(line, "`&&`/`||` require Bool values")),
            BinOp::Add | BinOp::Sub | BinOp::Mul => unreachable!("handled in binary()"),
        }
    }

    fn num_op(
        &self,
        op: BinOp,
        l: Value,
        r: Value,
        line: usize,
        int_op: impl Fn(i64, i64) -> i64,
        float_op: impl Fn(f64, f64) -> f64,
    ) -> Result<Value, SeqError> {
        if let (Value::Int(a), Value::Int(b)) = (&l, &r) {
            return Ok(Value::Int(int_op(*a, *b)));
        }
        match (l.as_f64(), r.as_f64()) {
            (Some(a), Some(b)) => Ok(Value::Float(float_op(a, b))),
            _ => Err(self.err(
                line,
                format!(
                    "`{op:?}` wymaga liczb (jest {} i {})",
                    l.type_name(),
                    r.type_name()
                ),
            )),
        }
    }

    fn call_builtin(
        &mut self,
        name: &str,
        args: Vec<Value>,
        line: usize,
    ) -> Result<Value, SeqError> {
        let want = |n: usize| -> Result<(), SeqError> {
            if args.len() == n {
                Ok(())
            } else {
                Err(SeqError::new(
                    &self.rt.file,
                    line,
                    1,
                    format!("{name}: expected {n} arguments, given {}", args.len()),
                ))
            }
        };
        match name {
            "starts_with" => {
                want(2)?;
                let (a, b) = (args[0].as_str(), args[1].as_str());
                match (a, b) {
                    (Some(a), Some(b)) => Ok(Value::Bool(a.starts_with(b))),
                    _ => Err(self.err(line, "starts_with(s, prefix): wymaga String")),
                }
            }
            "strip_prefix" => {
                want(2)?;
                let (a, b) = (args[0].as_str(), args[1].as_str());
                match (a, b) {
                    (Some(a), Some(b)) => Ok(Value::Str(Rc::new(
                        a.strip_prefix(b).unwrap_or("").to_string(),
                    ))),
                    _ => Err(self.err(line, "strip_prefix(s, prefix): wymaga String")),
                }
            }
            "parse_tag" => {
                want(1)?;
                match args[0].as_str() {
                    Some(s) => {
                        let tail = s.rsplit_once(':').map_or(s, |(_, t)| t);
                        Ok(Value::Int(tail.trim().parse::<i64>().unwrap_or(-1)))
                    }
                    None => Err(self.err(line, "parse_tag(s): wymaga String")),
                }
            }
            "len" => {
                want(1)?;
                match args[0].as_str() {
                    Some(s) => Ok(Value::Int(s.chars().count() as i64)),
                    None => Err(self.err(line, "len(s): wymaga String")),
                }
            }
            "clamp" => {
                want(3)?;
                let (v, lo, hi) = (args[0].as_f64(), args[1].as_f64(), args[2].as_f64());
                match (v, lo, hi) {
                    (Some(v), Some(lo), Some(hi)) => Ok(Value::Float(v.max(lo).min(hi))),
                    _ => Err(self.err(line, "clamp(v, min, max): wymaga liczb")),
                }
            }
            "powf" => {
                want(2)?;
                match (args[0].as_f64(), args[1].as_f64()) {
                    (Some(a), Some(b)) => Ok(Value::Float(a.powf(b))),
                    _ => Err(self.err(line, "powf(x, y): wymaga liczb")),
                }
            }
            "max" | "min" => {
                want(2)?;
                if let (Value::Int(a), Value::Int(b)) = (&args[0], &args[1]) {
                    return Ok(Value::Int(if name == "max" {
                        (*a).max(*b)
                    } else {
                        (*a).min(*b)
                    }));
                }
                match (args[0].as_f64(), args[1].as_f64()) {
                    (Some(a), Some(b)) => Ok(Value::Float(if name == "max" {
                        a.max(b)
                    } else {
                        a.min(b)
                    })),
                    _ => Err(self.err(line, format!("{name}(a, b): wymaga liczb"))),
                }
            }
            _ => {
                if self.rt.has_fn(name) {
                    self.call_fn(name, args, line)
                } else {
                    Err(self.err(line, format!("nieznana funkcja '{name}'")))
                }
            }
        }
    }
}

fn pattern_matches(pat: &MatchPat, v: &Value) -> bool {
    match (pat, v) {
        (MatchPat::Str(p), Value::Str(s)) => p.as_str() == s.as_str(),
        (MatchPat::Int(p), Value::Int(i)) => p == i,
        (MatchPat::Bool(p), Value::Bool(b)) => p == b,
        (MatchPat::Wildcard, _) => true,
        _ => false,
    }
}

fn same_kind(slot: &Value, v: &Value) -> bool {
    matches!(
        (slot, v),
        (Value::Int(_), Value::Int(_))
            | (Value::Float(_), Value::Float(_))
            | (Value::Float(_), Value::Int(_))
            | (Value::Str(_), Value::Str(_))
            | (Value::Bool(_), Value::Bool(_))
    )
}

fn coerce_arg(ev: &Eval<'_>, p: &crate::ir::FnParam, v: Value) -> Result<Value, SeqError> {
    let ok = match p.ty.as_str() {
        "Int" => matches!(v, Value::Int(_)),
        "Float" => v.is_number(),
        "String" => matches!(v, Value::Str(_)),
        "Bool" => matches!(v, Value::Bool(_)),
        t if t.starts_with("Option") => matches!(v, Value::Option(_)),
        t if t.starts_with('(') => matches!(v, Value::Tuple(_)),
        _ => false,
    };
    if !ok {
        return Err(ev.err(
            p.line,
            format!(
                "argument '{}': oczekiwano {}, dano {}",
                p.name,
                p.ty,
                v.type_name()
            ),
        ));
    }
    Ok(match (&p.ty[..], v) {
        ("Float", Value::Int(i)) => Value::Float(i as f64),
        (_, v) => v,
    })
}
