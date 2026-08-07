use tree_sitter;

use stew_core::ast;

use crate::{ParseError, common::TSNodeExt};

pub(crate) fn parse_type(
    node: tree_sitter::Node,
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
            let name = node.get_text(source)?;
            ast::Type::Custom(name)
        }
        "array_type" => {
            let array_type = parse_type(node.get_child_field("type")?, source)?;
            let length = node
                .child_by_field_name("length")
                .map(|length_node| length_node.parse_text::<usize>(source))
                .transpose()?;

            ast::Type::Array {
                item: Box::new(array_type),
                length,
            }
        }
        "tuple_type" => {
            let mut cursor = node.walk();
            let types = node
                .children_by_field_name("types", &mut cursor)
                .map(|child| parse_type(child, source))
                .collect::<Result<Vec<_>, _>>()?;
            ast::Type::Tuple(types)
        }
        "reference_type" => {
            let is_mut = node.child_by_field_name("is_mut").is_some();
            let is_shared = node.child_by_field_name("is_shared").is_some();
            let inner = Box::new(parse_type(node.get_child_field("type")?, source)?);

            ast::Type::Reference {
                is_mut,
                is_shared,
                inner,
            }
        }
        "function_type" => {
            let tuple_node = node.get_child_field("argument_types")?;
            let mut cursor = tuple_node.walk();
            let arguments = node
                .children_by_field_name("types", &mut cursor)
                .map(|child| parse_type(child, source))
                .collect::<Result<Vec<_>, _>>()?;

            let return_type = Box::new(parse_type(node.get_child_field("return_type")?, source)?);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_cst;
    use insta::assert_debug_snapshot;

    fn parse_ty(source: &str) -> ast::Spanned<ast::Type> {
        let formatted = format!("let x: {} = 0;", source);
        let tree = parse_cst(&formatted).unwrap();
        let root = tree.root_node();
        let stmt = root.child(0).unwrap();
        let id_type = stmt.child_by_field_name("identifier_type").unwrap();
        let ty_node = id_type.child_by_field_name("type").unwrap();
        parse_type(ty_node, formatted.as_bytes()).unwrap()
    }

    #[test]
    fn test_types() {
        assert_debug_snapshot!(parse_ty("int"));
        assert_debug_snapshot!(parse_ty("str"));
        assert_debug_snapshot!(parse_ty("&mut User"));
        assert_debug_snapshot!(parse_ty("[int; 4]"));
    }
}
