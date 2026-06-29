use std::{error::Error, fmt};

use crate::Span;

/// A stable, source-located compiler diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    message: String,
    primary_span: Span,
}

impl Diagnostic {
    #[must_use]
    pub fn new(message: impl Into<String>, primary_span: Span) -> Self {
        Self {
            message: message.into(),
            primary_span,
        }
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    #[must_use]
    pub const fn primary_span(&self) -> Span {
        self.primary_span
    }
}

/// Compilation failure containing one or more stable diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompileError {
    diagnostics: Vec<Diagnostic>,
}

impl CompileError {
    #[must_use]
    pub fn new(diagnostic: Diagnostic) -> Self {
        Self {
            diagnostics: vec![diagnostic],
        }
    }

    #[must_use]
    pub fn from_diagnostics(diagnostics: Vec<Diagnostic>) -> Self {
        assert!(
            !diagnostics.is_empty(),
            "compile errors require a diagnostic"
        );
        Self { diagnostics }
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.diagnostics[0].message())
    }
}

impl Error for CompileError {}
