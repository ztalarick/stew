use tree_sitter;

use stew_core::ast;

use crate::{ParseError, types};

pub(crate) fn parse_typed_identifier(
    node: tree_sitter::Node,
    source: &[u8],
) -> Result<ast::TypedIdentifier, ParseError> {
    if node.kind() != "identifier_type_annotation" {
        let span = (node.start_byte()..node.end_byte()).into();
        return Err(ParseError {
            message: format!("Expected a type annotation, but found '{}'", node.kind()),
            label: "Expected something like 'name: int' here".to_string(),
            err_span: span,
        });
    }
    let is_mut = node.child_by_field_name("mutable").is_some();

    let name = node.get_child_field_text("name", source)?;

    let type_node = node.get_child_field("type")?;
    let var_type = types::parse_type(type_node, source)?;

    Ok(ast::TypedIdentifier {
        is_mut,
        name,
        var_type,
    })
}

/// Determines if a tree-sitter node should be skipped. (Comments and puncuation)
pub(crate) fn is_skippable(node: tree_sitter::Node) -> bool {
    !node.is_named() || node.kind() == "comment"
}

/// Helper method extensions for tree-sitter node
pub(crate) trait TSNodeExt<'a> {
    /// Gets a required child node by its field name.
    /// If you want an optional field use built-in node.child_by_field_name("field")
    ///
    /// Returns a `ParseError` if the field is missing from the CST.
    fn get_child_field(&self, field: &str) -> Result<tree_sitter::Node<'a>, ParseError>;

    /// Extracts the raw text of this node from the source code.
    ///
    /// Returns a `ParseError` if the underlying source code bytes are not
    /// valid UTF-8.
    fn get_text(&self, source: &[u8]) -> Result<String, ParseError>;

    /// A convenience method that gets a required child node by its field name,
    /// and immediately extracts its raw text from the source code.
    ///
    /// Equivalent to `node.get_child_field(field)?.get_text(source)`.
    fn get_child_field_text(&self, field: &str, source: &[u8]) -> Result<String, ParseError>;

    /// Extracts the raw text of this node and parses it into the desired type `T`.
    ///
    /// `T` can be any type that implements `std::str::FromStr` (e.g., `i64`,
    /// `usize`, `f64`, `bool`). Returns a `ParseError` if the text cannot be
    /// parsed into the target type.
    fn parse_text<T: std::str::FromStr>(&self, source: &[u8]) -> Result<T, ParseError>;
}

impl<'a> TSNodeExt<'a> for tree_sitter::Node<'a> {
    fn get_child_field(&self, field: &str) -> Result<tree_sitter::Node<'a>, ParseError> {
        self.child_by_field_name(field).ok_or_else(|| ParseError {
            message: format!(
                "Internal Error: Missing expected field '{}' on '{}'",
                field,
                self.kind()
            ),
            label: "Compiler error: grammar mismatch".to_string(),
            err_span: (self.start_byte()..self.end_byte()).into(),
        })
    }

    fn get_text(&self, source: &[u8]) -> Result<String, ParseError> {
        self.utf8_text(source)
            .map(|s| s.to_string())
            .map_err(|_| ParseError {
                message: "Source code is not valid UTF-8".to_string(),
                label: "Encoding error here".to_string(),
                err_span: (self.start_byte()..self.end_byte()).into(),
            })
    }

    fn get_child_field_text(&self, field: &str, source: &[u8]) -> Result<String, ParseError> {
        let child = self.get_child_field(field)?;
        child.get_text(source)
    }

    fn parse_text<T: std::str::FromStr>(&self, source: &[u8]) -> Result<T, ParseError> {
        let text = self.get_text(source)?;
        text.parse::<T>().map_err(|_| ParseError {
            message: format!("Failed to parse '{}' into desired type", text),
            label: "Invalid format".to_string(),
            err_span: (self.start_byte()..self.end_byte()).into(),
        })
    }
}
