use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeqError {
    pub file: String,

    pub line: usize,

    pub col: usize,

    pub message: String,

    related: Vec<SeqError>,
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
            related: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_related(mut self, others: Vec<SeqError>) -> Self {
        self.related.extend(others);
        self
    }

    #[must_use]
    pub fn related(&self) -> &[SeqError] {
        &self.related
    }

    #[must_use]
    pub fn flattened(&self) -> Vec<&SeqError> {
        let mut out = vec![self];
        for error in &self.related {
            out.extend(error.flattened());
        }
        out
    }

    pub(crate) fn aggregate(errors: Vec<SeqError>) -> Option<Self> {
        let mut errors = errors.into_iter();
        let first = errors.next()?;
        Some(first.with_related(errors.collect()))
    }
}

impl fmt::Display for SeqError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}: {}",
            self.file, self.line, self.col, self.message
        )?;
        for error in &self.related {
            write!(f, "\n{error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for SeqError {}
