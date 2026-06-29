//! Shared compiler implementation for HTML Extractor.

mod diagnostic;
mod formatter;
mod functions;
mod program;
mod requirements;
mod source;
mod syntax;
mod validation;

pub use diagnostic::{CompileError, Diagnostic};
pub use formatter::format;
pub use functions::{BUILTIN_FUNCTIONS, BuiltinFunction, BuiltinFunctionKind, builtin_function};
pub use program::{CompiledProgram, ProgramView, compile};
pub use requirements::{
    FunctionIdentity, FunctionRequirement, InputConstraint, InputRequirement, Requirements,
};
pub use source::{SourceLocation, SourceText, Span};
pub use syntax::{
    AssignmentOperator, BinaryOperator, Expression, ExpressionKind, FunctionDefinition,
    InterpolatedStringPart, InterpolatedTokenPart, ObjectField, Program, Statement, StatementKind,
    SwitchArm, SwitchPattern, TestBlock, TestInput, Token, TokenKind, UnaryOperator, Visibility,
    lex, parse,
};

#[doc(hidden)]
pub mod __private {
    pub use crate::program::{__decode_compiled, __encode_compiled};
}
