//! Compile extractor source into embedded validated programs.

use std::{env, fs, path::PathBuf};

use html_extractor_core::{SourceText, compile};
use proc_macro::TokenStream;
use quote::quote;
use syn::{LitStr, parse_macro_input};

#[proc_macro]
pub fn extractor_program(input: TokenStream) -> TokenStream {
    let source = parse_macro_input!(input as LitStr);
    expand(source.value(), "<inline extractor>", None, source)
}

#[proc_macro]
pub fn extractor_program_file(input: TokenStream) -> TokenStream {
    let path = parse_macro_input!(input as LitStr);
    let manifest = match env::var("CARGO_MANIFEST_DIR") {
        Ok(manifest) => PathBuf::from(manifest),
        Err(error) => {
            return syn::Error::new(
                path.span(),
                format!("CARGO_MANIFEST_DIR is unavailable: {error}"),
            )
            .to_compile_error()
            .into();
        }
    };
    let relative = path.value();
    let resolved = manifest.join(&relative);
    let source = match fs::read_to_string(&resolved) {
        Ok(source) => source,
        Err(error) => {
            return syn::Error::new(
                path.span(),
                format!("failed to read extractor file `{relative}`: {error}"),
            )
            .to_compile_error()
            .into();
        }
    };
    expand(source, &relative, Some(resolved), path)
}

fn expand(source: String, name: &str, tracked_path: Option<PathBuf>, span: LitStr) -> TokenStream {
    let program = match compile(&source) {
        Ok(program) => program,
        Err(error) => {
            let source_text = SourceText::new(source);
            let diagnostics = error.diagnostics().iter().map(|diagnostic| {
                let location = source_text.location(diagnostic.primary_span().start());
                let message = if let Some(location) = location {
                    format!(
                        "{name}:{}:{}: {}",
                        location.line(),
                        location.column(),
                        diagnostic.message()
                    )
                } else {
                    format!("{name}: {}", diagnostic.message())
                };
                syn::Error::new(span.span(), message).to_compile_error()
            });
            return quote!(#(#diagnostics)*).into();
        }
    };

    let encoded = html_extractor_core::__private::__encode_compiled(&program);
    let bytes = encoded.iter();
    let source_literal = LitStr::new(&source, span.span());
    let tracking = tracked_path.map(|path| {
        let path = LitStr::new(&path.to_string_lossy(), span.span());
        quote!(
            const _: &str = include_str!(#path);
        )
    });

    quote!({
        #tracking
        ::html_extractor::__private::__decode_compiled(#source_literal, &[#(#bytes),*])
    })
    .into()
}
