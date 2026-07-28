//! Parser for Loom
//! 
//! This module provides functions to parse a source string using tree-sitter
//! and translate to a loom-core AST.

use miette::{Diagnostic, SourceSpan};
use thiserror::Error;
use tree_sitter::{Parser, Tree};

/// Parse a loom source string into a tree-sitter-loom CST
pub fn parse(source: String) -> Result<Tree, miette::Report> {
    let mut parser = Parser::new();
    let language = tree_sitter_loom::LANGUAGE.into();
    parser.set_language(&language).expect("Failed to load Loom grammar");
    let tree = parser.parse(&source, None).ok_or_else(|| miette::miette!("Failed to parse source"))?;

    return check_tree(&tree, &source);
}

/// Check a tree-sitter-loom CST and report any errors
fn check_tree(tree: &Tree, source: &String) -> Result<Tree, miette::Report> {
    if tree.root_node().has_error() {
        let mut cursor = tree.root_node().walk();
        let mut queue = vec![tree.root_node()];
        let mut all_errors = Vec::new();

        // BFS
        while let Some(node) = queue.pop() {
            // Check current node for errors
            if node.is_error() || node.is_missing() {
                let span = (node.start_byte()..node.end_byte()).into();
                all_errors.push(ParseError { err_span: span });
                continue;
            }

            // Also check the children
            for child in node.children(&mut cursor) {
                if child.has_error() {
                    queue.push(child);
                }
            }
        }
        if !all_errors.is_empty() {
            return Err(ParseErrors {
                src: source.clone(),
                errors: all_errors,
            }.into());
        }
    }
    Ok(tree.clone())
}

#[derive(Error, Debug, Diagnostic)]
#[error("Found {} syntax error(s)", errors.len())]
pub struct ParseErrors {
    #[source_code]
    pub src: String,
    
    #[related]
    pub errors: Vec<ParseError>,
}

#[derive(Error, Debug, Diagnostic)]
#[error("Syntax Error")]
#[diagnostic(code(loom::parser::syntax_error))]
pub struct ParseError {
    #[label("invalid syntax here")]
    pub err_span: SourceSpan,
}

pub fn format_sexp(sexp: &str) -> String {
    let mut indent = 0;
    let mut pretty = String::new();
    let mut in_string = false;

    for c in sexp.chars() {
        match c {
            // Don't format inside string literals
            '"' => {
                in_string = !in_string;
                pretty.push('"');
            }
            '(' if !in_string => {
                pretty.push('\n');
                pretty.push_str(&"  ".repeat(indent)); // 2 spaces per indent
                pretty.push('(');
                indent += 1;
            }
            ')' if !in_string => {
                indent = indent.saturating_sub(1);
                pretty.push(')');
            }
            // Skip the space immediately following a closing paren, 
            // the newlines will handle spacing
            ' ' if !in_string && pretty.ends_with(')') => {}
            _ => pretty.push(c),
        }
    }
        
    // Remove the very first newline that gets added
    pretty.trim_start().to_string()
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn it_works() {
//         let result = add(2, 2);
//         assert_eq!(result, 4);
//     }
// }
