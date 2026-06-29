use crate::Span;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Program {
    functions: Vec<FunctionDefinition>,
    tests: Vec<TestBlock>,
    statements: Vec<Statement>,
    span: Span,
}

impl Program {
    pub(crate) fn new(
        functions: Vec<FunctionDefinition>,
        tests: Vec<TestBlock>,
        statements: Vec<Statement>,
        span: Span,
    ) -> Self {
        Self {
            functions,
            tests,
            statements,
            span,
        }
    }

    #[must_use]
    pub fn functions(&self) -> &[FunctionDefinition] {
        &self.functions
    }

    #[must_use]
    pub fn tests(&self) -> &[TestBlock] {
        &self.tests
    }

    pub(crate) fn functions_mut(&mut self) -> &mut [FunctionDefinition] {
        &mut self.functions
    }

    pub(crate) fn tests_mut(&mut self) -> &mut [TestBlock] {
        &mut self.tests
    }

    #[must_use]
    pub fn statements(&self) -> &[Statement] {
        &self.statements
    }

    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    pub(crate) fn statements_mut(&mut self) -> &mut [Statement] {
        &mut self.statements
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TestBlock {
    inputs: Vec<TestInput>,
    body: Vec<Statement>,
    runs_script: bool,
    span: Span,
}

impl TestBlock {
    pub(crate) const fn new(
        inputs: Vec<TestInput>,
        body: Vec<Statement>,
        runs_script: bool,
        span: Span,
    ) -> Self {
        Self {
            inputs,
            body,
            runs_script,
            span,
        }
    }

    #[must_use]
    pub fn inputs(&self) -> &[TestInput] {
        &self.inputs
    }

    #[must_use]
    pub fn body(&self) -> &[Statement] {
        &self.body
    }

    pub(crate) fn body_mut(&mut self) -> &mut [Statement] {
        &mut self.body
    }

    #[must_use]
    pub const fn runs_script(&self) -> bool {
        self.runs_script
    }

    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TestInput {
    name: String,
    visibility: Visibility,
    value: Expression,
    span: Span,
}

impl TestInput {
    pub(crate) const fn new(
        name: String,
        visibility: Visibility,
        value: Expression,
        span: Span,
    ) -> Self {
        Self {
            name,
            visibility,
            value,
            span,
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn visibility(&self) -> Visibility {
        self.visibility
    }

    #[must_use]
    pub const fn value(&self) -> &Expression {
        &self.value
    }

    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct FunctionDefinition {
    name: String,
    parameters: Vec<String>,
    body: Vec<Statement>,
    span: Span,
}

impl FunctionDefinition {
    pub(crate) const fn new(
        name: String,
        parameters: Vec<String>,
        body: Vec<Statement>,
        span: Span,
    ) -> Self {
        Self {
            name,
            parameters,
            body,
            span,
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn parameters(&self) -> &[String] {
        &self.parameters
    }

    #[must_use]
    pub fn body(&self) -> &[Statement] {
        &self.body
    }

    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    pub(crate) fn body_mut(&mut self) -> &mut [Statement] {
        &mut self.body
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Statement {
    pub kind: StatementKind,
    span: Span,
}

impl Statement {
    pub(crate) const fn new(kind: StatementKind, span: Span) -> Self {
        Self { kind, span }
    }

    #[must_use]
    pub const fn kind(&self) -> &StatementKind {
        &self.kind
    }

    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    pub(crate) fn kind_mut(&mut self) -> &mut StatementKind {
        &mut self.kind
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum StatementKind {
    Assignment {
        name: String,
        operator: AssignmentOperator,
        visibility: Visibility,
        value: Expression,
    },
    ComputedAssignment {
        name: Expression,
        value: Expression,
    },
    Return(Expression),
    While {
        condition: Expression,
        body: Vec<Statement>,
    },
    Loop {
        body: Vec<Statement>,
    },
    Break,
    Expression(Expression),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AssignmentOperator {
    Assign,
    Add,
    Subtract,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Visibility {
    Public,
    Private,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Expression {
    pub kind: ExpressionKind,
    span: Span,
}

impl Expression {
    pub(crate) const fn new(kind: ExpressionKind, span: Span) -> Self {
        Self { kind, span }
    }

    #[must_use]
    pub const fn kind(&self) -> &ExpressionKind {
        &self.kind
    }

    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    pub(crate) fn kind_mut(&mut self) -> &mut ExpressionKind {
        &mut self.kind
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum ExpressionKind {
    Null,
    Boolean(bool),
    Number(String),
    String(String),
    InterpolatedString(Vec<InterpolatedStringPart>),
    Array(Vec<Expression>),
    Object(Vec<ObjectField>),
    Variable(String),
    Identifier(String),
    FunctionCall {
        name: String,
        arguments: Vec<Expression>,
    },
    QualifiedFunctionCall {
        module: String,
        name: String,
        arguments: Vec<Expression>,
    },
    CoreCall {
        name: String,
        arguments: Vec<Expression>,
    },
    MethodCall {
        receiver: Box<Expression>,
        name: String,
        multiple: bool,
        arguments: Vec<Expression>,
    },
    Lambda {
        parameters: Vec<String>,
        body: Box<Expression>,
    },
    FunctionalBlock {
        receiver: Box<Expression>,
        method: String,
        body: Vec<Statement>,
    },
    Required(Box<Expression>),
    Unary {
        operator: UnaryOperator,
        operand: Box<Expression>,
    },
    Binary {
        left: Box<Expression>,
        operator: BinaryOperator,
        right: Box<Expression>,
    },
    IfElse {
        condition: Box<Expression>,
        then_branch: Box<Expression>,
        else_branch: Box<Expression>,
    },
    BlockIf {
        condition: Box<Expression>,
        then_branch: Vec<Statement>,
        else_branch: Option<Vec<Statement>>,
    },
    Switch {
        target: Box<Expression>,
        arms: Vec<SwitchArm>,
    },
    Subscript {
        receiver: Box<Expression>,
        index: Box<Expression>,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum InterpolatedStringPart {
    Literal(String),
    Expression(Expression),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ObjectField {
    name: String,
    value: Expression,
    span: Span,
}

impl ObjectField {
    pub(crate) const fn new(name: String, value: Expression, span: Span) -> Self {
        Self { name, value, span }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn value(&self) -> &Expression {
        &self.value
    }

    pub(crate) fn value_mut(&mut self) -> &mut Expression {
        &mut self.value
    }

    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SwitchArm {
    pattern: SwitchPattern,
    value: Expression,
    span: Span,
}

impl SwitchArm {
    pub(crate) const fn new(pattern: SwitchPattern, value: Expression, span: Span) -> Self {
        Self {
            pattern,
            value,
            span,
        }
    }

    #[must_use]
    pub const fn pattern(&self) -> &SwitchPattern {
        &self.pattern
    }

    pub(crate) fn pattern_mut(&mut self) -> &mut SwitchPattern {
        &mut self.pattern
    }

    #[must_use]
    pub const fn value(&self) -> &Expression {
        &self.value
    }

    pub(crate) fn value_mut(&mut self) -> &mut Expression {
        &mut self.value
    }

    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum SwitchPattern {
    Value(Expression),
    Wildcard,
    Binding(String),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum UnaryOperator {
    Negate,
    Not,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BinaryOperator {
    Or,
    And,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}
