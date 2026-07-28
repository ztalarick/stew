//! Loom AST (Abstract Syntax Tree)

/// Start and end bytes mapping to source string
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize
}


/// Node with the span it covers
#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

// ---------------------------------------------------------
// TOP LEVEL
// ---------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Spanned<Statement>>,
}

#[derive(Debug, Clone)]
pub enum Statement {
    /// expression with a semicolon: `do_something();`
    ExprStatement(Box<Spanned<Expression>>),
    
    // Control Flow
    
    
    // $.function_definition,
    // $.if_else,
    // $.block,
    // $.for_loop,
    // $.while_loop,
    // $.match_expression,
    
    // Variables

    /// let mut x: int = 5;
    LetDeclaration {
        is_mut: bool,
        identifier: String,
        type_annotation: Type,
        value: Box<Spanned<Expression>>,
    },

    Assignment {
        target: Spanned<AssignTarget>,
        value: Box<Spanned<Expression>>,
    }
    
    
    // Advanced Definitions
    
    // $.struct,
    // $.choice,
    // $.machine,
}

/// Represents a location in memory that can be mutated
#[derive(Debug, Clone)]
pub enum AssignTarget {
    /// `x`
    Variable(String),
    
    /// `target[index]`
    Index {
        target: Box<Spanned<AssignTarget>>,
        index: Box<Spanned<Expression>>,
    },
    
    /// `target.field`
    StructField {
        target: Box<Spanned<AssignTarget>>,
        field_name: String,
    },
    
    /// `target.0`
    TupleField {
        target: Box<Spanned<AssignTarget>>,
        index: usize,
    }
}


// ---------------------------------------------------------
// EXPRESSIONS
// ---------------------------------------------------------

#[derive(Debug, Clone)]
pub enum Expression {
    // Literals
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    // $.array,
    // $.tuple,
    // $.struct,

    // Complex Expressions
    
    /// Math & Logic:  left + right
    Binary {
        op: BinaryOperation,
        left: Box<Spanned<Expression>>,
        right: Box<Spanned<Expression>>,
    },
    // $.unary_expression,
    // $.array_indexing,
    // $.field_access,
    // $.address_of,
    // $.wildcard_expression,
    
    // Control flow
    Identifier(String),
    If {
        condition: Box<Spanned<Expression>>,
        consequence: Vec<Spanned<Statement>>, // A block of statements
        alternative: Option<Vec<Spanned<Statement>>>, 
    }
    // $.call_expression,
    // $.block,
    // $.anonymous_function,
    // $.break_expression,
    // $.continue_expression,
    // $.return_expression,
    // $.match_expression,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperation {
    // Math
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,

    // Comparison
    Equals,
    NotEquals,
    GreaterThan,
    GreaterThanEquals,
    LessThan,
    LessThanEquals,

    // Boolean
    BooleanOr,
    BooleanAnd,

    // Bitwise
    LeftShift,
    RightShift,
    BitwiseAnd,
    BitwiseOr,
}

// ---------------------------------------------------------
// TYPES
// ---------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    U8,
    I64,
    F64,
    Bool,
    Str,
    Char,
    Function {
        arguments: Vec<Type>,
        return_type: Box<Type>
    },
    Tuple(Vec<Type>),

    /// Distinct unit type for void returns or discarded expressions
    Unit,

    Array {
        item: Box<Type>,
        length: usize,
    },

    /// Custom structs, choices, or state machines
    Custom(String),

    Reference {
        is_mut: bool,
        is_shared: bool,
        inner: Box<Type>,
    },
}