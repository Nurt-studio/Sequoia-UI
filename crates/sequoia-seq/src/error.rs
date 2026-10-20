use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeqError {
    pub file: String,

    pub line: usize,

    pub col: usize,

    pub message: String,
}

impl SeqError {
    pub fn new(
        file: impl Into<String>,
        line: usize,
        col: usize,
        message: impl Into<String>,
    ) -> Self {
        Self {
            file: file.into(),
            line,
            col,
            message: message.into(),
        }
    }
}

impl fmt::Display for SeqError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}: {}",
            self.file, self.line, self.col, self.message
        )
    }
}

impl std::error::Error for SeqError {}
