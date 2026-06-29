//! Runtime-facing interfaces for executing compiled extractor programs.

mod engine;
mod error;
mod evaluator;
mod functions;
mod html;
mod http;
mod lua;
mod scope;
mod serialize;
mod value;

pub use engine::{Engine, EngineBuilder, ExecutionResult, TestReport};
pub use error::{Diagnostic, ExecutionError, SerializationError, TypeError};
pub use html::{HtmlDocument, HtmlNode};
pub use http::{HttpClient, HttpMethod, HttpPolicy, HttpRequest, ReqwestHttpClient, Response};
pub use scope::Inputs;
pub use value::{Object, Value, ValueKind};

pub use html_extractor_core::{
    BinaryOperator, CompiledProgram, Expression, ExpressionKind, FunctionDefinition,
    FunctionIdentity, FunctionRequirement, InputConstraint, InputRequirement, ObjectField,
    Requirements, Span, Statement, StatementKind, TestBlock, TestInput, UnaryOperator, Visibility,
};

/// Read-only compiled script interface consumed by an execution engine.
pub trait Program {
    fn requirements(&self) -> &Requirements;
    fn source(&self) -> &str;
    fn statements(&self) -> &[Statement];
    fn functions(&self) -> &[FunctionDefinition];
    fn tests(&self) -> &[TestBlock];
}

impl Program for CompiledProgram {
    fn requirements(&self) -> &Requirements {
        self.requirements()
    }

    fn source(&self) -> &str {
        self.source()
    }

    fn statements(&self) -> &[Statement] {
        self.statements()
    }

    fn functions(&self) -> &[FunctionDefinition] {
        self.functions()
    }

    fn tests(&self) -> &[TestBlock] {
        self.tests()
    }
}
