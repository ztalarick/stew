use std::fs;
use stew_parser::parse;

#[test]
fn test_parse_shopping_cart() {
    let source = fs::read_to_string("../../tree-sitter-stew/examples/shopping_cart.stew").unwrap();
    let program = parse(source).expect("Failed to parse shopping_cart.stew");
    insta::assert_debug_snapshot!(program);
}

#[test]
fn test_parse_hello_world() {
    let source = fs::read_to_string("../../tree-sitter-stew/examples/hello_world.stew").unwrap();
    let program = parse(source).expect("Failed to parse hello_world.stew");
    insta::assert_debug_snapshot!(program);
}
