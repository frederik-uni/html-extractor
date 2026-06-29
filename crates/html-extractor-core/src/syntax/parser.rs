use std::mem::discriminant;

use crate::{CompileError, Diagnostic, Span};

use super::{
    AssignmentOperator, BinaryOperator, Expression, ExpressionKind, FunctionDefinition,
    InterpolatedStringPart, InterpolatedTokenPart, ObjectField, Program, Statement, StatementKind,
    SwitchArm, SwitchPattern, TestBlock, TestInput, Token, TokenKind, UnaryOperator, Visibility,
    lex,
};

pub fn parse(source: &str) -> Result<Program, CompileError> {
    Parser::new(lex(source)?).parse_program()
}

struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    fn parse_program(mut self) -> Result<Program, CompileError> {
        let mut functions = Vec::new();
        let mut tests = Vec::new();
        let mut statements = Vec::new();
        while !self.at(&TokenKind::Eof) {
            if self.identifier_at(0) == Some("fun") {
                functions.push(self.function_definition()?);
            } else if self.identifier_at(0) == Some("test")
                && self.kind_at(1).is_some_and(|kind| {
                    same(kind, &TokenKind::LeftParen) || same(kind, &TokenKind::LeftBrace)
                })
            {
                tests.push(self.test_block()?);
            } else {
                statements.push(self.statement()?);
            }
        }
        let end = self.peek().span().end();
        Ok(Program::new(
            functions,
            tests,
            statements,
            Span::new(0, end),
        ))
    }

    fn statement(&mut self) -> Result<Statement, CompileError> {
        if self.identifier_at(0) == Some("fun") {
            return Err(self.error_at(
                "function declarations are only allowed at top level",
                self.peek().span(),
            ));
        }
        if self.consume_identifier("return") {
            let start = self.previous().span().start();
            let value = self.expression()?;
            let span = Span::new(start, value.span().end());
            return Ok(Statement::new(StatementKind::Return(value), span));
        }
        if self.consume_identifier("while") {
            let start = self.previous().span().start();
            let condition = self.expression()?;
            let body = self.block()?;
            let end = self.previous().span().end();
            return Ok(Statement::new(
                StatementKind::While { condition, body },
                Span::new(start, end),
            ));
        }
        if self.consume_identifier("loop") {
            let start = self.previous().span().start();
            let body = self.block()?;
            let end = self.previous().span().end();
            return Ok(Statement::new(
                StatementKind::Loop { body },
                Span::new(start, end),
            ));
        }
        if self.consume_identifier("break") {
            let span = self.previous().span();
            return Ok(Statement::new(StatementKind::Break, span));
        }
        if self.identifier_at(0).is_some()
            && self
                .kind_at(1)
                .is_some_and(|kind| assignment_operator(kind).is_some())
        {
            return self.assignment();
        }
        if self.is_computed_assignment() {
            return self.computed_assignment();
        }

        let expression = self.expression()?;
        let span = expression.span();
        Ok(Statement::new(StatementKind::Expression(expression), span))
    }

    fn test_block(&mut self) -> Result<TestBlock, CompileError> {
        let start = self
            .expect_identifier("test", "expected `test` before test block")?
            .start();
        let mut inputs = Vec::new();
        let runs_script = if self.consume(&TokenKind::LeftParen) {
            if !self.at(&TokenKind::RightParen) {
                loop {
                    let (name, name_span) = self.take_identifier("expected test input name")?;
                    self.expect(&TokenKind::Equal, "expected `=` after test input name")?;
                    let value = self.expression()?;
                    let visibility = visibility_for(&name);
                    let span = Span::new(name_span.start(), value.span().end());
                    inputs.push(TestInput::new(name, visibility, value, span));
                    if !self.consume(&TokenKind::Comma) {
                        break;
                    }
                    if self.at(&TokenKind::RightParen) {
                        break;
                    }
                }
            }
            self.expect(&TokenKind::RightParen, "expected `)` after test inputs")?;
            true
        } else {
            false
        };
        let body = self.block()?;
        let end = self.previous().span().end();
        Ok(TestBlock::new(
            inputs,
            body,
            runs_script,
            Span::new(start, end),
        ))
    }

    fn function_definition(&mut self) -> Result<FunctionDefinition, CompileError> {
        let start = self
            .expect_identifier("fun", "expected `fun` before function declaration")?
            .start();
        let (name, _) = self.take_identifier("expected function name after `fun`")?;
        self.expect(&TokenKind::LeftParen, "expected `(` after function name")?;
        let mut parameters = Vec::new();
        if !self.at(&TokenKind::RightParen) {
            loop {
                parameters.push(self.take_identifier("expected parameter name")?.0);
                if !self.consume(&TokenKind::Comma) {
                    break;
                }
                if self.at(&TokenKind::RightParen) {
                    break;
                }
            }
        }
        self.expect(
            &TokenKind::RightParen,
            "expected `)` after function parameters",
        )?;
        let body = self.block()?;
        let end = self.previous().span().end();
        Ok(FunctionDefinition::new(
            name,
            parameters,
            body,
            Span::new(start, end),
        ))
    }

    fn is_computed_assignment(&self) -> bool {
        if !self.at(&TokenKind::LeftBracket) {
            return false;
        }
        let mut depth = 0usize;
        for offset in 0..self.tokens.len().saturating_sub(self.current) {
            match self.kind_at(offset) {
                Some(TokenKind::LeftBracket) => depth += 1,
                Some(TokenKind::RightBracket) => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return self
                            .kind_at(offset + 1)
                            .is_some_and(|kind| same(kind, &TokenKind::Equal));
                    }
                }
                Some(TokenKind::Eof) | None => break,
                _ => {}
            }
        }
        false
    }

    fn computed_assignment(&mut self) -> Result<Statement, CompileError> {
        let start = self
            .expect(
                &TokenKind::LeftBracket,
                "expected `[` before computed assignment",
            )?
            .span()
            .start();
        let name = self.expression()?;
        self.expect(
            &TokenKind::RightBracket,
            "expected `]` after computed assignment name",
        )?;
        self.expect(
            &TokenKind::Equal,
            "expected `=` after computed assignment name",
        )?;
        let value = self.expression()?;
        let span = Span::new(start, value.span().end());
        Ok(Statement::new(
            StatementKind::ComputedAssignment { name, value },
            span,
        ))
    }

    fn assignment(&mut self) -> Result<Statement, CompileError> {
        let (name, name_span) = self.take_identifier("expected assignment name")?;
        let start = name_span.start();
        let visibility = visibility_for(&name);
        let operator = assignment_operator(self.peek().kind())
            .ok_or_else(|| self.error_at("expected assignment operator", self.peek().span()))?;
        self.advance();
        let value = self.expression()?;
        let span = Span::new(start, value.span().end());
        Ok(Statement::new(
            StatementKind::Assignment {
                name,
                operator,
                visibility,
                value,
            },
            span,
        ))
    }

    fn expression(&mut self) -> Result<Expression, CompileError> {
        if self.identifier_at(0) == Some("if") {
            let start = self.advance().span().start();
            return self.block_if_expression(start);
        }
        if let Some(parameter) = self.identifier_at(0).map(str::to_owned)
            && self
                .kind_at(1)
                .is_some_and(|kind| same(kind, &TokenKind::FatArrow))
        {
            let start = self.advance().span().start();
            self.advance();
            let body = self.expression()?;
            let span = Span::new(start, body.span().end());
            return Ok(Expression::new(
                ExpressionKind::Lambda {
                    parameters: vec![parameter],
                    body: Box::new(body),
                },
                span,
            ));
        }
        if self.is_parenthesized_lambda() {
            let start = self.advance().span().start();
            let mut parameters = Vec::new();
            loop {
                parameters.push(self.take_identifier("expected lambda parameter")?.0);
                if !self.consume(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(
                &TokenKind::RightParen,
                "expected `)` after lambda parameters",
            )?;
            self.expect(
                &TokenKind::FatArrow,
                "expected `=>` after lambda parameters",
            )?;
            let body = self.expression()?;
            let span = Span::new(start, body.span().end());
            return Ok(Expression::new(
                ExpressionKind::Lambda {
                    parameters,
                    body: Box::new(body),
                },
                span,
            ));
        }
        self.conditional()
    }

    fn conditional(&mut self) -> Result<Expression, CompileError> {
        let then_branch = self.binary(0)?;
        if self.identifier_at(0) == Some("if") && self.next_if_starts_block() {
            return Ok(then_branch);
        }
        if !self.consume_identifier("if") {
            return Ok(then_branch);
        }

        let condition = self.binary(0)?;
        self.expect_identifier("else", "expected `else` in conditional expression")?;
        let else_branch = self.expression()?;
        let span = Span::new(then_branch.span().start(), else_branch.span().end());
        Ok(Expression::new(
            ExpressionKind::IfElse {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
            },
            span,
        ))
    }

    fn next_if_starts_block(&self) -> bool {
        let mut parentheses = 0usize;
        let mut brackets = 0usize;
        let mut offset = 1usize;
        loop {
            match self.kind_at(offset) {
                Some(TokenKind::LeftParen) => parentheses += 1,
                Some(TokenKind::RightParen) => parentheses = parentheses.saturating_sub(1),
                Some(TokenKind::LeftBracket) => brackets += 1,
                Some(TokenKind::RightBracket) => brackets = brackets.saturating_sub(1),
                Some(TokenKind::LeftBrace) if parentheses == 0 && brackets == 0 => return true,
                Some(TokenKind::Identifier(name))
                    if name == "else" && parentheses == 0 && brackets == 0 =>
                {
                    return false;
                }
                Some(TokenKind::Eof) | None => return false,
                _ => {}
            }
            offset += 1;
        }
    }

    fn is_parenthesized_lambda(&self) -> bool {
        if !self.at(&TokenKind::LeftParen) || self.identifier_at(1).is_none() {
            return false;
        }
        let mut offset = 2;
        while self
            .kind_at(offset)
            .is_some_and(|kind| same(kind, &TokenKind::Comma))
        {
            if self.identifier_at(offset + 1).is_none() {
                return false;
            }
            offset += 2;
        }
        self.kind_at(offset)
            .is_some_and(|kind| same(kind, &TokenKind::RightParen))
            && self
                .kind_at(offset + 1)
                .is_some_and(|kind| same(kind, &TokenKind::FatArrow))
    }

    fn binary(&mut self, minimum_precedence: u8) -> Result<Expression, CompileError> {
        let mut left = self.unary()?;
        while let Some((operator, precedence)) = self.binary_operator() {
            if precedence < minimum_precedence {
                break;
            }
            self.advance();
            let right = self.binary(precedence + 1)?;
            let span = Span::new(left.span().start(), right.span().end());
            left = Expression::new(
                ExpressionKind::Binary {
                    left: Box::new(left),
                    operator,
                    right: Box::new(right),
                },
                span,
            );
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expression, CompileError> {
        let operator = if self.at(&TokenKind::Minus) {
            Some(UnaryOperator::Negate)
        } else if self.at(&TokenKind::Bang) {
            Some(UnaryOperator::Not)
        } else {
            None
        };
        if let Some(operator) = operator {
            let start = self.advance().span().start();
            let operand = self.unary()?;
            let span = Span::new(start, operand.span().end());
            return Ok(Expression::new(
                ExpressionKind::Unary {
                    operator,
                    operand: Box::new(operand),
                },
                span,
            ));
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Result<Expression, CompileError> {
        let mut expression = self.primary()?;
        loop {
            if self.at(&TokenKind::Dot) {
                self.advance();
                match self.peek().kind() {
                    TokenKind::Identifier(_) => {
                        let (name, name_span) = self
                            .take_identifier("expected property name or method name after `.`")?;
                        if self.at(&TokenKind::LeftBrace) {
                            let body = self.block()?;
                            let end = self.previous().span().end();
                            let start = expression.span().start();
                            expression = Expression::new(
                                ExpressionKind::FunctionalBlock {
                                    receiver: Box::new(expression),
                                    method: name,
                                    body,
                                },
                                Span::new(start, end),
                            );
                        } else if self.at(&TokenKind::LeftParen) || self.at(&TokenKind::At) {
                            let multiple = self.consume(&TokenKind::At);
                            let arguments = self.arguments()?;
                            let end = self.previous().span().end();
                            let start = expression.span().start();
                            expression = Expression::new(
                                ExpressionKind::MethodCall {
                                    receiver: Box::new(expression),
                                    name,
                                    multiple,
                                    arguments,
                                },
                                Span::new(start, end),
                            );
                        } else {
                            let start = expression.span().start();
                            let end = name_span.end();
                            let index = Expression::new(ExpressionKind::String(name), name_span);
                            expression = Expression::new(
                                ExpressionKind::Subscript {
                                    receiver: Box::new(expression),
                                    index: Box::new(index),
                                },
                                Span::new(start, end),
                            );
                        }
                    }
                    TokenKind::Number(value) if value.bytes().all(|byte| byte.is_ascii_digit()) => {
                        let token = self.advance().clone();
                        let span = token.span();
                        let TokenKind::Number(value) = token.kind() else {
                            unreachable!();
                        };
                        let start = expression.span().start();
                        let index = Expression::new(ExpressionKind::Number(value.clone()), span);
                        expression = Expression::new(
                            ExpressionKind::Subscript {
                                receiver: Box::new(expression),
                                index: Box::new(index),
                            },
                            Span::new(start, span.end()),
                        );
                    }
                    TokenKind::Number(_) => {
                        return Err(self.error_at(
                            "expected non-negative integer index after `.`",
                            self.peek().span(),
                        ));
                    }
                    _ => {
                        return Err(self.error_at(
                            "expected property name or non-negative integer after `.`",
                            self.peek().span(),
                        ));
                    }
                }
                continue;
            }
            if self.at(&TokenKind::Bang) {
                let start = expression.span().start();
                let end = self.advance().span().end();
                expression = Expression::new(
                    ExpressionKind::Required(Box::new(expression)),
                    Span::new(start, end),
                );
                continue;
            }
            if self.at(&TokenKind::LeftBracket) {
                if self.is_computed_assignment() {
                    break;
                }
                self.advance();
                let index = self.expression()?;
                self.expect(
                    &TokenKind::RightBracket,
                    "expected `]` after subscript index",
                )?;
                let end = self.previous().span().end();
                let start = expression.span().start();
                expression = Expression::new(
                    ExpressionKind::Subscript {
                        receiver: Box::new(expression),
                        index: Box::new(index),
                    },
                    Span::new(start, end),
                );
                continue;
            }
            break;
        }
        Ok(expression)
    }

    fn primary(&mut self) -> Result<Expression, CompileError> {
        let token = self.advance().clone();
        let span = token.span();
        match token.kind() {
            TokenKind::String(value) => {
                Ok(Expression::new(ExpressionKind::String(value.clone()), span))
            }
            TokenKind::InterpolatedString(parts) => {
                let parts = parts
                    .iter()
                    .map(|part| match part {
                        InterpolatedTokenPart::Literal(value) => {
                            Ok(InterpolatedStringPart::Literal(value.clone()))
                        }
                        InterpolatedTokenPart::Expression(tokens) => {
                            let mut parser = Parser::new(tokens.clone());
                            let expression = parser.expression()?;
                            if !parser.at(&TokenKind::Eof) {
                                return Err(parser.error_at(
                                    "expected end of interpolation expression",
                                    parser.peek().span(),
                                ));
                            }
                            Ok(InterpolatedStringPart::Expression(expression))
                        }
                    })
                    .collect::<Result<Vec<_>, CompileError>>()?;
                Ok(Expression::new(
                    ExpressionKind::InterpolatedString(parts),
                    span,
                ))
            }
            TokenKind::Number(value) => {
                Ok(Expression::new(ExpressionKind::Number(value.clone()), span))
            }
            TokenKind::Identifier(name) if name == "null" => {
                Ok(Expression::new(ExpressionKind::Null, span))
            }
            TokenKind::Identifier(name) if name == "true" || name == "false" => Ok(
                Expression::new(ExpressionKind::Boolean(name == "true"), span),
            ),
            TokenKind::Identifier(name) if name == "switch" => self.switch_expression(span.start()),
            TokenKind::Identifier(name) => {
                if self.at(&TokenKind::LeftParen) || self.at(&TokenKind::At) {
                    let multiple = self.consume(&TokenKind::At);
                    let arguments = self.arguments()?;
                    let end = self.previous().span().end();
                    let name = if multiple {
                        format!("{name}@")
                    } else {
                        name.clone()
                    };
                    Ok(Expression::new(
                        ExpressionKind::FunctionCall { name, arguments },
                        Span::new(span.start(), end),
                    ))
                } else {
                    Ok(Expression::new(
                        ExpressionKind::Variable(name.clone()),
                        span,
                    ))
                }
            }
            TokenKind::LeftParen => {
                let expression = self.expression()?;
                self.expect(&TokenKind::RightParen, "expected `)` after expression")?;
                Ok(expression)
            }
            TokenKind::LeftBracket => self.array(span.start()),
            TokenKind::LeftBrace => self.object(span.start()),
            _ => Err(self.error_at("expected expression", span)),
        }
    }

    fn block_if_expression(&mut self, start: usize) -> Result<Expression, CompileError> {
        let condition = self.expression()?;
        let then_branch = self.block()?;
        let else_branch = if self.consume_identifier("else") {
            Some(self.block()?)
        } else {
            None
        };
        let end = self.previous().span().end();
        Ok(Expression::new(
            ExpressionKind::BlockIf {
                condition: Box::new(condition),
                then_branch,
                else_branch,
            },
            Span::new(start, end),
        ))
    }

    fn switch_expression(&mut self, start: usize) -> Result<Expression, CompileError> {
        self.expect(&TokenKind::LeftParen, "expected `(` after `switch`")?;
        let target = self.expression()?;
        self.expect(&TokenKind::RightParen, "expected `)` after switch target")?;
        self.expect(&TokenKind::LeftBrace, "expected `{` before switch arms")?;

        let mut arms = Vec::new();
        while !self.at(&TokenKind::RightBrace) && !self.at(&TokenKind::Eof) {
            arms.push(self.switch_arm()?);
            if !self.consume(&TokenKind::Comma) {
                break;
            }
            if self.at(&TokenKind::RightBrace) {
                break;
            }
        }

        let end = self
            .expect(&TokenKind::RightBrace, "expected `}` after switch arms")?
            .span()
            .end();
        if let Some(non_final) = arms.iter().take(arms.len().saturating_sub(1)).find(|arm| {
            matches!(
                arm.pattern(),
                SwitchPattern::Wildcard | SwitchPattern::Binding(_)
            )
        }) {
            return Err(self.error_at(
                "only the final switch arm can be `_ => ...` or `name => ...`",
                non_final.span(),
            ));
        }
        if !arms.last().is_some_and(|arm| {
            matches!(
                arm.pattern(),
                SwitchPattern::Wildcard | SwitchPattern::Binding(_)
            )
        }) {
            return Err(self.error_at(
                "final switch arm must be `_ => ...` or `name => ...`",
                Span::new(start, end),
            ));
        }

        Ok(Expression::new(
            ExpressionKind::Switch {
                target: Box::new(target),
                arms,
            },
            Span::new(start, end),
        ))
    }

    fn switch_arm(&mut self) -> Result<SwitchArm, CompileError> {
        let start = self.peek().span().start();
        let pattern = if self.identifier_at(0).is_some()
            && self
                .kind_at(1)
                .is_some_and(|kind| same(kind, &TokenKind::FatArrow))
        {
            let (name, _) = self.take_identifier("expected switch pattern")?;
            if name == "_" {
                SwitchPattern::Wildcard
            } else {
                SwitchPattern::Binding(name)
            }
        } else {
            SwitchPattern::Value(self.expression()?)
        };
        self.expect(&TokenKind::FatArrow, "expected `=>` after switch pattern")?;
        let value = self.expression()?;
        let span = Span::new(start, value.span().end());
        Ok(SwitchArm::new(pattern, value, span))
    }

    fn array(&mut self, start: usize) -> Result<Expression, CompileError> {
        let mut values = Vec::new();
        if !self.at(&TokenKind::RightBracket) {
            loop {
                values.push(self.expression()?);
                if !self.consume(&TokenKind::Comma) {
                    break;
                }
                if self.at(&TokenKind::RightBracket) {
                    break;
                }
            }
        }
        let end = self
            .expect(&TokenKind::RightBracket, "expected `]` after array")?
            .span()
            .end();
        Ok(Expression::new(
            ExpressionKind::Array(values),
            Span::new(start, end),
        ))
    }

    fn object(&mut self, start: usize) -> Result<Expression, CompileError> {
        let mut fields = Vec::new();
        if !self.at(&TokenKind::RightBrace) {
            loop {
                let (name, name_span) = self.take_identifier("expected object field name")?;
                self.expect(&TokenKind::Colon, "expected `:` after object field name")?;
                let value = self.expression()?;
                let span = Span::new(name_span.start(), value.span().end());
                fields.push(ObjectField::new(name, value, span));
                if !self.consume(&TokenKind::Comma) {
                    break;
                }
                if self.at(&TokenKind::RightBrace) {
                    break;
                }
            }
        }
        let end = self
            .expect(&TokenKind::RightBrace, "expected `}` after object")?
            .span()
            .end();
        Ok(Expression::new(
            ExpressionKind::Object(fields),
            Span::new(start, end),
        ))
    }

    fn arguments(&mut self) -> Result<Vec<Expression>, CompileError> {
        self.expect(&TokenKind::LeftParen, "expected `(` before arguments")?;
        let mut arguments = Vec::new();
        if !self.at(&TokenKind::RightParen) {
            loop {
                arguments.push(self.expression()?);
                if !self.consume(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(&TokenKind::RightParen, "expected `)` after arguments")?;
        Ok(arguments)
    }

    fn block(&mut self) -> Result<Vec<Statement>, CompileError> {
        self.expect(&TokenKind::LeftBrace, "expected `{` before block")?;
        let mut statements = Vec::new();
        while !self.at(&TokenKind::RightBrace) && !self.at(&TokenKind::Eof) {
            statements.push(self.statement()?);
        }
        self.expect(&TokenKind::RightBrace, "expected `}` after block")?;
        Ok(statements)
    }

    fn binary_operator(&self) -> Option<(BinaryOperator, u8)> {
        Some(match self.peek().kind() {
            TokenKind::PipePipe => (BinaryOperator::Or, 1),
            TokenKind::AmpAmp => (BinaryOperator::And, 2),
            TokenKind::EqualEqual => (BinaryOperator::Equal, 3),
            TokenKind::BangEqual => (BinaryOperator::NotEqual, 3),
            TokenKind::Less => (BinaryOperator::Less, 4),
            TokenKind::LessEqual => (BinaryOperator::LessEqual, 4),
            TokenKind::Greater => (BinaryOperator::Greater, 4),
            TokenKind::GreaterEqual => (BinaryOperator::GreaterEqual, 4),
            TokenKind::Plus => (BinaryOperator::Add, 5),
            TokenKind::Minus => (BinaryOperator::Subtract, 5),
            TokenKind::Star => (BinaryOperator::Multiply, 6),
            TokenKind::Slash => (BinaryOperator::Divide, 6),
            TokenKind::Percent => (BinaryOperator::Remainder, 6),
            _ => return None,
        })
    }

    fn take_identifier(&mut self, message: &str) -> Result<(String, Span), CompileError> {
        let token = self.advance().clone();
        match token.kind() {
            TokenKind::Identifier(name) => Ok((name.clone(), token.span())),
            _ => Err(self.error_at(message, token.span())),
        }
    }

    fn consume_identifier(&mut self, expected: &str) -> bool {
        if self.identifier_at(0) == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect_identifier(&mut self, expected: &str, message: &str) -> Result<Span, CompileError> {
        if !self.consume_identifier(expected) {
            return Err(self.error_at(message, self.peek().span()));
        }
        Ok(self.previous().span())
    }

    fn expect(&mut self, kind: &TokenKind, message: &str) -> Result<&Token, CompileError> {
        if !self.at(kind) {
            return Err(self.error_at(message, self.peek().span()));
        }
        Ok(self.advance())
    }

    fn consume(&mut self, kind: &TokenKind) -> bool {
        if self.at(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn at(&self, kind: &TokenKind) -> bool {
        same(self.peek().kind(), kind)
    }

    fn identifier_at(&self, offset: usize) -> Option<&str> {
        match self.tokens.get(self.current + offset)?.kind() {
            TokenKind::Identifier(name) => Some(name),
            _ => None,
        }
    }

    fn kind_at(&self, offset: usize) -> Option<&TokenKind> {
        self.tokens.get(self.current + offset).map(Token::kind)
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn advance(&mut self) -> &Token {
        let index = self.current;
        if !self.at(&TokenKind::Eof) {
            self.current += 1;
        }
        &self.tokens[index]
    }

    fn error_at(&self, message: impl Into<String>, span: Span) -> CompileError {
        CompileError::new(Diagnostic::new(message, span))
    }
}

fn same(left: &TokenKind, right: &TokenKind) -> bool {
    discriminant(left) == discriminant(right)
}

fn assignment_operator(kind: &TokenKind) -> Option<AssignmentOperator> {
    match kind {
        TokenKind::Equal => Some(AssignmentOperator::Assign),
        TokenKind::PlusEqual => Some(AssignmentOperator::Add),
        TokenKind::MinusEqual => Some(AssignmentOperator::Subtract),
        _ => None,
    }
}

fn visibility_for(name: &str) -> Visibility {
    if name.starts_with('_') {
        Visibility::Private
    } else {
        Visibility::Public
    }
}
