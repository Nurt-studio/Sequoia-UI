#![forbid(unsafe_code)]

mod attr;
mod cursor;
mod lines;
mod piece;
mod styled;
mod undo;

pub use attr::{
    SpanAttr, StyledSpan, hash_span_attr, hash_spans_rebased, line_hash, normalize_spans,
    rebase_spans, spans_hash,
};
pub use cursor::{
    Cursor, Selection, line_end, line_home, move_lines, next_grapheme, prev_grapheme,
    word_end_after, word_start_before,
};
pub use lines::LineIndex;
pub use piece::TextBuffer;
pub use styled::StyledBuffer;
pub use undo::{Edit, UndoGroup, UndoStack};
