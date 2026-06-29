use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// A half-open byte range into retained extractor source.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Span {
    start: usize,
    end: usize,
}

impl Span {
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    #[must_use]
    pub const fn start(self) -> usize {
        self.start
    }

    #[must_use]
    pub const fn end(self) -> usize {
        self.end
    }
}

/// A one-based source position suitable for user-facing diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceLocation {
    line: usize,
    column: usize,
}

impl SourceLocation {
    #[must_use]
    pub const fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }

    #[must_use]
    pub const fn line(self) -> usize {
        self.line
    }

    #[must_use]
    pub const fn column(self) -> usize {
        self.column
    }
}

/// Immutable, cheaply cloneable extractor source retained by compiled programs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceText {
    text: Arc<str>,
}

impl SourceText {
    #[must_use]
    pub fn new(source: impl Into<Arc<str>>) -> Self {
        Self {
            text: source.into(),
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub fn slice(&self, span: Span) -> Option<&str> {
        if span.start > span.end {
            return None;
        }
        self.text.get(span.start..span.end)
    }

    #[must_use]
    pub fn location(&self, offset: usize) -> Option<SourceLocation> {
        if !self.text.is_char_boundary(offset) {
            return None;
        }

        let prefix = self.text.get(..offset)?;
        let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
        let column = self.text[line_start..offset].chars().count() + 1;
        Some(SourceLocation::new(line, column))
    }
}

impl From<&str> for SourceText {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}
