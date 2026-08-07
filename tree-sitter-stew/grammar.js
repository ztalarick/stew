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
    [$._statement, $._expression],
    [$.tuple_literal, $._parameter_list],
    [$._match_body, $.binary_expression],
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
        $._expression_statement,

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
        $.machine_definition,
      ),

    _expression_statement: ($) => seq(field("expression", $._expression), ";"),

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
          field("argument", $._expression),
        ),
      ),

    binary_expression: ($) =>
      choice(
        // Multiplication / Division / Modulo (Highest Math Priority)
        prec.left(
          PREC.MULTIPLY,
          seq(
            field("left", $._expression),
            field("operator", choice("*", "/", "%")),
            field("right", $._expression),
          ),
        ),

        // Boolean or
        prec.left(
          PREC.OR,
          seq(
            field("left", $._expression),
            field("operator", "or"),
            field("right", $._expression),
          ),
        ),

        // Boolean AND
        prec.left(
          PREC.AND,
          seq(
            field("left", $._expression),
            field("operator", "and"),
            field("right", $._expression),
          ),
        ),

        // Comparison
        prec.left(
          PREC.COMPARE,
          seq(
            field("left", $._expression),
            field("operator", choice("==", ">", "<", ">=", "<=", "!=")),
            field("right", $._expression),
          ),
        ),

        // Bitwise Shifts
        prec.left(
          PREC.SHIFT,
          seq(
            field("left", $._expression),
            field("operator", choice("<<", ">>")),
            field("right", $._expression),
          ),
        ),

        // Bitwise AND
        prec.left(
          PREC.BITWISE_AND,
          seq(
            field("left", $._expression),
            field("operator", "&"),
            field("right", $._expression),
          ),
        ),

        // Bitwise OR
        prec.left(
          PREC.BITWISE_OR,
          seq(
            field("left", $._expression),
            field("operator", "|"),
            field("right", $._expression),
          ),
        ),

        // Addition / Subtraction (Lowest priority)
        prec.left(
          PREC.ADD,
          seq(
            field("left", $._expression),
            field("operator", choice("+", "-")),
            field("right", $._expression),
          ),
        ),
      ),

    address_of: ($) =>
      prec(
        PREC.UNARY,
        seq(
          "&",
          optional(field("mutable", "mut")),
          field("value", $._expression),
        ),
      ),

    // needs to support expression evaluating to array and index eval to positive int
    array_indexing: ($) =>
      prec.left(
        PREC.CALL,
        seq(
          field("array", $._expression),
          "[",
          field("index", $._expression),
          "]",
        ),
      ),

    // both tuple and struct access
    field_access: ($) =>
      prec.left(
        PREC.CALL,
        seq(
          field("object", $._expression),
          ".",
          field("field", choice($.identifier, $.int_literal)),
        ),
      ),

    // LITERALS
    string_literal: ($) => /"([^"\\]|\\.)*"/,
    int_literal: ($) => /\d+/,
    float_literal: ($) => /\d+\.\d+/,
    bool_literal: ($) => choice("true", "false"),
    array_literal: ($) => seq("[", commaSep($._expression), "]"),
    tuple_literal: ($) => seq("(", commaSep($._expression), ")"),

    struct_field_initializer: ($) =>
      seq(field("field", $.identifier), ":", field("value", $._expression)),
    struct_literal: ($) => seq("{", commaSep($.struct_field_initializer), "}"),

    // VARIABLES AND ASSIGNMENT
    identifier: ($) => /[a-zA-Z_][a-zA-Z0-9_]*/,

    identifier_type_annotation: ($) =>
      seq(
        optional("mut"),
        field("name", $.identifier),
        ":",
        field("type", $._type),
      ),

    let_declaration: ($) =>
      seq(
        "let",
        field("identifier_type", $.identifier_type_annotation),
        "=",
        field("value", $._expression),
        ";",
      ),

    assignment_statement: ($) =>
      seq(
        field("left", choice($.identifier, $.array_indexing, $.field_access)),
        "=",
        field("right", $._expression),
        ";",
      ),

    // TYPES
    _type: ($) =>
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

    array_type: ($) =>
      seq(
        "[",
        field("type", $._type),
        optional(seq(";", field("length", $.int_literal))),
        "]",
      ),
    tuple_type: ($) => seq("(", commaSep(field("types", $._type)), ")"),

    // This allows &int, &mut str, shared &[int; 4], etc.
    reference_type: ($) =>
      seq(
        field("is_shared", optional("shared")),
        "&",
        field("is_mut", optional("mut")),
        field("type", $._type),
      ),

    function_type: ($) =>
      seq(
        field("argument_types", $.tuple_type),
        "->",
        field("return_type", $._type),
      ),

    // CONTROL FLOW
    function_definition: ($) =>
      seq(
        "fun",
        field("name", $.identifier),
        $._parameter_list,
        "->",
        field("return_type", $._type),
        field("body", $.block),
      ),

    _parameter_list: ($) =>
      seq(
        "(",
        commaSep(field("parameters", $.identifier_type_annotation)),
        ")",
      ),

    block: ($) =>
      seq(
        "{",
        field("statements", repeat($._statement)),
        optional(field("final_expression", $._expression)),
        "}",
      ),

    // this is for choice literals `Option.Some(5)` and function calls `print("hi")
    call_expression: ($) =>
      prec.left(
        PREC.CALL,
        seq(
          field("callable", $._expression),
          "(",
          commaSep(field("arguments", $._expression)),
          ")",
        ),
      ),

    if_else: ($) =>
      seq(
        "if",
        field("condition", $._expression),
        field("consequence", $.block),
        optional(seq("else", field("alternative", choice($.block, $.if_else)))),
      ),

    match_expression: ($) =>
      seq(
        "match",
        field("value", $._expression),
        "{",
        repeat(
          prec.left(PREC.BITWISE_OR, seq("|", field("cases", $.match_case))),
        ),
        "}",
      ),

    match_case: ($) =>
      prec(
        PREC.BITWISE_OR,
        seq(
          field("pattern", $._expression),
          optional(seq("if", field("guard", $._expression))),
          "->",
          field("body", $._match_body),
        ),
      ),

    _match_body: ($) => prec(PREC.BITWISE_OR, $._expression),

    while_loop: ($) =>
      seq("while", field("condition", $._expression), field("body", $.block)),

    for_loop: ($) =>
      seq(
        "for",
        field("bindings", commaSep($.identifier_type_annotation)),
        "in",
        field("iterator", $._expression),
        field("body", $.block),
      ),

    // Break can optionally take an expression to return a value from a loop
    break_expression: ($) =>
      prec.right(seq("break", optional(field("value", $._expression)))),

    continue_expression: ($) => "continue",

    return_expression: ($) =>
      prec.right(seq("return", optional(field("value", $._expression)))),

    wildcard_expression: ($) => "_",

    // ADVANCED DEFINITIONS

    struct: ($) =>
      seq(
        "struct",
        field("name", $.identifier),
        "{",
        repeat(
          seq(field("fields", $.identifier_type_annotation), optional(",")),
        ),
        "}",
      ),
    anonymous_function: ($) =>
      seq($._parameter_list, "->", field("body", $._expression)),

    choice_variant: ($) =>
      seq(
        "|",
        choice(
          field("name", $.identifier),
          seq(
            field("name", $.identifier),
            "(",
            field("types", commaSep1($._type)),
            ")",
          ),
        ),
      ),
    choice_definition: ($) =>
      seq(
        "choice",
        field("name", $.identifier),
        "{",
        field("choice_variants", repeat($.choice_variant)),
        "}",
      ),

    // MACHINE
    machine_definition: ($) =>
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
            field("events", $.event_declaration),
            field("states", $.state_declaration),
            field("initializers", $.machine_init),
            field("state_behaviors", $.machine_state_behavior),
          ),
        ),
        "}",
      ),

    // `event AddItem(price: int)` or `event StartCheckout`
    event_declaration: ($) =>
      seq("event", field("name", $.identifier), optional($._parameter_list)),

    // `state Browsing(cart_id: int, item_count: int, subtotal: int)` or `state Terminated`
    state_declaration: ($) =>
      seq("state", field("name", $.identifier), optional($._parameter_list)),

    // `init(id: int) -> ShoppingCart { ... }`
    machine_init: ($) =>
      seq(
        "init",
        $._parameter_list,
        "->",
        field("return_type", $._type),
        field("body", $.block),
      ),

    // `in Browsing(cart_id: string, item_count: int, subtotal: float) { ... }`
    machine_state_behavior: ($) =>
      seq(
        "in",
        field("state", $.identifier),

        optional(field("bindings", $._parameter_list)),
        "{",
        // Inside an `in` block, we only allow `on` handlers
        repeat(field("event_handlers", $.machine_event_handler)),
        "}",
      ),

    // `on AddItem(price: int) -> ShoppingCart { ... }`
    // `on _ -> ShoppingCart { ... }`
    machine_event_handler: ($) =>
      seq(
        "on",
        field(
          "event",
          choice(
            $.identifier,
            $.wildcard_expression, // Allows wildcard fallback
          ),
        ),
        optional(field("bindings", $._parameter_list)),
        "->",
        field("return_type", $._type),
        field("body", $.block),
      ),
  },
});
