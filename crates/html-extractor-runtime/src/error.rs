use std::{error::Error, fmt};

use html_extractor_core::Span;

use crate::ValueKind;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeError {
    expected: &'static str,
    actual: ValueKind,
}

impl TypeError {
    pub(crate) const fn new(expected: &'static str, actual: ValueKind) -> Self {
        Self { expected, actual }
    }
}

impl fmt::Display for TypeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "expected {}, got {}", self.expected, self.actual)
    }
}

impl Error for TypeError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SerializationError {
    path: String,
    kind: ValueKind,
}

impl SerializationError {
    pub(crate) fn unsupported(path: String, kind: ValueKind) -> Self {
        Self { path, kind }
    }

    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub const fn kind(&self) -> ValueKind {
        self.kind
    }
}

impl fmt::Display for SerializationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "cannot serialize {} at {}", self.kind, self.path)
    }
}

impl Error for SerializationError {}

#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    message: String,
    span: Option<Span>,
}

impl Diagnostic {
    pub(crate) fn new(message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    #[must_use]
    pub const fn span(&self) -> Option<Span> {
        self.span
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExecutionError {
    message: String,
    span: Option<Span>,
}

impl ExecutionError {
    pub fn new(message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    #[must_use]
    pub const fn span(&self) -> Option<Span> {
        self.span
    }
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ExecutionError {}

impl From<TypeError> for ExecutionError {
    fn from(error: TypeError) -> Self {
        Self::new(error.to_string(), None)
    }
}
