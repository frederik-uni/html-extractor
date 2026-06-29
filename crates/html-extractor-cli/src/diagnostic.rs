use std::path::Path;

use html_extractor_core::{SourceText, Span};

pub(crate) fn render(
    path: &Path,
    source: &str,
    severity: &str,
    message: &str,
    span: Option<Span>,
) -> String {
    if let Some(location) = span.and_then(|span| SourceText::from(source).location(span.start())) {
        format!(
            "{}:{}:{}: {severity}: {message}",
            path.display(),
            location.line(),
            location.column()
        )
    } else {
        format!("{}: {severity}: {message}", path.display())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use html_extractor_core::Span;

    use super::render;

    #[test]
    fn renders_one_based_source_location() {
        let rendered = render(
            Path::new("sample.extractor"),
            "first = 1\nsecond = 2",
            "error",
            "bad value",
            Some(Span::new(10, 17)),
        );

        assert_eq!(rendered, "sample.extractor:2:1: error: bad value");
    }

    #[test]
    fn renders_spanless_diagnostic_at_file_level() {
        let rendered = render(Path::new("sample.extractor"), "", "error", "failed", None);

        assert_eq!(rendered, "sample.extractor: error: failed");
    }

    #[test]
    fn invalid_offsets_fall_back_to_file_level() {
        let rendered = render(
            Path::new("sample.extractor"),
            "short",
            "warning",
            "failed",
            Some(Span::new(100, 101)),
        );

        assert_eq!(rendered, "sample.extractor: warning: failed");
    }
}
