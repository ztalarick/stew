//! Parser for Stew
//! 
//! This module provides functions to parse a source string using tree-sitter
//! and translate to a stew-core AST.

use miette::{Diagnostic, SourceSpan};
use thiserror::Error;
use tree_sitter;
use stew_core::ast::{self, Expression, Spanned};

/// Parse a stew source string into a program
pub fn parse(source: String) -> Result<tree_sitter::Tree, miette::Report> {
    let tree = parse_tree_sitter(&source).ok_or_else(|| miette::miette!("Failed to parse source"))?;
    // let checked_tree = check_tree(&tree, &source);
    // if checked_tree.is_err() {
    //     return 
    // }
    // cst_to_ast(&tree)
    check_tree(&tree, &source)

    // Parse as tree-sitter,
    // Walk the CST and convert into AST 
    
}

/// Check a tree-sitter-stew CST and report any errors
fn check_tree(tree: &tree_sitter::Tree, source: &String) -> Result<tree_sitter::Tree, miette::Report> {
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
#[diagnostic(code(stew::parser::syntax_error))]
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

/// Parses a source string into a tree-sitter CST
fn parse_tree_sitter(source: &String) -> Option<tree_sitter::Tree> {
    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_stew::LANGUAGE.into();
    parser.set_language(&language).expect("Failed to load Stew grammar");
    parser.parse(&source, None)
}

// fn cst_to_ast(cst: &tree_sitter::Tree) -> Result<ast::Program, miette::Report> {
//     convert_node(&cst.root_node());
// }

// fn convert_node(node: &tree_sitter::Node) -> ast::Program {
//     let mut result = ast::Program {statements: vec![]};
//     let converted_current_node: ast::Statement = match node.kind() {

//     };
//     result.statements.push(converted_current_node);
    
//     let mut cursor = node.walk();
//     let converted_children = node.children(&mut cursor).map(|child| {convert_node(&child)});
//     result.statements.append(converted_children.collect());

//     return result;
// }

// fn parse_expression(node: tree_sitter::Node, source: &[u8]) -> Result<ast::Spanned<ast::Expression>, ParseError> {
//     match node.kind() {
//         // Literals
//         "int_literal" => {
//             // TODO handle these unwraps
//             let text = node.utf8_text(source).unwrap();
//             let val = text.parse::<i64>().unwrap();
//             Ok(Spanned::new(Expression::Int(val), node.start_byte(), node.end_byte()))
//         },
//         "bool_literal"
//         "array_literal"

//         // Operations
//         "binary_expression"
//         "address_of"
//         "assignment_statement"

//         // Complex Expressions
//         "anonymous_function"
//         "array_indexing"
//         "block"
//         "choice_definition"
//         "choice_variant"
//         "event_declaration"
//         "expression"
//         // Types
//         "array_type"

//         // Control Flow
//         "break_expression"
//         "call_expression"

//         // TODO add error message
//         _ => Err(ParseError { err_span: (node.start_byte()..node.end_byte()).into() }),
//     }
// }


// things to fix:
// 1. whats going on with miette spans and ast spans, its confusing
// 2. add a message to the parse error or parse error type enum
// 3. Tree-sitter wrapper nodes with _expression 


// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn it_works() {
//         let result = add(2, 2);
//         assert_eq!(result, 4);
//     }
// }
