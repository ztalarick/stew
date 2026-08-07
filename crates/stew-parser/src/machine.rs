use tree_sitter;

use stew_core::ast;

use crate::expression::parse_expression;
use crate::{ParseError, common, common::TSNodeExt, types};

pub(crate) fn parse_machine(
    node: tree_sitter::Node,
    source: &[u8],
) -> Result<ast::Statement, ParseError> {
    let name = node.get_child_field_text("name", source)?;

    let machine_body_node = node.get_child_field("body")?;

    let mut cursor = node.walk();
    let events = machine_body_node
        .children_by_field_name("events", &mut cursor)
        .map(|child| parse_machine_variant(child, source))
        .collect::<Result<Vec<_>, _>>()?;

    cursor.reset(machine_body_node);
    let states = machine_body_node
        .children_by_field_name("states", &mut cursor)
        .map(|child| parse_machine_variant(child, source))
        .collect::<Result<Vec<_>, _>>()?;

    // how can I error if more than one initilzier is specified?
    cursor.reset(machine_body_node);

    let init = {
        let mut init_iterator =
            machine_body_node.children_by_field_name("initializers", &mut cursor);
        let init_node = match init_iterator.next() {
            Some(child) => parse_machine_initializer(child, source)?,
            None => {
                let span = (machine_body_node.start_byte()..machine_body_node.end_byte()).into();
                return Err(ParseError {
                    message: "A state machine must have an initialization block".to_string(),
                    label: "Add an `init` block here".to_string(),
                    err_span: span,
                });
            }
        };
        if let Some(second_child) = init_iterator.next() {
            let span = (second_child.start_byte()..second_child.end_byte()).into();
            return Err(ParseError {
                message: "A state machine can only have one initialization block".to_string(),
                label: "Remove this duplicate init block".to_string(),
                err_span: span,
            });
        }
        init_node
    };

    cursor.reset(machine_body_node);
    let state_handlers = machine_body_node
        .children_by_field_name("state_behaviors", &mut cursor)
        .map(|child| parse_machine_state_handler(child, source))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ast::Statement::Machine {
        name,
        init,
        events,
        states,
        state_handlers,
    })
}

fn parse_machine_variant(
    node: tree_sitter::Node,
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

    let name = node.get_child_field_text("name", source)?;

    let mut cursor = node.walk();
    let parameters = node
        .children_by_field_name("parameters", &mut cursor)
        .map(|child| common::parse_typed_identifier(child, source))
        .collect::<Result<Vec<_>, ParseError>>()?;

    Ok(ast::MachineVariant { name, parameters })
}

fn parse_machine_initializer(
    node: tree_sitter::Node,
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
        .map(|child| common::parse_typed_identifier(child, source))
        .collect::<Result<Vec<_>, _>>()?;

    let return_type_node = node.get_child_field("return_type")?;
    let return_type = types::parse_type(return_type_node, source)?;

    let body_node = node.get_child_field("body")?;
    let body = Box::new(parse_expression(body_node, source)?);

    Ok(ast::MachineInit {
        parameters,
        return_type,
        body,
    })
}

fn parse_machine_state_handler(
    node: tree_sitter::Node,
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
    let state = node.get_child_field_text("state", source)?;

    let mut cursor = node.walk();
    let event_handlers = node
        .children_by_field_name("event_handlers", &mut cursor)
        .map(|child| parse_machine_event_handler(child, source))
        .collect::<Result<Vec<_>, _>>()?;

    // What to do about bindings?
    cursor.reset(node);
    let bindings = node
        .children_by_field_name("parameters", &mut cursor)
        .map(|child| common::parse_typed_identifier(child, source))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ast::MachineStateHandler {
        state,
        bindings,
        event_handlers,
    })
}

fn parse_machine_event_handler(
    node: tree_sitter::Node,
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
    let event_node = node.get_child_field("event")?;
    let event = parse_expression(event_node, source)?;

    let bindings = node
        .child_by_field_name("bindings")
        .map(|bindings_node| {
            let mut cursor = bindings_node.walk();
            bindings_node
                .children_by_field_name("parameters", &mut cursor)
                .map(|child| common::parse_typed_identifier(child, source))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();

    let body_node = node.get_child_field("body")?;
    let body = Box::new(parse_expression(body_node, source)?);

    // What to do about this return type? this isn't needed if its an anonymous function.
    let return_type_node = node.get_child_field("return_type")?;
    let return_type = types::parse_type(return_type_node, source)?;

    Ok(ast::MachineEventHandler {
        event,
        bindings,
        return_type,
        body,
    })
}
