use crate::Span;

/// A lexical token with its exact source byte span.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Token {
    kind: TokenKind,
    span: Span,
}

impl Token {
    #[must_use]
    pub(crate) const fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }

    #[must_use]
    pub const fn kind(&self) -> &TokenKind {
        &self.kind
    }

    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
}

/// Token kinds recognized by the extractor lexer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TokenKind {
    Identifier(String),
    Number(String),
    String(String),
    InterpolatedString(Vec<InterpolatedTokenPart>),
    Dot,
    At,
    Bang,
    Equal,
    EqualEqual,
    BangEqual,
    AmpAmp,
    PipePipe,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    FatArrow,
    Plus,
    PlusEqual,
    Minus,
    MinusEqual,
    Star,
    Slash,
    Percent,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Comma,
    Colon,
    Eof,
}

/// A literal or embedded expression inside an interpolated string token.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InterpolatedTokenPart {
    Literal(String),
    Expression(Vec<Token>),
}
