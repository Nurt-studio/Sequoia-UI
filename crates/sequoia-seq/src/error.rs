use std::fmt;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Warning,

    Error,
}

impl Severity {
    #[must_use]
    pub const fn as_lsp(self) -> u32 {
        match self {
            Self::Warning => 2,
            Self::Error => 1,
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Warning => f.write_str("warning"),
            Self::Error => f.write_str("error"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeqError {
    pub file: String,

    pub line: usize,

    pub col: usize,

    pub message: String,

    severity: Severity,

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
            severity: Severity::Error,
            related: Vec::new(),
        }
    }

    #[must_use]
    pub fn warning(
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
            severity: Severity::Warning,
            related: Vec::new(),
        }
    }

    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }

    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self.severity, Severity::Error)
    }

    #[must_use]
    pub const fn is_warning(&self) -> bool {
        matches!(self.severity, Severity::Warning)
    }

    #[must_use]
    pub fn with_related(mut self, others: Vec<SeqError>) -> Self {
        self.related.extend(others);
        if self.related.iter().any(SeqError::is_error) {
            self.severity = Severity::Error;
        }
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
        let mut errors = errors.into_iter().filter(SeqError::is_error);
        let first = errors.next()?;
        Some(first.with_related(errors.collect()))
    }
}

impl fmt::Display for SeqError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_warning() {
            write!(f, "warning: ")?;
        }
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
