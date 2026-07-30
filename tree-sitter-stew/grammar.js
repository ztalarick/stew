/**
 * @file Stew grammar for tree-sitter
 * @author Zach Talarick <ztalarick@gmail.com>
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

/**
 * Matches zero or more items separated by commas
 * @param {RuleOrLiteral} rule
 * @returns {Rule}
 */
function commaSep(rule) {
  return optional(commaSep1(rule));
}

/**
 * Matches one or more items separated by commas
 * @param {RuleOrLiteral} rule
 * @returns {Rule}
 */
function commaSep1(rule) {
  return seq(rule, repeat(seq(",", rule)));
}

// Order of operations
const PREC = {
  // Lowest Priority
  OR: 1, // or
  AND: 2, // and
  COMPARE: 3, // ==, !=, <, >, <=, >=
  BITWISE_OR: 4, // |
  BITWISE_XOR: 5, // ^
  BITWISE_AND: 6, // &
  SHIFT: 7, // <<, >>
  ADD: 8, // +, -
  MULTIPLY: 9, // *, /, %
  UNARY: 10, // not, - (negative numbers)
  CALL: 11, // ., [], ()
  // Highest Priority
};

module.exports = grammar({
  name: "stew",

  extras: ($) => [
    /\s/, //standard whitespace
    $.comment,
  ],

  conflicts: ($) => [
    [$.block, $.struct_literal],
    [$._statement, $.expression],
    [$.tuple_literal, $.parameter_list],
    [$.match_body, $.binary_expression],
  ],

  rules: {
    source_file: ($) => repeat($._statement),

    comment: ($) =>
      token(
        choice(
          // Single-line comment: // ...
          /\/\/[^\n]*/,

          // Multi-line comment: /* ... */
          seq("/*", /[^*]*\*+([^/*][^*]*\*+)*/, "/"),
        ),
      ),

    _statement: ($) =>
      choice(
        // expression
        $.expression_statement,

        // control flow
        $.function_definition,
        $.if_else,
        $.block,
        $.for_loop,
        $.while_loop,
        $.match_expression,

        // variable
        $.let_declaration,
        $.assignment_statement,

        //advanced definitions
        $.struct,
        $.choice_definition,
        $.machine_declaration,
      ),

    _expression_statement: ($) => seq($.expression, ";"),

    _expression: ($) =>
      choice(
        // literals
        $.string_literal,
        $.int_literal,
        $.bool_literal,
        $.float_literal,
        $.array_literal,
        $.tuple_literal,
        $.struct_literal,

        // complex expressions
        $.binary_expression,
        $.unary_expression,
        $.array_indexing,
        $.field_access,
        $.address_of,
        $.wildcard_expression,

        // control flow
        $.identifier,
        $.call_expression,
        $.block,
        $.if_else,
        $.anonymous_function,
        $.break_expression,
        $.continue_expression,
        $.return_expression,
        $.match_expression,
      ),

    // OPERATORS AND MATH/BOOLEAN/BIT EXPRESSIONS
    unary_expression: ($) =>
      prec(
        PREC.UNARY,
        seq(
          field("operator", choice("-", "not")),
          field("argument", $.expression),
        ),
      ),

    binary_expression: ($) =>
      choice(
        // Multiplication / Division / Modulo (Highest Math Priority)
        prec.left(
          PREC.MULTIPLY,
          seq(
            field("left", $.expression),
            field("operator", choice("*", "/", "%")),
            field("right", $.expression),
          ),
        ),

        // Boolean or
        prec.left(
          PREC.OR,
          seq(
            field("left", $.expression),
            field("operator", "or"),
            field("right", $.expression),
          ),
        ),

        // Boolean AND
        prec.left(
          PREC.AND,
          seq(
            field("left", $.expression),
            field("operator", "and"),
            field("right", $.expression),
          ),
        ),

        // Comparison
        prec.left(
          PREC.COMPARE,
          seq(
            field("left", $.expression),
            field("operator", choice("==", ">", "<", ">=", "<=", "!=")),
            field("right", $.expression),
          ),
        ),

        // Bitwise Shifts
        prec.left(
          PREC.SHIFT,
          seq(
            field("left", $.expression),
            field("operator", choice("<<", ">>")),
            field("right", $.expression),
          ),
        ),

        // Bitwise AND
        prec.left(
          PREC.BITWISE_AND,
          seq(
            field("left", $.expression),
            field("operator", "&"),
            field("right", $.expression),
          ),
        ),

        // Bitwise OR
        prec.left(
          PREC.BITWISE_OR,
          seq(
            field("left", $.expression),
            field("operator", "|"),
            field("right", $.expression),
          ),
        ),

        // Addition / Subtraction (Lowest priority)
        prec.left(
          PREC.ADD,
          seq(
            field("left", $.expression),
            field("operator", choice("+", "-")),
            field("right", $.expression),
          ),
        ),
      ),

    address_of: ($) =>
      prec(PREC.UNARY, seq("&", optional("mut"), field("value", $.expression))),

    // needs to support expression evaluating to array and index eval to positive int
    array_indexing: ($) =>
      prec.left(
        PREC.CALL,
        seq(
          field("array", $.expression),
          "[",
          field("index", $.expression),
          "]",
        ),
      ),

    // both tuple and struct access
    field_access: ($) =>
      prec.left(
        PREC.CALL,
        seq(
          field("object", $.expression),
          ".",
          field("field", choice($.identifier, $.int_literal)),
        ),
      ),

    // LITERALS
    string_literal: ($) => /"([^"\\]|\\.)*"/,
    int_literal: ($) => /\d+/,
    float_literal: ($) => /\d+\.\d+/,
    bool_literal: ($) => choice("true", "false"),
    array_literal: ($) => seq("[", commaSep($.expression), "]"),
    tuple_literal: ($) => seq("(", commaSep($.expression), ")"),

    struct_field_initializer: ($) =>
      seq(field("field", $.identifier), ":", field("value", $.expression)),
    struct_literal: ($) => seq("{", commaSep($.struct_field_initializer), "}"),

    // VARIABLES AND ASSIGNMENT
    identifier: ($) => /[a-zA-Z_][a-zA-Z0-9_]*/,

    identifier_type_annotation: ($) =>
      seq(
        optional("mut"),
        field("name", $.identifier),
        ":",
        field("type", $.type),
      ),

    let_declaration: ($) =>
      seq(
        "let",
        $.identifier_type_annotation,
        "=",
        field("value", $.expression),
        ";",
      ),

    assignment_statement: ($) =>
      seq(
        field("left", choice($.identifier, $.array_indexing, $.field_access)),
        "=",
        field("right", $.expression),
        ";",
      ),

    // TYPES
    type: ($) =>
      choice(
        "int",
        "u8",
        "i64",
        "f64",
        "bool",
        "char",
        "str",
        $.identifier,
        $.array_type,
        $.tuple_type,
        $.reference_type,
        $.function_type,
      ),

    array_type: ($) => seq("[", $.type, optional(seq(";", $.int_literal)), "]"),
    tuple_type: ($) => seq("(", commaSep($.type), ")"),

    // This allows &int, &mut str, shared &[int; 4], etc.
    reference_type: ($) =>
      seq(optional("shared"), "&", optional("mut"), $.type),

    function_type: ($) => seq($.tuple_type, "->", $.type),

    // CONTROL FLOW
    function_definition: ($) =>
      seq(
        "fun",
        field("name", $.identifier),
        field("parameters", $.parameter_list),
        "->",
        field("return_type", $.type),
        field("body", $.block),
      ),

    parameter_list: ($) =>
      seq("(", commaSep($.identifier_type_annotation), ")"),

    block: ($) => seq("{", repeat($._statement), optional($.expression), "}"),

    // this is for choice literals `Option.Some(5)` and function calls `print("hi")
    call_expression: ($) =>
      prec.left(
        PREC.CALL,
        seq(field("callable", $.expression), "(", commaSep($.expression), ")"),
      ),

    if_else: ($) =>
      seq(
        "if",
        field("condition", $.expression),
        field("consequence", $.block),
        optional(seq("else", field("alternative", choice($.block, $.if_else)))),
      ),

    match_expression: ($) =>
      seq(
        "match",
        field("value", $.expression),
        "{",
        repeat(prec.left(PREC.BITWISE_OR, seq("|", $.match_case))),
        "}",
      ),

    match_case: ($) =>
      prec(
        PREC.BITWISE_OR,
        seq(
          field("pattern", $.expression),
          optional(seq("if", field("guard", $.expression))),
          "->",
          field("body", $.match_body),
        ),
      ),

    match_body: ($) => prec(PREC.BITWISE_OR, $.expression),

    while_loop: ($) =>
      seq("while", field("condition", $.expression), field("body", $.block)),

    for_loop: ($) =>
      seq(
        "for",
        field("bindings", commaSep($.identifier_type_annotation)),
        "in",
        field("iterator", $.expression),
        field("body", $.block),
      ),

    // Break can optionally take an expression to return a value from a loop
    break_expression: ($) =>
      prec.right(seq("break", optional(field("value", $.expression)))),

    continue_expression: ($) => "continue",

    return_expression: ($) =>
      prec.right(seq("return", optional(field("value", $.expression)))),

    wildcard_expression: ($) => "_",

    // ADVANCED DEFINITIONS

    struct: ($) =>
      seq(
        "struct",
        field("name", $.identifier),
        "{",
        repeat(seq($.identifier_type_annotation, optional(","))),
        "}",
      ),
    anonymous_function: ($) =>
      seq(
        field("parameters", $.parameter_list),
        "->",
        field("body", $.expression),
      ),

    choice_variant: ($) =>
      seq(
        "|",
        choice(
          field("name", $.identifier),
          seq(
            field("name", $.identifier),
            "(",
            field("types", commaSep1($.type)),
            ")",
          ),
        ),
      ),
    choice_definition: ($) =>
      seq(
        "choice",
        field("name", $.identifier),
        "{",
        repeat($.choice_variant),
        "}",
      ),

    // MACHINE
    machine_declaration: ($) =>
      seq(
        "machine",
        field("name", $.identifier),
        field("body", $.machine_block),
      ),

    machine_block: ($) =>
      seq(
        "{",
        repeat(
          choice(
            $.event_declaration,
            $.state_declaration,
            $.machine_init,
            $.machine_state_behavior,
          ),
        ),
        "}",
      ),

    // `event AddItem(price: int)` or `event StartCheckout`
    event_declaration: ($) =>
      seq(
        "event",
        field("name", $.identifier),
        optional(field("parameters", $.parameter_list)),
      ),

    // `state Browsing(cart_id: int, item_count: int, subtotal: int)` or `state Terminated`
    state_declaration: ($) =>
      seq(
        "state",
        field("name", $.identifier),
        optional(field("parameters", $.parameter_list)),
      ),

    // `init(id: int) -> ShoppingCart { ... }`
    machine_init: ($) =>
      seq(
        "init",
        field("parameters", $.parameter_list),
        "->",
        field("return_type", $.type),
        field("body", $.block),
      ),

    // `in Browsing(cart_id: string, item_count: int, subtotal: float) { ... }`
    machine_state_behavior: ($) =>
      seq(
        "in",
        field("state", $.identifier),

        optional(field("bindings", $.parameter_list)),
        "{",
        // Inside an `in` block, we only allow `on` handlers
        repeat($.machine_event_handler),
        "}",
      ),

    // `on AddItem(price: int) -> ShoppingCart { ... }`
    // `on _ -> ShoppingCart { ... }`
    machine_event_handler: ($) =>
      seq(
        "on",
        choice(
          field("event", $.identifier),
          "_", // Allows wildcard fallback
        ),
        optional(field("bindings", $.parameter_list)),
        "->",
        field("return_type", $.type),
        field("body", $.block),
      ),
  },
});
