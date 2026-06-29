use std::sync::Arc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HtmlDocument {
    source: Arc<str>,
}

impl HtmlDocument {
    #[must_use]
    pub fn parse(source: impl Into<Arc<str>>) -> Self {
        Self {
            source: source.into(),
        }
    }

    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HtmlNode {
    fragment: Arc<str>,
    root_tag: Arc<str>,
}

impl HtmlNode {
    pub(crate) fn new(fragment: impl Into<Arc<str>>, root_tag: impl Into<Arc<str>>) -> Self {
        Self {
            fragment: fragment.into(),
            root_tag: root_tag.into(),
        }
    }

    #[must_use]
    pub fn fragment(&self) -> &str {
        &self.fragment
    }

    pub(crate) fn root_tag(&self) -> &str {
        &self.root_tag
    }
}
