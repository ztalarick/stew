use tree_sitter;

use stew_core::ast;

use crate::{ParseError, common, common::TSNodeExt, statement};

pub(crate) fn parse_expression(
    node: tree_sitter::Node,
    source: &[u8],
) -> Result<ast::Spanned<ast::Expression>, ParseError> {
    let expression = match node.kind() {
        // ==========================================
        // LITERALS
        // ==========================================
        "int_literal" => {
            let val: i64 = node.parse_text(source)?;
            ast::Expression::Int(val)
        }
        "float_literal" => {
            let val: f64 = node.parse_text(source)?;

            ast::Expression::Float(val)
        }
        "bool_literal" => {
            let val: bool = node.parse_text(source)?;
            ast::Expression::Bool(val)
        }
        "string_literal" => {
            let text = node.get_text(source)?;
            ast::Expression::Str(text.to_string())
        }
        "array_literal" => {
            let mut cursor = node.walk();
            let array = node
                .children_by_field_name("elements", &mut cursor)
                .map(|child| parse_expression(child, source))
                .collect::<Result<Vec<_>, _>>()?;

            ast::Expression::Array(array)
        }
        "tuple_literal" => {
            let mut cursor = node.walk();
            let tuple = node
                .children_by_field_name("elements", &mut cursor)
                .map(|child| parse_expression(child, source))
                .collect::<Result<Vec<_>, _>>()?;

            ast::Expression::Tuple(tuple)
        }
        "struct_literal" => {
            let mut fields = vec![];
            let mut cursor = node.walk();
            let children = node.children_by_field_name("fields", &mut cursor);
            for child in children {
                if child.kind() == "struct_field_initializer" {
                    let field_node = child.get_child_field("field")?;
                    let value_node = child.get_child_field("value")?;

                    let field_name = field_node.get_text(source)?.to_string();
                    let value_expr = parse_expression(value_node, source)?;
                    fields.push((field_name, value_expr))
                }
            }

            ast::Expression::Struct { fields }
        }

        // ==========================================
        // COMPLEX EXPRESSIONS
        // ==========================================
        "unary_expression" => {
            let operator_node = node.get_child_field("operator")?;
            let operand_node = node.get_child_field("argument")?;

            let op = match operator_node.get_text(source)?.as_str() {
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

            ast::Expression::Unary {
                op,
                operand: Box::new(operand),
            }
        }
        "binary_expression" => {
            let left_node = node.get_child_field("left")?;
            let operator_node = node.get_child_field("operator")?;
            let right_node = node.get_child_field("left")?;

            let op = match operator_node.get_text(source)?.as_str() {
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
            ast::Expression::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            }
        }
        "array_indexing" => {
            let array_node = node.get_child_field("array")?;
            let index_node = node.get_child_field("index")?;

            let array = parse_expression(array_node, source)?;
            let index = parse_expression(index_node, source)?;

            ast::Expression::ArrayIndex {
                target: Box::new(array),
                index: Box::new(index),
            }
        }
        "field_access" => {
            let object_node = node.get_child_field("object")?;
            let field_node = node.get_child_field("field")?;

            let target = parse_expression(object_node, source)?;

            let field = match field_node.kind() {
                "identifier" => ast::FieldName::Named(field_node.get_text(source)?),
                "int_literal" => ast::FieldName::Index(field_node.parse_text::<usize>(source)?),

                _ => {
                    let span = (field_node.start_byte()..field_node.end_byte()).into();
                    return Err(ParseError {
                        message: "Invalid field access".to_string(),
                        label: "Expected an identifier or an integer index".to_string(),
                        err_span: span,
                    });
                }
            };

            ast::Expression::Field {
                target: Box::new(target),
                field,
            }
        }
        "address_of" => {
            let is_mut = node.child_by_field_name("mutable").is_some();

            let value_node = node.get_child_field("value")?;
            let value = parse_expression(value_node, source)?;

            ast::Expression::AddressOf {
                is_mut,
                target: Box::new(value),
            }
        }
        "wildcard_expression" => ast::Expression::Wildcard,

        // ==========================================
        // CONTROL FLOW & FUNCTIONS
        // ==========================================
        "identifier" => {
            let identifier = node.get_text(source)?;

            ast::Expression::Identifier(identifier)
        }
        "block" => {
            let mut cursor = node.walk();
            let statements = node
                .children_by_field_name("statements_nodes", &mut cursor)
                .filter(|child| !common::is_skippable(*child))
                .map(|child| statement::parse_statement(child, source))
                .collect::<Result<Vec<_>, _>>()?;

            let final_expression = match node.child_by_field_name("final_expression") {
                Some(expr_node) => Some(Box::new(parse_expression(expr_node, source)?)),
                None => None,
            };

            ast::Expression::Block {
                statements,
                final_expression,
            }
        }
        "if_else" => {
            let condition_node = node.get_child_field("condition")?;
            let consequence_node = node.get_child_field("consequence")?;

            let condition = Box::new(parse_expression(condition_node, source)?);
            let consequence = Box::new(parse_expression(consequence_node, source)?);

            let alternative = node
                .child_by_field_name("alternative")
                .map(|alternative_node| parse_expression(alternative_node, source).map(Box::new))
                .transpose()?;

            ast::Expression::If {
                condition,
                consequence,
                alternative,
            }
        }
        "match_expression" => {
            let match_value_node = node.get_child_field("value")?;
            let match_value = Box::new(parse_expression(match_value_node, source)?);

            let mut cursor = node.walk();
            let cases = node
                .children_by_field_name("cases", &mut cursor)
                .map(|child| parse_match_case(child, source))
                .collect::<Result<Vec<_>, _>>()?;

            ast::Expression::Match {
                target: match_value,
                options: cases,
            }
        }
        "for_loop" => {
            let mut cursor = node.walk();
            let bindings = node
                .children_by_field_name("bindings", &mut cursor)
                .map(|child| common::parse_typed_identifier(child, source))
                .collect::<Result<Vec<_>, _>>()?;

            let iterator_node = node.get_child_field("iterator")?;
            let iterator = Box::new(parse_expression(iterator_node, source)?);

            let body_node = node.get_child_field("body")?;
            let body = Box::new(parse_expression(body_node, source)?);

            ast::Expression::For {
                bindings,
                iterator,
                body,
            }
        }
        "while_loop" => {
            let condition_node = node.get_child_field("condition")?;
            let condition = Box::new(parse_expression(condition_node, source)?);

            let body_node = node.get_child_field("body")?;
            let body = Box::new(parse_expression(body_node, source)?);

            ast::Expression::While { condition, body }
        }
        "break_expression" => {
            let break_value = match node.child_by_field_name("value") {
                Some(child) => Some(Box::new(parse_expression(child, source)?)),
                None => None,
            };

            ast::Expression::Break(break_value)
        }
        "continue_expression" => ast::Expression::Continue,
        "return_expression" => {
            let return_value = match node.child_by_field_name("value") {
                Some(child) => Some(Box::new(parse_expression(child, source)?)),
                None => None,
            };

            ast::Expression::Break(return_value)
        }
        "anonymous_function" => {
            let body_node = node.get_child_field("body")?;
            let body = Box::new(parse_expression(body_node, source)?);

            let mut cursor = node.walk();
            let parameters = node
                .children_by_field_name("parameters", &mut cursor)
                .map(|child| common::parse_typed_identifier(child, source))
                .collect::<Result<Vec<_>, _>>()?;

            ast::Expression::AnonymousFunction { parameters, body }
        }
        "call_expression" => {
            let callable_node = node.get_child_field("callable")?;
            let callable = Box::new(parse_expression(callable_node, source)?);

            let mut cursor = node.walk();
            let arguments = node
                .children_by_field_name("arguments", &mut cursor)
                .map(|child| parse_expression(child, source))
                .collect::<Result<Vec<_>, _>>()?;

            ast::Expression::Call {
                callable,
                arguments,
            }
        }

        // ==========================================
        // Errors
        // ==========================================

        // these nodes can't resolve to an expression
        "match_case" | "identifier_type_annotation" | "function_definition" => {
            return Err(ParseError {
                message: format!("'{}' cannot be used as an expression", node.kind()),
                label: "This doesn't produce a value".to_string(),
                err_span: (node.start_byte()..node.end_byte()).into(),
            });
        }
        unknown => {
            return Err(ParseError {
                message: format!("The parser doesn't know how to handle '{}'", unknown),
                label: "Unknown syntax node".to_string(),
                err_span: (node.start_byte()..node.end_byte()).into(),
            });
        }
    };
    return Ok(ast::Spanned::new(
        expression,
        node.start_byte(),
        node.end_byte(),
    ));
}

fn parse_match_case(node: tree_sitter::Node, source: &[u8]) -> Result<ast::MatchCase, ParseError> {
    if node.kind() != "match_case" {
        let span = (node.start_byte()..node.end_byte()).into();
        return Err(ParseError {
            message: format!("Expected a match case, but found '{}'", node.kind()),
            label: "Expected a pattern and a body".to_string(),
            err_span: span,
        });
    }
    let pattern_node = node.get_child_field("pattern")?;
    let pattern = parse_expression(pattern_node, source)?;

    let body_node = node.get_child_field("body")?;
    let body = parse_expression(body_node, source)?;

    let guard = node
        .child_by_field_name("guard")
        .map(|guard_node| parse_expression(guard_node, source))
        .transpose()?;

    Ok(ast::MatchCase {
        pattern,
        guard,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_cst;
    use insta::assert_debug_snapshot;

    fn parse_expr(source: &str) -> ast::Spanned<ast::Expression> {
        let formatted = format!("let _dummy: int = {};", source);
        let tree = parse_cst(&formatted).unwrap();
        let root = tree.root_node();
        let let_node = root.child(0).unwrap();
        let expr_node = let_node.child_by_field_name("value").unwrap();
        parse_expression(expr_node, formatted.as_bytes()).unwrap()
    }

    #[test]
    fn test_literals() {
        assert_debug_snapshot!(parse_expr("42"));
        assert_debug_snapshot!(parse_expr("3.14"));
        assert_debug_snapshot!(parse_expr("true"));
        assert_debug_snapshot!(parse_expr(r#""hello""#));
        assert_debug_snapshot!(parse_expr("[1, 2, 3]"));
        assert_debug_snapshot!(parse_expr("{ id: 1, name: \"test\" }"));
    }

    #[test]
    fn test_complex_expressions() {
        assert_debug_snapshot!(parse_expr("1 + 2 * 3"));
        assert_debug_snapshot!(parse_expr("a == b and c != d"));
        assert_debug_snapshot!(parse_expr("-x"));
        assert_debug_snapshot!(parse_expr("not true"));
    }

    #[test]
    fn test_control_flow() {
        assert_debug_snapshot!(parse_expr("if x > 5 { 1 } else { 0 }"));
        assert_debug_snapshot!(parse_expr(
            "match x { | Option.Some(v) -> v | Option.None -> 0 }"
        ));
        assert_debug_snapshot!(parse_expr("for x: int in arr { print(x); }"));
        assert_debug_snapshot!(parse_expr("while true { break; }"));
    }

    #[test]
    fn test_calls_and_access() {
        assert_debug_snapshot!(parse_expr("print(\"hello\", 42)"));
        assert_debug_snapshot!(parse_expr("user.name"));
        assert_debug_snapshot!(parse_expr("arr[0]"));
        assert_debug_snapshot!(parse_expr("tuple.1"));
    }

    #[test]
    fn test_edge_cases() {
        assert_debug_snapshot!(parse_expr("&x"));
        assert_debug_snapshot!(parse_expr("&mut x"));
        assert_debug_snapshot!(parse_expr("(x: int) -> x + 1"));
        assert_debug_snapshot!(parse_expr("return 5"));
        assert_debug_snapshot!(parse_expr("break 10"));
        assert_debug_snapshot!(parse_expr("match x { | Option.Some(v) if v > 5 -> v }"));
        assert_debug_snapshot!(parse_expr("(1, \"hello\")"));
    }
}
