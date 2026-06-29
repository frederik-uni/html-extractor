//! Compile extractor scripts at runtime or during Rust compilation.
//!
//! Runtime compilation:
//!
//! ```
//! let program = html_extractor::compile("title = html.css(\"h1\")!.text()")?;
//! assert_eq!(program.requirements().inputs()[0].name(), "html");
//! # Ok::<(), html_extractor::CompileError>(())
//! ```
//!
//! Compile-time compilation embeds a validated program rather than reparsing
//! extractor source when the application runs:
//!
//! ```
//! let program = html_extractor::extractor_program!(
//!     r#"title = html.css("h1")!.text()"#
//! );
//! assert_eq!(program.statements().len(), 1);
//! ```
//!
//! Invalid extractor programs fail while compiling the Rust crate:
//!
//! ```compile_fail
//! let _ = html_extractor::extractor_program!("value = 1!");
//! ```
//!
//! `include_str!` is intentionally replaced by the file-specific macro:
//!
//! ```compile_fail
//! let _ = html_extractor::extractor_program!(include_str!("extractor.script"));
//! ```
//!
//! Missing extractor files are also Rust compilation failures:
//!
//! ```compile_fail
//! let _ = html_extractor::extractor_program_file!("missing.extractor");
//! ```
//!
//! A macro-generated program can be retained globally with `LazyLock`:
//!
//! ```
//! use std::sync::LazyLock;
//! static PROGRAM: LazyLock<html_extractor::CompiledProgram> = LazyLock::new(|| {
//!     html_extractor::extractor_program!("value = host_value")
//! });
//! assert_eq!(PROGRAM.requirements().inputs()[0].name(), "host_value");
//! ```

pub use html_extractor_core::*;
pub use html_extractor_macros::{extractor_program, extractor_program_file};

#[doc(hidden)]
pub mod __private {
    pub use html_extractor_core::__private::*;
}
