//! Parser for Stew
//!
//! This module provides functions to parse a source string using tree-sitter
//! and translate to a stew-core AST.

use std::vec;

use miette::{Diagnostic, SourceSpan};
use stew_core::ast::{self, Spanned};
use thiserror::Error;
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
        .filter(|child| !is_skippable(&child))
        .map(|child| parse_statement(child, &source))
        .collect::<Result<Vec<_>, ParseError>>()?;
    Ok(ast::Program { statements })
}

fn parse_statement(
    node: tree_sitter::Node,
    source: &[u8],
) -> Result<ast::Spanned<ast::Statement>, ParseError> {
    match node.kind() {
        "expression_statement" => {
            let child_node = node.child(0).unwrap();

            let expr = parse_expression(child_node, source)?;

            Ok(ast::Spanned {
                node: ast::Statement::ExprStatement(Box::new(expr)),
                span: ast::Span {
                    start: node.start_byte(),
                    end: node.end_byte(),
                },
            })
        }

        // control flow
        "function_definition" => {
            let name_node = node.child_by_field_name("name").unwrap();
            let name = name_node.utf8_text(source).unwrap().to_string();

            let return_type_node = node.child_by_field_name("return_type").unwrap();
            let return_type = parse_type(&return_type_node, source)?;

            let body_node = node.child_by_field_name("body").unwrap();
            let body = Box::new(parse_expression(body_node, source)?);

            let mut cursor = node.walk();
            let parameters = node
                .children_by_field_name("parameters", &mut cursor)
                .map(|child| parse_typed_identifier(&child, source))
                .collect::<Result<Vec<_>, _>>()?;

            Ok(ast::Spanned::new(
                ast::Statement::Function {
                    name,
                    parameters,
                    return_type,
                    body,
                },
                node.start_byte(),
                node.end_byte(),
            ))
        }
        "if_else" | "block" | "for_loop" | "while_loop" | "match_expression" => {
            let expr = parse_expression(node, source)?;
            Ok(ast::Spanned::new(
                ast::Statement::ExprStatement(Box::new(expr)),
                node.start_byte(),
                node.end_byte(),
            ))
        }

        // variable
        "let_declaration" => {
            let identifier_type_node = node.child_by_field_name("identifier_type").unwrap();
            let declaration = parse_typed_identifier(&identifier_type_node, source)?;

            let value_node = node.child_by_field_name("value").unwrap();
            let value = Box::new(parse_expression(value_node, source)?);

            Ok(Spanned::new(
                ast::Statement::LetDeclaration { declaration, value },
                node.start_byte(),
                node.end_byte(),
            ))
        }
        "assignment_statement" => {
            let value_node = node.child_by_field_name("right").unwrap();
            let value = Box::new(parse_expression(value_node, source)?);

            let assign_target_node = node.child_by_field_name("left").unwrap();
            let target = parse_assign_target(assign_target_node, source)?;

            Ok(Spanned::new(
                ast::Statement::Assignment { target, value },
                node.start_byte(),
                node.end_byte(),
            ))
        }

        //advanced definitions
        "struct" => {
            let name_node = node.child_by_field_name("name").unwrap();
            let name = name_node.utf8_text(source).unwrap().to_string();

            let mut cursor = node.walk();
            let field_types = node
                .children_by_field_name("fields", &mut cursor)
                .map(|child| parse_typed_identifier(&child, source))
                .collect::<Result<Vec<_>, _>>()?;

            Ok(Spanned::new(
                ast::Statement::StructDefinition { name, field_types },
                node.start_byte(),
                node.end_byte(),
            ))
        }
        "choice_definition" => {
            let name_node = node.child_by_field_name("name").unwrap();
            let name = name_node.utf8_text(source).unwrap().to_string();

            let mut cursor = node.walk();
            let choices = node
                .children_by_field_name("choice_variants", &mut cursor)
                .map(|child| parse_choice_variant(&child, source))
                .collect::<Result<Vec<_>, _>>()?;

            Ok(Spanned::new(
                ast::Statement::ChoiceDefinition { name, choices },
                node.start_byte(),
                node.end_byte(),
            ))
        }
        "machine_definition" => {
            let name_node = node.child_by_field_name("name").unwrap();
            let name = name_node.utf8_text(source).unwrap().to_string();

            let machine_body_node = node.child_by_field_name("body").unwrap();

            let mut cursor = node.walk();
            let events = machine_body_node
                .children_by_field_name("events", &mut cursor)
                .map(|child| parse_machine_variant(&child, source))
                .collect::<Result<Vec<_>, _>>()?;

            cursor.reset(machine_body_node);
            let states = machine_body_node
                .children_by_field_name("states", &mut cursor)
                .map(|child| parse_machine_variant(&child, source))
                .collect::<Result<Vec<_>, _>>()?;

            // how can I error if more than one initilzier is specified?
            cursor.reset(machine_body_node);

            let init = {
                let mut init_iterator =
                    machine_body_node.children_by_field_name("initializers", &mut cursor);
                let init_node = match init_iterator.next() {
                    Some(child) => parse_machine_initializer(&child, source)?,
                    None => {
                        let span =
                            (machine_body_node.start_byte()..machine_body_node.end_byte()).into();
                        return Err(ParseError {
                            message: "A state machine must have an initialization block"
                                .to_string(),
                            label: "Add an `init` block here".to_string(),
                            err_span: span,
                        });
                    }
                };
                if let Some(second_child) = init_iterator.next() {
                    let span = (second_child.start_byte()..second_child.end_byte()).into();
                    return Err(ParseError {
                        message: "A state machine can only have one initialization block"
                            .to_string(),
                        label: "Remove this duplicate init block".to_string(),
                        err_span: span,
                    });
                }
                init_node
            };

            cursor.reset(machine_body_node);
            let state_handlers = machine_body_node
                .children_by_field_name("state_behaviors", &mut cursor)
                .map(|child| parse_machine_state_handler(&child, source))
                .collect::<Result<Vec<_>, _>>()?;

            Ok(Spanned::new(
                ast::Statement::Machine {
                    name,
                    init,
                    events,
                    states,
                    state_handlers,
                },
                node.start_byte(),
                node.end_byte(),
            ))
        }

        // ==========================================
        // FALLBACK
        // ==========================================
        unknown => Err(ParseError {
            message: format!("The parser doesn't know how to handle '{}'", unknown),
            label: "Unknown syntax node".to_string(),
            err_span: (node.start_byte()..node.end_byte()).into(),
        }),
    }
}

fn parse_expression(
    node: tree_sitter::Node,
    source: &[u8],
) -> Result<ast::Spanned<ast::Expression>, ParseError> {
    match node.kind() {
        // ==========================================
        // LITERALS
        // ==========================================
        "int_literal" => {
            let text = node.utf8_text(source).unwrap();
            let val = text.parse::<i64>().unwrap();
            Ok(ast::Spanned::new(
                ast::Expression::Int(val),
                node.start_byte(),
                node.end_byte(),
            ))
        }
        "float_literal" => {
            let text = node.utf8_text(source).unwrap();
            let val = text.parse::<f64>().unwrap();
            Ok(ast::Spanned::new(
                ast::Expression::Float(val),
                node.start_byte(),
                node.end_byte(),
            ))
        }
        "bool_literal" => {
            let text = node.utf8_text(source).unwrap();
            let val = text.parse::<bool>().unwrap();
            Ok(ast::Spanned::new(
                ast::Expression::Bool(val),
                node.start_byte(),
                node.end_byte(),
            ))
        }
        "string_literal" => {
            let text = node.utf8_text(source).unwrap();
            Ok(ast::Spanned::new(
                ast::Expression::Str(text.to_string()),
                node.start_byte(),
                node.end_byte(),
            ))
        }
        "array_literal" => {
            let mut cursor = node.walk();
            let array = node
                .children(&mut cursor)
                .map(|child| parse_expression(child, source).unwrap()) // TODO fix unwrap
                .collect();

            Ok(ast::Spanned::new(
                ast::Expression::Array(array),
                node.start_byte(),
                node.end_byte(),
            ))
        }
        "tuple_literal" => {
            let mut cursor = node.walk();
            let tuple = node
                .children(&mut cursor)
                .map(|child| parse_expression(child, source))
                .collect::<Result<Vec<_>, _>>()?;

            Ok(ast::Spanned::new(
                ast::Expression::Tuple(tuple),
                node.start_byte(),
                node.end_byte(),
            ))
        }
        "struct_literal" => {
            let mut fields = vec![];
            let mut cursor = node.walk();
            let children = node.children(&mut cursor);
            for child in children {
                if child.kind() == "struct_field_initializer" {
                    let field_node = child.child_by_field_name("field").unwrap();
                    let value_node = child.child_by_field_name("value").unwrap();

                    let field_name = field_node.utf8_text(source).unwrap().to_string();
                    let value_expr = parse_expression(value_node, source)?;
                    fields.push((field_name, value_expr))
                }
            }

            Ok(ast::Spanned::new(
                ast::Expression::Struct { fields },
                node.start_byte(),
                node.end_byte(),
            ))
        }

        // ==========================================
        // COMPLEX EXPRESSIONS
        // ==========================================
        "unary_expression" => {
            let operator_node = node.child_by_field_name("operator").unwrap();
            let operand_node = node.child_by_field_name("argument").unwrap();

            let op = match operator_node.utf8_text(source).unwrap() {
                "-" => ast::UnaryOperation::Negative,
                "not" => ast::UnaryOperation::Not,
                _ => {
                    let span = (node.start_byte()..node.end_byte()).into();
                    return Err(ParseError {
                        message: "Unknown unary operator".to_string(),
                        label: "Expected '-' or 'not'".to_string(),
                        err_span: span,
                    });
                }
            };

            let operand = parse_expression(operand_node, source)?;

            Ok(ast::Spanned::new(
                ast::Expression::Unary {
                    op,
                    operand: Box::new(operand),
                },
                node.start_byte(),
                node.end_byte(),
            ))
        }
        "binary_expression" => {
            let left_node = node.child_by_field_name("left").unwrap();
            let operator_node = node.child_by_field_name("operator").unwrap();
            let right_node = node.child_by_field_name("left").unwrap();

            let op = match operator_node.utf8_text(source).unwrap() {
                "*" => ast::BinaryOperation::Multiply,
                "/" => ast::BinaryOperation::Divide,
                "%" => ast::BinaryOperation::Modulo,
                "or" => ast::BinaryOperation::BooleanOr,
                "and" => ast::BinaryOperation::BooleanAnd,
                "==" => ast::BinaryOperation::Equals,
                ">" => ast::BinaryOperation::GreaterThan,
                "<" => ast::BinaryOperation::LessThan,
                ">=" => ast::BinaryOperation::GreaterThanEquals,
                "<=" => ast::BinaryOperation::LessThanEquals,
                "!=" => ast::BinaryOperation::NotEquals,
                "<<" => ast::BinaryOperation::LeftShift,
                ">>" => ast::BinaryOperation::RightShift,
                "&" => ast::BinaryOperation::BitwiseAnd,
                "|" => ast::BinaryOperation::BitwiseOr,
                "+" => ast::BinaryOperation::Add,
                "-" => ast::BinaryOperation::Subtract,
                _ => {
                    let span = (node.start_byte()..node.end_byte()).into();
                    return Err(ParseError {
                        message: "Unknown binary operator".to_string(),
                        label: "Invalid operator".to_string(),
                        err_span: span,
                    });
                }
            };

            let left = parse_expression(left_node, source)?;
            let right = parse_expression(right_node, source)?;
            Ok(ast::Spanned::new(
                ast::Expression::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                node.start_byte(),
                node.end_byte(),
            ))
        }
        "array_indexing" => {
            let array_node = node.child_by_field_name("array").unwrap();
            let index_node = node.child_by_field_name("index").unwrap();

            let array = parse_expression(array_node, source)?;
            let index = parse_expression(index_node, source)?;

            Ok(ast::Spanned::new(
                ast::Expression::ArrayIndex {
                    target: Box::new(array),
                    index: Box::new(index),
                },
                node.start_byte(),
                node.end_byte(),
            ))
        }
        "field_access" => {
            let object_node = node.child_by_field_name("object").unwrap();
            let field_node = node.child_by_field_name("field").unwrap();

            let target = parse_expression(object_node, source)?;

            let field = match parse_expression(field_node, source)?.node {
                ast::Expression::Identifier(label) => ast::FieldName::Named(label),
                // TODO fix this i64 -> usize conversion unwrapping!
                ast::Expression::Int(index) => ast::FieldName::Index(index.try_into().unwrap()),
                _ => {
                    let span = (node.start_byte()..node.end_byte()).into();
                    return Err(ParseError {
                        message: "Invalid field access".to_string(),
                        label: "Expected an identifier or an integer index".to_string(),
                        err_span: span,
                    });
                }
            };

            Ok(ast::Spanned::new(
                ast::Expression::Field {
                    target: Box::new(target),
                    field,
                },
                node.start_byte(),
                node.end_byte(),
            ))
        }
        "address_of" => {
            let is_mut = node.child_by_field_name("mutable").is_some();

            let value_node = node.child_by_field_name("value").unwrap();
            let value = parse_expression(value_node, source)?;

            Ok(ast::Spanned::new(
                ast::Expression::AddressOf {
                    is_mut,
                    target: Box::new(value),
                },
                node.start_byte(),
                node.end_byte(),
            ))
        }
        "wildcard_expression" => Ok(ast::Spanned::new(
            ast::Expression::Wildcard,
            node.start_byte(),
            node.end_byte(),
        )),

        // ==========================================
        // CONTROL FLOW & FUNCTIONS
        // ==========================================
        "identifier" => {
            let identifier = node.utf8_text(source).unwrap().to_string();
            Ok(ast::Spanned::new(
                ast::Expression::Identifier(identifier),
                node.start_byte(),
                node.end_byte(),
            ))
        }
        "block" => {
            let mut cursor = node.walk();
            let statements = node
                .children_by_field_name("statements_nodes", &mut cursor)
                .filter(|child| !is_skippable(&child))
                .map(|child| parse_statement(child, source))
                .collect::<Result<Vec<_>, _>>()?;

            let final_expression_node = node.child_by_field_name("final_expression");
            let final_expression = match final_expression_node {
                Some(expr_node) => Some(Box::new(parse_expression(expr_node, source)?)),
                None => None,
            };

            Ok(ast::Spanned::new(
                ast::Expression::Block {
                    statements,
                    final_expression,
                },
                node.start_byte(),
                node.end_byte(),
            ))
        }
        "if_else" => {
            let condition_node = node.child_by_field_name("condition").unwrap();
            let consequence_node = node.child_by_field_name("consequence").unwrap();

            let condition = Box::new(parse_expression(condition_node, source)?);
            let consequence = Box::new(parse_expression(consequence_node, source)?);

            let alternative_node = node.child_by_field_name("alternative");
            let alternative = match alternative_node {
                Some(expr_node) => Some(Box::new(parse_expression(expr_node, source)?)),
                None => None,
            };

            Ok(ast::Spanned::new(
                ast::Expression::If {
                    condition,
                    consequence,
                    alternative,
                },
                node.start_byte(),
                node.end_byte(),
            ))
        }
        "match_expression" => {
            let match_value_node = node.child_by_field_name("value").unwrap();
            let match_value = Box::new(parse_expression(match_value_node, source)?);

            let mut cursor = node.walk();
            let cases = node
                .children_by_field_name("cases", &mut cursor)
                .map(|child| parse_match_case(&child, source))
                .collect::<Result<Vec<_>, _>>()?;

            Ok(ast::Spanned::new(
                ast::Expression::Match {
                    target: match_value,
                    options: cases,
                },
                node.start_byte(),
                node.end_byte(),
            ))
        }
        "for_loop" => {
            let mut cursor = node.walk();
            let bindings = node
                .children_by_field_name("bindings", &mut cursor)
                .map(|child| parse_typed_identifier(&child, source))
                .collect::<Result<Vec<_>, _>>()?;

            let iterator_node = node.child_by_field_name("iterator").unwrap();
            let iterator = Box::new(parse_expression(iterator_node, source)?);

            let body_node = node.child_by_field_name("body").unwrap();
            let body = Box::new(parse_expression(body_node, source)?);

            Ok(ast::Spanned::new(
                ast::Expression::For {
                    bindings,
                    iterator,
                    body,
                },
                node.start_byte(),
                node.end_byte(),
            ))
        }
        "while_loop" => {
            let condition_node = node.child_by_field_name("condition").unwrap();
            let condition = Box::new(parse_expression(condition_node, source)?);

            let body_node = node.child_by_field_name("body").unwrap();
            let body = Box::new(parse_expression(body_node, source)?);
            Ok(ast::Spanned::new(
                ast::Expression::While { condition, body },
                node.start_byte(),
                node.end_byte(),
            ))
        }
        "break_expression" => {
            let break_value = match node.child_by_field_name("value") {
                Some(child) => Some(Box::new(parse_expression(child, source)?)),
                None => None,
            };
            Ok(ast::Spanned::new(
                ast::Expression::Break(break_value),
                node.start_byte(),
                node.end_byte(),
            ))
        }
        "continue_expression" => Ok(ast::Spanned::new(
            ast::Expression::Continue,
            node.start_byte(),
            node.end_byte(),
        )),
        "return_expression" => {
            let return_value = match node.child_by_field_name("value") {
                Some(child) => Some(Box::new(parse_expression(child, source)?)),
                None => None,
            };
            Ok(ast::Spanned::new(
                ast::Expression::Break(return_value),
                node.start_byte(),
                node.end_byte(),
            ))
        }
        "anonymous_function" => {
            let body_node = node.child_by_field_name("body").unwrap();
            let body = Box::new(parse_expression(body_node, source)?);

            let mut cursor = node.walk();
            let parameters = node
                .children_by_field_name("parameters", &mut cursor)
                .map(|child| parse_typed_identifier(&child, source))
                .collect::<Result<Vec<_>, _>>()?;

            Ok(ast::Spanned::new(
                ast::Expression::AnonymousFunction { parameters, body },
                node.start_byte(),
                node.end_byte(),
            ))
        }
        "call_expression" => {
            let callable_node = node.child_by_field_name("callable").unwrap();
            let callable = Box::new(parse_expression(callable_node, source)?);

            let mut cursor = node.walk();
            let arguments = node
                .children_by_field_name("arguments", &mut cursor)
                .map(|child| parse_expression(child, source))
                .collect::<Result<Vec<_>, _>>()?;

            Ok(ast::Spanned::new(
                ast::Expression::Call {
                    callable,
                    arguments,
                },
                node.start_byte(),
                node.end_byte(),
            ))
        }

        // ==========================================
        // Errors
        // ==========================================

        // these nodes can't resolve to an expression
        "match_case" | "identifier_type_annotation" | "function_definition" => Err(ParseError {
            message: format!("'{}' cannot be used as an expression", node.kind()),
            label: "This doesn't produce a value".to_string(),
            err_span: (node.start_byte()..node.end_byte()).into(),
        }),
        unknown => Err(ParseError {
            message: format!("The parser doesn't know how to handle '{}'", unknown),
            label: "Unknown syntax node".to_string(),
            err_span: (node.start_byte()..node.end_byte()).into(),
        }),
    }
}

fn parse_type(
    node: &tree_sitter::Node,
    source: &[u8],
) -> Result<ast::Spanned<ast::Type>, ParseError> {
    let ty = match node.kind() {
        "int" => ast::Type::Int,
        "u8" => ast::Type::U8,
        "i64" => ast::Type::I64,
        "f64" => ast::Type::F64,
        "bool" => ast::Type::Bool,
        "char" => ast::Type::Char,
        "str" => ast::Type::Str,
        "identifier" => {
            let name = node.utf8_text(source).unwrap().to_string();
            ast::Type::Custom(name)
        }
        "array_type" => {
            let array_type = parse_type(&node.child_by_field_name("type").unwrap(), source)?;
            let length = node.child_by_field_name("length").map(|length_node| {
                length_node
                    .utf8_text(source)
                    .unwrap()
                    .parse::<usize>()
                    .unwrap()
            });

            ast::Type::Array {
                item: Box::new(array_type),
                length,
            }
        }
        "tuple_type" => {
            let mut cursor = node.walk();
            let types = node
                .children_by_field_name("types", &mut cursor)
                .map(|child| parse_type(&child, source))
                .collect::<Result<Vec<_>, _>>()?;
            ast::Type::Tuple(types)
        }
        "reference_type" => {
            let is_mut = node.child_by_field_name("is_mut").is_some();
            let is_shared = node.child_by_field_name("is_shared").is_some();
            let inner = Box::new(parse_type(
                &node.child_by_field_name("type").unwrap(),
                source,
            )?);

            ast::Type::Reference {
                is_mut,
                is_shared,
                inner,
            }
        }
        "function_type" => {
            let tuple_node = node.child_by_field_name("argument_types").unwrap();
            let mut cursor = tuple_node.walk();
            let arguments = node
                .children_by_field_name("types", &mut cursor)
                .map(|child| parse_type(&child, source))
                .collect::<Result<Vec<_>, _>>()?;

            let return_type = Box::new(parse_type(
                &node.child_by_field_name("return_type").unwrap(),
                source,
            )?);
            ast::Type::Function {
                arguments,
                return_type,
            }
        }

        unknown => {
            return Err(ParseError {
                message: format!("The parser doesn't know how to handle '{}'", unknown),
                label: "Unknown syntax node".to_string(),
                err_span: (node.start_byte()..node.end_byte()).into(),
            });
        }
    };

    Ok(ast::Spanned::new(ty, node.start_byte(), node.end_byte()))
}

/// Determines if a tree-sitter node should be skipped. (Comments and puncuation)
fn is_skippable(node: &tree_sitter::Node) -> bool {
    !node.is_named() || node.kind() == "comment"
}

fn parse_match_case(node: &tree_sitter::Node, source: &[u8]) -> Result<ast::MatchCase, ParseError> {
    if node.kind() != "match_case" {
        let span = (node.start_byte()..node.end_byte()).into();
        return Err(ParseError {
            message: format!("Expected a match case, but found '{}'", node.kind()),
            label: "Expected a pattern and a body".to_string(),
            err_span: span,
        });
    }
    let pattern_node = node.child_by_field_name("pattern").unwrap();
    let pattern = parse_expression(pattern_node, source)?;

    let body_node = node.child_by_field_name("body").unwrap();
    let body = parse_expression(body_node, source)?;

    let guard_node = node.child_by_field_name("guard");
    let guard = match guard_node {
        Some(expr_node) => Some(parse_expression(expr_node, source)?),
        None => None,
    };

    Ok(ast::MatchCase {
        pattern,
        guard,
        body,
    })
}

fn parse_typed_identifier(
    node: &tree_sitter::Node,
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

    let name_node = node.child_by_field_name("name").unwrap();
    let name = name_node.utf8_text(source).unwrap().to_string();

    let type_node = node.child_by_field_name("type").unwrap();
    let var_type = parse_type(&type_node, source)?;

    Ok(ast::TypedIdentifier {
        is_mut,
        name,
        var_type,
    })
}

fn parse_choice_variant(
    node: &tree_sitter::Node,
    source: &[u8],
) -> Result<ast::ChoiceVariant, ParseError> {
    let name_node = node.child_by_field_name("name").unwrap();
    let name = name_node.utf8_text(source).unwrap().to_string();

    let mut cursor = node.walk();
    let types = node
        .children_by_field_name("types", &mut cursor)
        .map(|child| parse_type(&child, source))
        .collect::<Result<Vec<_>, ParseError>>()?;

    Ok(ast::ChoiceVariant { name, types })
}

fn parse_assign_target(
    node: tree_sitter::Node,
    source: &[u8],
) -> Result<ast::Spanned<ast::AssignTarget>, ParseError> {
    let target = match node.kind() {
        "identifier" => ast::AssignTarget::Variable(node.utf8_text(source).unwrap().to_string()),

        "array_indexing" => {
            let array_node = node.child_by_field_name("array").unwrap();
            let target = Box::new(parse_assign_target(array_node, source)?);

            let index_node = node.child_by_field_name("index").unwrap();
            let index = Box::new(parse_expression(index_node, source)?);

            ast::AssignTarget::Index { target, index }
        }

        "field_access" => {
            let object_node = node.child_by_field_name("object").unwrap();
            let target = Box::new(parse_assign_target(object_node, source)?);

            let field_node = node.child_by_field_name("field").unwrap();

            // struct vs tuple assignment
            if field_node.kind() == "identifier" {
                let field_name = field_node.utf8_text(source).unwrap().to_string();
                ast::AssignTarget::StructField { target, field_name }
            } else if field_node.kind() == "int_literal" {
                let index_text = field_node.utf8_text(source).unwrap();
                let index = index_text.parse::<usize>().unwrap();
                ast::AssignTarget::TupleField { target, index }
            } else {
                return Err(ParseError {
                    message: "Invalid field access target".to_string(),
                    label: "Expected an identifier or an integer index".to_string(),
                    err_span: (field_node.start_byte()..field_node.end_byte()).into(),
                });
            }
        }

        // You can't assign expressions to things
        _ => {
            return Err(ParseError {
                message: "Invalid left-hand side of assignment".to_string(),
                label: "You can only assign to variables, arrays, or fields".to_string(),
                err_span: (node.start_byte()..node.end_byte()).into(),
            });
        }
    };

    Ok(ast::Spanned::new(
        target,
        node.start_byte(),
        node.end_byte(),
    ))
}

fn parse_machine_variant(
    node: &tree_sitter::Node,
    source: &[u8],
) -> Result<ast::MachineVariant, ParseError> {
    if node.kind() != "event_declaration" && node.kind() != "state_declaration" {
        let span = (node.start_byte()..node.end_byte()).into();
        return Err(ParseError {
            message: format!(
                "Expected event or state declaration, but found '{}'",
                node.kind()
            ),
            label: "Expected 'event' or 'state' here".to_string(),
            err_span: span,
        });
    }

    let name_node = node.child_by_field_name("name").unwrap();
    let name = name_node.utf8_text(source).unwrap().to_string();

    let mut cursor = node.walk();
    let parameters = node
        .children_by_field_name("parameters", &mut cursor)
        .map(|child| parse_typed_identifier(&child, source))
        .collect::<Result<Vec<_>, ParseError>>()?;

    Ok(ast::MachineVariant { name, parameters })
}

fn parse_machine_initializer(
    node: &tree_sitter::Node,
    source: &[u8],
) -> Result<ast::MachineInit, ParseError> {
    if node.kind() != "machine_init" {
        let span = (node.start_byte()..node.end_byte()).into();
        return Err(ParseError {
            message: format!("Expected machine initializer, but found '{}'", node.kind()),
            label: "Expected an 'init' block here".to_string(),
            err_span: span,
        });
    }
    let mut cursor = node.walk();
    let parameters = node
        .children_by_field_name("parameters", &mut cursor)
        .map(|child| parse_typed_identifier(&child, source))
        .collect::<Result<Vec<_>, _>>()?;

    let return_type_node = node.child_by_field_name("return_type").unwrap();
    let return_type = parse_type(&return_type_node, source)?;

    let body_node = node.child_by_field_name("body").unwrap();
    let body = Box::new(parse_expression(body_node, source)?);

    Ok(ast::MachineInit {
        parameters,
        return_type,
        body,
    })
}

fn parse_machine_state_handler(
    node: &tree_sitter::Node,
    source: &[u8],
) -> Result<ast::MachineStateHandler, ParseError> {
    if node.kind() != "machine_state_behavior" {
        let span = (node.start_byte()..node.end_byte()).into();
        return Err(ParseError {
            message: format!("Expected state behavior, but found '{}'", node.kind()),
            label: "Expected an 'in' block here".to_string(),
            err_span: span,
        });
    }
    let state_node = node.child_by_field_name("state").unwrap();
    let state = state_node.utf8_text(source).unwrap().to_string();

    let mut cursor = node.walk();
    let event_handlers = node
        .children_by_field_name("event_handlers", &mut cursor)
        .map(|child| parse_machine_event_handler(&child, source))
        .collect::<Result<Vec<_>, _>>()?;

    // What to do about bindings?
    cursor.reset(*node);
    let bindings = node
        .children_by_field_name("parameters", &mut cursor)
        .map(|child| parse_typed_identifier(&child, source))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ast::MachineStateHandler {
        state,
        bindings,
        event_handlers,
    })
}

fn parse_machine_event_handler(
    node: &tree_sitter::Node,
    source: &[u8],
) -> Result<ast::MachineEventHandler, ParseError> {
    if node.kind() != "machine_event_handler" {
        let span = (node.start_byte()..node.end_byte()).into();
        return Err(ParseError {
            message: format!("Expected event handler, but found '{}'", node.kind()),
            label: "Expected an 'on' block here".to_string(),
            err_span: span,
        });
    }
    let event_node = node.child_by_field_name("event").unwrap();
    let event = parse_expression(event_node, source)?;

    let bindings = match node.child_by_field_name("bindings") {
        Some(bindings_node) => {
            let mut cursor = node.walk();
            bindings_node
                .children_by_field_name("parameters", &mut cursor)
                .map(|child| parse_typed_identifier(&child, source))
                .collect::<Result<Vec<_>, _>>()?
        }
        None => vec![],
    };

    let body_node = node.child_by_field_name("body").unwrap();
    let body = Box::new(parse_expression(body_node, source)?);

    // What to do about this return type? this isn't needed if its an anonymous function.
    let return_type_node = node.child_by_field_name("return_type").unwrap();
    let return_type = parse_type(&return_type_node, source)?;

    Ok(ast::MachineEventHandler {
        event,
        bindings,
        return_type,
        body,
    })
}

// things to fix:
// 1. whats going on with miette spans and ast spans, its confusing
//      - also can just extract the spans from all the parse_functions into a parent function and let expression just focus on expressions
// 2. Unwraps everywhere could be something like this instead:  `.ok_or_else(|| ParseError { err_span: (node.start_byte()..node.end_byte()).into() })?;`
// 3. pass nodes around as references if thats better. like parse_expresion and all of them.
// 4. add tests :)
