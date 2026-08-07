use stew_core::ast;

use crate::{ParseError, common, common::TSNodeExt, expression, machine, types};

pub(crate) fn parse_statement(
    node: tree_sitter::Node,
    source: &[u8],
) -> Result<ast::Spanned<ast::Statement>, ParseError> {
    let statement = match node.kind() {
        "expression_statement" => {
            let child_node = node.get_child_field("expression")?;
            let expr = expression::parse_expression(child_node, source)?;
            ast::Statement::ExprStatement(Box::new(expr))
        }

        // control flow
        "function_definition" => {
            let name = node.get_child_field_text("name", source)?;

            let return_type_node = node.get_child_field("return_type")?;
            let return_type = types::parse_type(return_type_node, source)?;

            let body_node = node.get_child_field("body")?;
            let body = Box::new(expression::parse_expression(body_node, source)?);

            let mut cursor = node.walk();
            let parameters = node
                .children_by_field_name("parameters", &mut cursor)
                .map(|child| common::parse_typed_identifier(child, source))
                .collect::<Result<Vec<_>, _>>()?;

            ast::Statement::Function {
                name,
                parameters,
                return_type,
                body,
            }
        }
        "if_else" | "block" | "for_loop" | "while_loop" | "match_expression" => {
            let expr = expression::parse_expression(node, source)?;
            ast::Statement::ExprStatement(Box::new(expr))
        }

        // variable
        "let_declaration" => {
            let identifier_type_node = node.get_child_field("identifier_type")?;
            let declaration = common::parse_typed_identifier(identifier_type_node, source)?;

            let value_node = node.get_child_field("value")?;
            let value = Box::new(expression::parse_expression(value_node, source)?);

            ast::Statement::LetDeclaration { declaration, value }
        }
        "assignment_statement" => {
            let value_node = node.get_child_field("right")?;
            let value = Box::new(expression::parse_expression(value_node, source)?);

            let assign_target_node = node.get_child_field("left")?;
            let target = parse_assign_target(assign_target_node, source)?;

            ast::Statement::Assignment { target, value }
        }

        //advanced definitions
        "struct" => {
            let name = node.get_child_field_text("name", source)?;

            let mut cursor = node.walk();
            let field_types = node
                .children_by_field_name("fields", &mut cursor)
                .map(|child| common::parse_typed_identifier(child, source))
                .collect::<Result<Vec<_>, _>>()?;

            ast::Statement::StructDefinition { name, field_types }
        }
        "choice_definition" => {
            let name = node.get_child_field_text("name", source)?;

            let mut cursor = node.walk();
            let choices = node
                .children_by_field_name("choice_variants", &mut cursor)
                .map(|child| parse_choice_variant(child, source))
                .collect::<Result<Vec<_>, _>>()?;

            ast::Statement::ChoiceDefinition { name, choices }
        }
        "machine_definition" => machine::parse_machine(node, source)?,

        // ==========================================
        // FALLBACK
        // ==========================================
        unknown => {
            return Err(ParseError {
                message: format!("The parser doesn't know how to handle '{}'", unknown),
                label: "Unknown syntax node".to_string(),
                err_span: (node.start_byte()..node.end_byte()).into(),
            });
        }
    };
    Ok(ast::Spanned::new(
        statement,
        node.start_byte(),
        node.end_byte(),
    ))
}

fn parse_assign_target(
    node: tree_sitter::Node,
    source: &[u8],
) -> Result<ast::Spanned<ast::AssignTarget>, ParseError> {
    let target = match node.kind() {
        "identifier" => ast::AssignTarget::Variable(node.get_child_field_text("name", source)?),

        "array_indexing" => {
            let array_node = node.get_child_field("array")?;
            let target = Box::new(parse_assign_target(array_node, source)?);

            let index_node = node.get_child_field("index")?;
            let index = Box::new(expression::parse_expression(index_node, source)?);

            ast::AssignTarget::Index { target, index }
        }

        "field_access" => {
            let object_node = node.get_child_field("object")?;
            let target = Box::new(parse_assign_target(object_node, source)?);

            let field_node = node.get_child_field("field")?;

            match field_node.kind() {
                "identifier" => {
                    let field_name = field_node.get_text(source)?;
                    ast::AssignTarget::StructField { target, field_name }
                }
                "int_literal" => {
                    let index = field_node.parse_text::<usize>(source)?;
                    ast::AssignTarget::TupleField { target, index }
                }
                _ => {
                    return Err(ParseError {
                        message: "Invalid field access target".to_string(),
                        label: "Expected an identifier or an integer index".to_string(),
                        err_span: (field_node.start_byte()..field_node.end_byte()).into(),
                    });
                }
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

fn parse_choice_variant(
    node: tree_sitter::Node,
    source: &[u8],
) -> Result<ast::ChoiceVariant, ParseError> {
    if node.kind() != "choice_variant" {
        let span = (node.start_byte()..node.end_byte()).into();
        return Err(ParseError {
            message: format!("Expected a choice variant, but found '{}'", node.kind()),
            label: "Expected a choice variant".to_string(),
            err_span: span,
        });
    }
    let name = node.get_child_field_text("name", source)?;

    let mut cursor = node.walk();
    let types = node
        .children_by_field_name("types", &mut cursor)
        .map(|child| types::parse_type(child, source))
        .collect::<Result<Vec<_>, ParseError>>()?;

    Ok(ast::ChoiceVariant { name, types })
}
