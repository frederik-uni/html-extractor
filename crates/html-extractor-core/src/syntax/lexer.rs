use crate::{CompileError, Diagnostic, Span};

use super::{InterpolatedTokenPart, Token, TokenKind};

/// Converts extractor source into source-spanned tokens.
pub fn lex(source: &str) -> Result<Vec<Token>, CompileError> {
    Lexer::new(source).lex()
}

struct Lexer<'source> {
    source: &'source str,
    offset: usize,
    tokens: Vec<Token>,
}

impl<'source> Lexer<'source> {
    fn new(source: &'source str) -> Self {
        Self {
            source,
            offset: 0,
            tokens: Vec::new(),
        }
    }

    fn lex(mut self) -> Result<Vec<Token>, CompileError> {
        while let Some(character) = self.current() {
            if character.is_whitespace() {
                self.advance();
                continue;
            }

            let start = self.offset;
            match character {
                '/' if self.next_is('/') => self.skip_line_comment(),
                '/' if self.next_is('*') => self.skip_block_comment(start)?,
                '"' => self.lex_string(start)?,
                'f' if self.next_is('"') => self.lex_interpolated_string(start)?,
                character if character.is_ascii_digit() => self.lex_number(start),
                character if is_identifier_start(character) => self.lex_identifier(start),
                '.' => self.single(TokenKind::Dot, start),
                '@' => self.single(TokenKind::At, start),
                '+' => self.one_or_two(TokenKind::Plus, '=', TokenKind::PlusEqual, start),
                '-' => self.one_or_two(TokenKind::Minus, '=', TokenKind::MinusEqual, start),
                '*' => self.single(TokenKind::Star, start),
                '%' => self.single(TokenKind::Percent, start),
                '/' => self.single(TokenKind::Slash, start),
                '(' => self.single(TokenKind::LeftParen, start),
                ')' => self.single(TokenKind::RightParen, start),
                '{' => self.single(TokenKind::LeftBrace, start),
                '}' => self.single(TokenKind::RightBrace, start),
                '[' => self.single(TokenKind::LeftBracket, start),
                ']' => self.single(TokenKind::RightBracket, start),
                ',' => self.single(TokenKind::Comma, start),
                ':' => self.single(TokenKind::Colon, start),
                '!' => self.one_or_two(TokenKind::Bang, '=', TokenKind::BangEqual, start),
                '&' if self.next_is('&') => {
                    self.advance();
                    self.advance();
                    self.push(TokenKind::AmpAmp, start);
                }
                '|' if self.next_is('|') => {
                    self.advance();
                    self.advance();
                    self.push(TokenKind::PipePipe, start);
                }
                '<' => self.one_or_two(TokenKind::Less, '=', TokenKind::LessEqual, start),
                '>' => self.one_or_two(TokenKind::Greater, '=', TokenKind::GreaterEqual, start),
                '=' => {
                    self.advance();
                    let kind = match self.current() {
                        Some('=') => {
                            self.advance();
                            TokenKind::EqualEqual
                        }
                        Some('>') => {
                            self.advance();
                            TokenKind::FatArrow
                        }
                        _ => TokenKind::Equal,
                    };
                    self.push(kind, start);
                }
                unexpected => {
                    self.advance();
                    return Err(error(
                        format!("unexpected character `{unexpected}`"),
                        Span::new(start, self.offset),
                    ));
                }
            }
        }

        self.tokens.push(Token::new(
            TokenKind::Eof,
            Span::new(self.offset, self.offset),
        ));
        Ok(self.tokens)
    }

    fn current(&self) -> Option<char> {
        self.source.get(self.offset..)?.chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let character = self.current()?;
        self.offset += character.len_utf8();
        Some(character)
    }

    fn next_is(&self, expected: char) -> bool {
        let Some(current) = self.current() else {
            return false;
        };
        self.source[self.offset + current.len_utf8()..].starts_with(expected)
    }

    fn single(&mut self, kind: TokenKind, start: usize) {
        self.advance();
        self.push(kind, start);
    }

    fn one_or_two(&mut self, single: TokenKind, second: char, double: TokenKind, start: usize) {
        self.advance();
        let kind = if self.current() == Some(second) {
            self.advance();
            double
        } else {
            single
        };
        self.push(kind, start);
    }

    fn push(&mut self, kind: TokenKind, start: usize) {
        self.tokens
            .push(Token::new(kind, Span::new(start, self.offset)));
    }

    fn skip_line_comment(&mut self) {
        self.advance();
        self.advance();
        while !matches!(self.current(), None | Some('\n')) {
            self.advance();
        }
    }

    fn skip_block_comment(&mut self, start: usize) -> Result<(), CompileError> {
        self.advance();
        self.advance();
        while self.current().is_some() {
            if self.current() == Some('*') && self.next_is('/') {
                self.advance();
                self.advance();
                return Ok(());
            }
            self.advance();
        }
        Err(error(
            "unterminated block comment",
            Span::new(start, start + 2),
        ))
    }

    fn lex_string(&mut self, start: usize) -> Result<(), CompileError> {
        self.advance();
        let mut value = String::new();
        loop {
            match self.current() {
                None => {
                    return Err(error(
                        "unterminated string literal",
                        Span::new(start, start + 1),
                    ));
                }
                Some('"') => {
                    self.advance();
                    self.push(TokenKind::String(value), start);
                    return Ok(());
                }
                Some('\\') => {
                    let escape_start = self.offset;
                    self.advance();
                    let Some(escaped) = self.advance() else {
                        return Err(error(
                            "unterminated string literal",
                            Span::new(start, start + 1),
                        ));
                    };
                    let decoded = match escaped {
                        '"' => '"',
                        '\\' => '\\',
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        unsupported => {
                            return Err(error(
                                format!("unsupported string escape `\\{unsupported}`"),
                                Span::new(escape_start, self.offset),
                            ));
                        }
                    };
                    value.push(decoded);
                }
                Some(character) => {
                    self.advance();
                    value.push(character);
                }
            }
        }
    }

    fn lex_interpolated_string(&mut self, start: usize) -> Result<(), CompileError> {
        self.advance();
        self.advance();
        let mut parts = Vec::new();
        let mut literal = String::new();

        loop {
            match self.current() {
                None => {
                    return Err(error(
                        "unterminated interpolated string literal",
                        Span::new(start, start + 2),
                    ));
                }
                Some('"') => {
                    self.advance();
                    push_literal(&mut parts, &mut literal);
                    self.push(TokenKind::InterpolatedString(parts), start);
                    return Ok(());
                }
                Some('\\') => {
                    let escape_start = self.offset;
                    self.advance();
                    let Some(escaped) = self.advance() else {
                        return Err(error(
                            "unterminated interpolated string literal",
                            Span::new(start, start + 2),
                        ));
                    };
                    let decoded = match escaped {
                        '"' => '"',
                        '\\' => '\\',
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        unsupported => {
                            return Err(error(
                                format!("unsupported string escape `\\{unsupported}`"),
                                Span::new(escape_start, self.offset),
                            ));
                        }
                    };
                    literal.push(decoded);
                }
                Some('{') if self.next_is('{') => {
                    self.advance();
                    self.advance();
                    literal.push('{');
                }
                Some('}') if self.next_is('}') => {
                    self.advance();
                    self.advance();
                    literal.push('}');
                }
                Some('{') => {
                    push_literal(&mut parts, &mut literal);
                    let interpolation_start = self.offset;
                    self.advance();
                    let expression_start = self.offset;
                    let expression_end = self.scan_interpolation_end(interpolation_start)?;
                    if expression_start == expression_end {
                        return Err(error(
                            "empty interpolation expression",
                            Span::new(interpolation_start, self.offset),
                        ));
                    }
                    let tokens = shifted_tokens(
                        lex(&self.source[expression_start..expression_end])
                            .map_err(|error| shifted_error(error, expression_start))?,
                        expression_start,
                    );
                    parts.push(InterpolatedTokenPart::Expression(tokens));
                }
                Some('}') => {
                    let unmatched_start = self.offset;
                    self.advance();
                    return Err(error(
                        "unmatched `}` in interpolated string",
                        Span::new(unmatched_start, self.offset),
                    ));
                }
                Some(character) => {
                    self.advance();
                    literal.push(character);
                }
            }
        }
    }

    fn scan_interpolation_end(
        &mut self,
        interpolation_start: usize,
    ) -> Result<usize, CompileError> {
        let mut depth = 1_usize;
        while let Some(character) = self.current() {
            match character {
                '"' => self.skip_quoted_expression_string(interpolation_start)?,
                '/' if self.next_is('/') => self.skip_line_comment(),
                '/' if self.next_is('*') => self.skip_block_comment(self.offset)?,
                '{' => {
                    depth += 1;
                    self.advance();
                }
                '}' => {
                    depth -= 1;
                    let end = self.offset;
                    self.advance();
                    if depth == 0 {
                        return Ok(end);
                    }
                }
                _ => {
                    self.advance();
                }
            }
        }
        Err(error(
            "unterminated interpolation expression",
            Span::new(interpolation_start, interpolation_start + 1),
        ))
    }

    fn skip_quoted_expression_string(
        &mut self,
        interpolation_start: usize,
    ) -> Result<(), CompileError> {
        self.advance();
        loop {
            match self.current() {
                None => {
                    return Err(error(
                        "unterminated interpolation expression",
                        Span::new(interpolation_start, interpolation_start + 1),
                    ));
                }
                Some('"') => {
                    self.advance();
                    return Ok(());
                }
                Some('\\') => {
                    self.advance();
                    if self.advance().is_none() {
                        return Err(error(
                            "unterminated interpolation expression",
                            Span::new(interpolation_start, interpolation_start + 1),
                        ));
                    }
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn lex_number(&mut self, start: usize) {
        while self
            .current()
            .is_some_and(|character| character.is_ascii_digit())
        {
            self.advance();
        }
        if self.current() == Some('.')
            && self
                .source
                .get(self.offset + 1..)
                .and_then(|rest| rest.chars().next())
                .is_some_and(|character| character.is_ascii_digit())
        {
            self.advance();
            while self
                .current()
                .is_some_and(|character| character.is_ascii_digit())
            {
                self.advance();
            }
        }
        self.push(
            TokenKind::Number(self.source[start..self.offset].to_owned()),
            start,
        );
    }

    fn lex_identifier(&mut self, start: usize) {
        self.advance();
        while self.current().is_some_and(is_identifier_continue) {
            self.advance();
        }
        self.push(
            TokenKind::Identifier(self.source[start..self.offset].to_owned()),
            start,
        );
    }
}

fn push_literal(parts: &mut Vec<InterpolatedTokenPart>, literal: &mut String) {
    if !literal.is_empty() {
        parts.push(InterpolatedTokenPart::Literal(std::mem::take(literal)));
    }
}

fn shifted_tokens(tokens: Vec<Token>, offset: usize) -> Vec<Token> {
    tokens
        .into_iter()
        .map(|token| {
            let span = token.span();
            Token::new(
                shifted_kind(token.kind(), offset),
                Span::new(span.start() + offset, span.end() + offset),
            )
        })
        .collect()
}

fn shifted_kind(kind: &TokenKind, offset: usize) -> TokenKind {
    match kind {
        TokenKind::InterpolatedString(parts) => TokenKind::InterpolatedString(
            parts
                .iter()
                .map(|part| match part {
                    InterpolatedTokenPart::Literal(value) => {
                        InterpolatedTokenPart::Literal(value.clone())
                    }
                    InterpolatedTokenPart::Expression(tokens) => {
                        InterpolatedTokenPart::Expression(shifted_tokens(tokens.clone(), offset))
                    }
                })
                .collect(),
        ),
        other => other.clone(),
    }
}

fn shifted_error(error: CompileError, offset: usize) -> CompileError {
    CompileError::from_diagnostics(
        error
            .diagnostics()
            .iter()
            .map(|diagnostic| {
                let span = diagnostic.primary_span();
                Diagnostic::new(
                    diagnostic.message(),
                    Span::new(span.start() + offset, span.end() + offset),
                )
            })
            .collect(),
    )
}

fn is_identifier_start(character: char) -> bool {
    character == '_' || character.is_alphabetic()
}

fn is_identifier_continue(character: char) -> bool {
    character == '_' || character.is_alphanumeric()
}

fn error(message: impl Into<String>, span: Span) -> CompileError {
    CompileError::new(Diagnostic::new(message, span))
}
