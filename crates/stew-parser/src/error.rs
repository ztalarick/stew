use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
#[error("Found {} syntax error(s)", errors.len())]
pub struct ParseErrors {
    #[source_code]
    pub src: String,

    #[related]
    pub errors: Vec<ParseError>,
}

#[derive(Error, Debug, Diagnostic)]
#[error("Syntax Error: {message}")]
#[diagnostic(code(stew::parser::syntax_error))]
pub struct ParseError {
    /// Error message
    pub message: String,

    #[label("{label}")]
    /// Location of error in source
    pub err_span: SourceSpan,
    /// Message which will point to the span
    pub label: String,
}
