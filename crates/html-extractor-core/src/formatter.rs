use crate::{
    AssignmentOperator, BinaryOperator, CompileError, Expression, ExpressionKind,
    FunctionDefinition, InterpolatedStringPart, ObjectField, ProgramView, Statement, StatementKind,
    SwitchArm, SwitchPattern, TestBlock, UnaryOperator, Visibility, compile,
};

const INDENT: &str = "    ";
const PREC_LOWEST: u8 = 0;
const PREC_UNARY: u8 = 7;
const PREC_POSTFIX: u8 = 8;

pub fn format(source: &str) -> Result<String, CompileError> {
    let program = compile(source)?;
    let mut formatter = Formatter::default();
    formatter.program(&program);
    Ok(formatter.finish())
}

#[derive(Default)]
struct Formatter {
    output: String,
    indent: usize,
}

impl Formatter {
    fn finish(mut self) -> String {
        if !self.output.ends_with('\n') {
            self.output.push('\n');
        }
        self.output
    }

    fn program(&mut self, program: &impl ProgramView) {
        let mut wrote_any = false;
        for function in program.functions() {
            if wrote_any {
                self.blank_line();
            }
            self.function(function);
            wrote_any = true;
        }
        if !program.statements().is_empty() && wrote_any {
            self.blank_line();
        }
        for statement in program.statements() {
            self.statement(statement);
            wrote_any = true;
        }
        if !program.statements().is_empty() && !program.tests().is_empty() {
            self.blank_line();
        }
        for (index, test) in program.tests().iter().enumerate() {
            if index > 0 || (program.statements().is_empty() && wrote_any) {
                self.blank_line();
            }
            self.test(test);
            wrote_any = true;
        }
    }

    fn function(&mut self, function: &FunctionDefinition) {
        self.write_indent();
        self.write("fun ");
        self.write(function.name());
        self.write("(");
        self.write_joined(function.parameters(), ", ", |formatter, parameter| {
            formatter.write(parameter);
        });
        self.write(") ");
        self.block(function.body());
    }

    fn test(&mut self, test: &TestBlock) {
        self.write_indent();
        self.write("test");
        if test.runs_script() {
            self.write("(");
            self.write_joined(test.inputs(), ", ", |formatter, input| {
                formatter.write(input.name());
                formatter.write(" = ");
                formatter.expression(input.value());
            });
            self.write(")");
        }
        self.write(" ");
        self.block(test.body());
    }

    fn statement(&mut self, statement: &Statement) {
        self.write_indent();
        match statement.kind() {
            StatementKind::Assignment {
                name,
                operator,
                visibility,
                value,
            } => {
                if *visibility == Visibility::Private && !name.starts_with('_') {
                    self.write("_");
                }
                self.write(name);
                self.write(match operator {
                    AssignmentOperator::Assign => " = ",
                    AssignmentOperator::Add => " += ",
                    AssignmentOperator::Subtract => " -= ",
                });
                self.expression(value);
                self.newline();
            }
            StatementKind::ComputedAssignment { name, value } => {
                self.write("[");
                self.expression(name);
                self.write("] = ");
                self.expression(value);
                self.newline();
            }
            StatementKind::Return(value) => {
                self.write("return ");
                self.expression(value);
                self.newline();
            }
            StatementKind::While { condition, body } => {
                self.write("while ");
                self.expression(condition);
                self.write(" ");
                self.block(body);
                self.newline();
            }
            StatementKind::Loop { body } => {
                self.write("loop ");
                self.block(body);
                self.newline();
            }
            StatementKind::Break => {
                self.write("break");
                self.newline();
            }
            StatementKind::Expression(value) => {
                self.expression(value);
                self.newline();
            }
        }
    }

    fn expression(&mut self, expression: &Expression) {
        self.expression_with_min(expression, PREC_LOWEST);
    }

    fn expression_with_min(&mut self, expression: &Expression, minimum_precedence: u8) {
        let needs_parentheses = expression_precedence(expression) < minimum_precedence;
        if needs_parentheses {
            self.write("(");
        }
        self.expression_unwrapped(expression);
        if needs_parentheses {
            self.write(")");
        }
    }

    fn expression_unwrapped(&mut self, expression: &Expression) {
        if postfix_chain_len(expression) >= 4 {
            self.postfix_chain(expression);
            return;
        }
        match expression.kind() {
            ExpressionKind::Null => self.write("null"),
            ExpressionKind::Boolean(value) => self.write(if *value { "true" } else { "false" }),
            ExpressionKind::Number(value) => self.write(value),
            ExpressionKind::String(value) => self.string(value),
            ExpressionKind::InterpolatedString(parts) => self.interpolated_string(parts),
            ExpressionKind::Array(values) => {
                self.write("[");
                self.write_joined(values, ", ", |formatter, value| formatter.expression(value));
                self.write("]");
            }
            ExpressionKind::Object(fields) => self.object(fields),
            ExpressionKind::Variable(name) | ExpressionKind::Identifier(name) => self.write(name),
            ExpressionKind::FunctionCall { name, arguments } => {
                self.write(name.trim_end_matches('@'));
                if name.ends_with('@') {
                    self.write("@");
                }
                self.arguments(arguments);
            }
            ExpressionKind::QualifiedFunctionCall {
                module,
                name,
                arguments,
            } => {
                self.write(module);
                self.write(".");
                self.write(name);
                self.arguments(arguments);
            }
            ExpressionKind::CoreCall { name, arguments } => {
                self.write(name);
                self.arguments(arguments);
            }
            ExpressionKind::MethodCall {
                receiver,
                name,
                multiple,
                arguments,
            } => {
                self.expression_with_min(receiver, PREC_POSTFIX);
                self.write(".");
                self.write(name);
                if *multiple {
                    self.write("@");
                }
                self.arguments(arguments);
            }
            ExpressionKind::Lambda { parameters, body } => {
                if parameters.as_slice() == ["it"] {
                    self.expression(body);
                } else {
                    self.write_joined(parameters, ", ", |formatter, parameter| {
                        formatter.write(parameter);
                    });
                    self.write(" => ");
                    self.expression(body);
                }
            }
            ExpressionKind::FunctionalBlock {
                receiver,
                method,
                body,
            } => {
                self.expression_with_min(receiver, PREC_POSTFIX);
                self.write(".");
                self.write(method);
                self.write(" ");
                self.block(body);
            }
            ExpressionKind::Required(value) => {
                self.expression_with_min(value, PREC_POSTFIX);
                self.write("!");
            }
            ExpressionKind::Unary { operator, operand } => {
                self.write(match operator {
                    UnaryOperator::Negate => "-",
                    UnaryOperator::Not => "!",
                });
                self.expression_with_min(operand, PREC_UNARY);
            }
            ExpressionKind::Binary {
                left,
                operator,
                right,
            } => {
                let precedence = binary_precedence(*operator);
                self.binary_child(left, precedence, false);
                self.write(" ");
                self.write(binary_operator(*operator));
                self.write(" ");
                self.binary_child(right, precedence, true);
            }
            ExpressionKind::IfElse {
                condition,
                then_branch,
                else_branch,
            } => {
                self.write("if ");
                self.expression(condition);
                self.write(" then ");
                self.expression(then_branch);
                self.write(" else ");
                self.expression(else_branch);
            }
            ExpressionKind::BlockIf {
                condition,
                then_branch,
                else_branch,
            } => {
                self.write("if ");
                self.expression(condition);
                self.write(" ");
                self.block(then_branch);
                if let Some(else_branch) = else_branch {
                    self.write(" else ");
                    self.block(else_branch);
                }
            }
            ExpressionKind::Switch { target, arms } => {
                self.write("switch(");
                self.expression(target);
                self.write(") {");
                if arms.is_empty() {
                    self.write("}");
                    return;
                }
                self.newline();
                self.indent += 1;
                for (index, arm) in arms.iter().enumerate() {
                    self.switch_arm(arm, index + 1 < arms.len());
                }
                self.indent -= 1;
                self.write_indent();
                self.write("}");
            }
            ExpressionKind::Subscript { receiver, index } => {
                self.expression_with_min(receiver, PREC_POSTFIX);
                self.subscript(index);
            }
        }
    }

    fn postfix_chain(&mut self, expression: &Expression) {
        let mut segments = Vec::new();
        let base = postfix_chain_base(expression, &mut segments);
        self.expression_with_min(base, PREC_POSTFIX);
        self.indent += 1;
        for segment in segments.iter().rev() {
            self.newline();
            self.write_indent();
            self.postfix_segment(segment);
        }
        self.indent -= 1;
    }

    fn postfix_segment(&mut self, segment: &PostfixSegment<'_>) {
        match segment {
            PostfixSegment::Method {
                name,
                multiple,
                arguments,
            } => {
                self.write(".");
                self.write(name);
                if *multiple {
                    self.write("@");
                }
                self.arguments(arguments);
            }
            PostfixSegment::FunctionalBlock { method, body } => {
                self.write(".");
                self.write(method);
                self.write(" ");
                self.block(body);
            }
            PostfixSegment::Required => self.write("!"),
            PostfixSegment::Subscript { index } => {
                self.subscript(index);
            }
        }
    }

    fn subscript(&mut self, index: &Expression) {
        if let ExpressionKind::String(name) = index.kind()
            && is_dot_access_key(name)
        {
            self.write(".");
            self.write(name);
            return;
        }
        self.write("[");
        self.expression(index);
        self.write("]");
    }

    fn binary_child(&mut self, expression: &Expression, parent_precedence: u8, is_right: bool) {
        let child_precedence = expression_precedence(expression);
        let needs_parentheses = child_precedence < parent_precedence
            || (is_right && child_precedence == parent_precedence);
        if needs_parentheses {
            self.write("(");
        }
        self.expression_unwrapped(expression);
        if needs_parentheses {
            self.write(")");
        }
    }

    fn object(&mut self, fields: &[ObjectField]) {
        self.write("{");
        self.write_joined(fields, ", ", |formatter, field| {
            formatter.write(field.name());
            formatter.write(": ");
            formatter.expression(field.value());
        });
        self.write("}");
    }

    fn switch_arm(&mut self, arm: &SwitchArm, trailing_comma: bool) {
        self.write_indent();
        match arm.pattern() {
            SwitchPattern::Value(value) => self.expression(value),
            SwitchPattern::Wildcard => self.write("_"),
            SwitchPattern::Binding(name) => self.write(name),
        }
        self.write(" => ");
        self.expression(arm.value());
        if trailing_comma {
            self.write(",");
        }
        self.newline();
    }

    fn arguments(&mut self, arguments: &[Expression]) {
        self.write("(");
        self.write_joined(arguments, ", ", |formatter, argument| {
            formatter.expression(argument);
        });
        self.write(")");
    }

    fn block(&mut self, statements: &[Statement]) {
        self.write("{");
        if statements.is_empty() {
            self.write("}");
            return;
        }
        self.newline();
        self.indent += 1;
        for statement in statements {
            self.statement(statement);
        }
        self.indent -= 1;
        self.write_indent();
        self.write("}");
    }

    fn interpolated_string(&mut self, parts: &[InterpolatedStringPart]) {
        self.write("f\"");
        for part in parts {
            match part {
                InterpolatedStringPart::Literal(value) => self.escaped_string_content(value),
                InterpolatedStringPart::Expression(value) => {
                    self.write("{");
                    self.expression(value);
                    self.write("}");
                }
            }
        }
        self.write("\"");
    }

    fn string(&mut self, value: &str) {
        self.write("\"");
        self.escaped_string_content(value);
        self.write("\"");
    }

    fn escaped_string_content(&mut self, value: &str) {
        for character in value.chars() {
            match character {
                '\\' => self.write("\\\\"),
                '"' => self.write("\\\""),
                '\n' => self.write("\\n"),
                '\r' => self.write("\\r"),
                '\t' => self.write("\\t"),
                _ => self.output.push(character),
            }
        }
    }

    fn write_joined<T>(
        &mut self,
        values: &[T],
        separator: &str,
        mut write_value: impl FnMut(&mut Self, &T),
    ) {
        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                self.write(separator);
            }
            write_value(self, value);
        }
    }

    fn blank_line(&mut self) {
        if !self.output.ends_with("\n\n") {
            if !self.output.ends_with('\n') {
                self.output.push('\n');
            }
            self.output.push('\n');
        }
    }

    fn newline(&mut self) {
        self.output.push('\n');
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.write(INDENT);
        }
    }

    fn write(&mut self, value: &str) {
        self.output.push_str(value);
    }
}

enum PostfixSegment<'a> {
    Method {
        name: &'a str,
        multiple: bool,
        arguments: &'a [Expression],
    },
    FunctionalBlock {
        method: &'a str,
        body: &'a [Statement],
    },
    Required,
    Subscript {
        index: &'a Expression,
    },
}

fn postfix_chain_base<'a>(
    expression: &'a Expression,
    segments: &mut Vec<PostfixSegment<'a>>,
) -> &'a Expression {
    match expression.kind() {
        ExpressionKind::MethodCall {
            receiver,
            name,
            multiple,
            arguments,
        } => {
            segments.push(PostfixSegment::Method {
                name,
                multiple: *multiple,
                arguments,
            });
            postfix_chain_base(receiver, segments)
        }
        ExpressionKind::FunctionalBlock {
            receiver,
            method,
            body,
        } => {
            segments.push(PostfixSegment::FunctionalBlock { method, body });
            postfix_chain_base(receiver, segments)
        }
        ExpressionKind::Required(value) => {
            segments.push(PostfixSegment::Required);
            postfix_chain_base(value, segments)
        }
        ExpressionKind::Subscript { receiver, index } => {
            segments.push(PostfixSegment::Subscript { index });
            postfix_chain_base(receiver, segments)
        }
        _ => expression,
    }
}

fn binary_operator(operator: BinaryOperator) -> &'static str {
    match operator {
        BinaryOperator::Or => "||",
        BinaryOperator::And => "&&",
        BinaryOperator::Equal => "==",
        BinaryOperator::NotEqual => "!=",
        BinaryOperator::Less => "<",
        BinaryOperator::LessEqual => "<=",
        BinaryOperator::Greater => ">",
        BinaryOperator::GreaterEqual => ">=",
        BinaryOperator::Add => "+",
        BinaryOperator::Subtract => "-",
        BinaryOperator::Multiply => "*",
        BinaryOperator::Divide => "/",
        BinaryOperator::Remainder => "%",
    }
}

fn expression_precedence(expression: &Expression) -> u8 {
    match expression.kind() {
        ExpressionKind::IfElse { .. } | ExpressionKind::Lambda { .. } => PREC_LOWEST,
        ExpressionKind::Binary { operator, .. } => binary_precedence(*operator),
        ExpressionKind::Unary { .. } => PREC_UNARY,
        ExpressionKind::MethodCall { .. }
        | ExpressionKind::FunctionalBlock { .. }
        | ExpressionKind::Required(_)
        | ExpressionKind::Subscript { .. } => PREC_POSTFIX,
        _ => PREC_POSTFIX,
    }
}

fn postfix_chain_len(expression: &Expression) -> usize {
    match expression.kind() {
        ExpressionKind::MethodCall { receiver, .. }
        | ExpressionKind::FunctionalBlock { receiver, .. }
        | ExpressionKind::Subscript { receiver, .. } => 1 + postfix_chain_len(receiver),
        ExpressionKind::Required(value) => 1 + postfix_chain_len(value),
        _ => 0,
    }
}

fn is_dot_access_key(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphabetic() || byte == b'_')
}

fn binary_precedence(operator: BinaryOperator) -> u8 {
    match operator {
        BinaryOperator::Or => 1,
        BinaryOperator::And => 2,
        BinaryOperator::Equal | BinaryOperator::NotEqual => 3,
        BinaryOperator::Less
        | BinaryOperator::LessEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual => 4,
        BinaryOperator::Add | BinaryOperator::Subtract => 5,
        BinaryOperator::Multiply | BinaryOperator::Divide | BinaryOperator::Remainder => 6,
    }
}
