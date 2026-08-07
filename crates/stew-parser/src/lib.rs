//! Parser for Stew
//!
//! This module provides functions to parse a source string using tree-sitter
//! and translate to a stew-core AST.

mod common;
mod error;
mod expression;
mod machine;
mod statement;
mod types;

pub use error::{ParseError, ParseErrors};

use std::vec;

use stew_core::ast;
use tree_sitter;

/// Parse a stew source string into a program
pub fn parse(source: String) -> Result<ast::Program, miette::Report> {
    let tree = parse_cst(&source).ok_or_else(|| miette::miette!("Failed to parse source"))?;

    check_tree(&tree, &source)?;

    let root_node = tree.root_node();
    if root_node.kind() != "source_file" {
        return Err(miette::miette!("Expected root_node to be a source_file"));
    }
    let program = parse_program(root_node, source.as_bytes())
        .map_err(|err| miette::Report::from(err).with_source_code(source))?;
    Ok(program)
}

/// Check a tree-sitter-stew CST and report any errors
fn check_tree(
    tree: &tree_sitter::Tree,
    source: &String,
) -> Result<tree_sitter::Tree, miette::Report> {
    if tree.root_node().has_error() {
        let mut cursor = tree.root_node().walk();
        let mut queue = vec![tree.root_node()];
        let mut all_errors = Vec::new();

        // BFS
        while let Some(node) = queue.pop() {
            // Check current node for errors
            if node.is_error() || node.is_missing() {
                let span = (node.start_byte()..node.end_byte()).into();
                all_errors.push(ParseError {
                    message: "Syntax error detected by tree-sitter".to_string(),
                    label: "invalid syntax here".to_string(),
                    err_span: span,
                });
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
            }
            .into());
        }
    }
    Ok(tree.clone())
}

/// Parses a source string into a tree-sitter CST
pub fn parse_cst(source: &String) -> Option<tree_sitter::Tree> {
    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_stew::LANGUAGE.into();
    parser
        .set_language(&language)
        .expect("Failed to load Stew grammar");
    parser.parse(&source, None)
}

fn parse_program(node: tree_sitter::Node, source: &[u8]) -> Result<ast::Program, ParseError> {
    let mut cursor = node.walk();
    let statements = node
        .children(&mut cursor)
        .filter(|child| !common::is_skippable(*child))
        .map(|child| statement::parse_statement(child, &source))
        .collect::<Result<Vec<_>, ParseError>>()?;
    Ok(ast::Program { statements })
}

// things to fix:
// 1. add tests :)
