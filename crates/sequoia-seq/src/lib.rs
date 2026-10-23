#![forbid(unsafe_code)]

mod bindings;
mod check;
mod error;
mod eval;
mod ir;
mod loader;
mod lower;
mod parser;
mod resolve;
mod value;

pub use bindings::{ActionSig, FieldSignal, HostBindings, PropType, SeqRow, SeqRowData};
pub use check::check;
pub use error::SeqError;
pub use ir::{
    BinOp, CompParam, ComponentDef, DeferredDecl, DeferredKind, FnParam, ImportDecl, LengthVal,
    MatchPat, SeqApp, SeqArg, SeqExpr, SeqExprKind, SeqFile, SeqFn, SeqMatchArm, SeqNode, SeqStmt,
    SeqValue, SeqWindow, StateDecl, StyleDecl, ThemeName, UnOp, Visibility, resolve_seq_calls,
};
pub use loader::{AppMeta, LoadedApp, SeqLoader};
pub use lower::{
    BuiltStyles, SeqTextStyle, TextWeight, build_styles, check_decls, expand_components, lower,
    lower_window, lower_window_with, lower_with, resolve_theme,
};
pub use parser::{parse, parse_seq_file};
pub use resolve::{FsSource, MapSource, ResolvedImports, SeqSource, resolve_imports};
