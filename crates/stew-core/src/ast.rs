//! Stew AST (Abstract Syntax Tree)

/// Start and end bytes mapping to source string
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

/// Node with the span it covers
#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(node: T, start: usize, end: usize) -> Self {
        Self {
            node,
            span: Span { start, end },
        }
    }
}

// Only compare the node, not the span
impl<T: PartialEq> PartialEq for Spanned<T> {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
    }
}

impl<T: Eq> Eq for Spanned<T> {}

// Aliases
pub type BoxExpr = Box<Spanned<Expression>>;
pub type BoxStmt = Box<Spanned<Statement>>;
pub type BoxAssignTarget = Box<Spanned<AssignTarget>>;
pub type BoxType = Box<Spanned<Type>>;

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
    ExprStatement(BoxExpr),

    // Control Flow
    Function {
        name: String,
        parameters: Vec<TypedIdentifier>,
        return_type: Spanned<Type>,
        body: BoxExpr,
    },

    // Variables
    /// let mut x: int = 5;
    LetDeclaration {
        declaration: TypedIdentifier,
        value: BoxExpr,
    },

    Assignment {
        target: Spanned<AssignTarget>,
        value: BoxExpr,
    },

    // Advanced Definitions

    // $.struct,
    StructDefinition {
        name: String,
        field_types: Vec<TypedIdentifier>,
    },

    ChoiceDefinition {
        name: String,
        choices: Vec<ChoiceVariant>,
    },

    // $.machine,
    Machine {
        name: String,

        // Initililization logic
        init: MachineInit,

        // Choice of events
        events: Vec<MachineVariant>,

        // Choice of states
        states: Vec<MachineVariant>,

        // State Behvaiors
        state_handlers: Vec<MachineStateHandler>,
    },
}

#[derive(Debug, Clone)]
pub struct ChoiceVariant {
    pub name: String,
    pub types: Vec<Spanned<Type>>,
}

#[derive(Debug, Clone)]
pub struct MachineStateHandler {
    pub state: String,
    pub bindings: Vec<TypedIdentifier>,
    pub event_handlers: Vec<MachineEventHandler>,
}

#[derive(Debug, Clone)]
pub struct MachineEventHandler {
    pub event: Spanned<Expression>,
    pub bindings: Vec<TypedIdentifier>,
    pub return_type: Spanned<Type>,
    pub body: BoxExpr,
}

#[derive(Debug, Clone)]
pub struct MachineVariant {
    pub name: String,
    pub parameters: Vec<TypedIdentifier>,
}

#[derive(Debug, Clone)]
pub struct MachineInit {
    pub parameters: Vec<TypedIdentifier>,
    pub return_type: Spanned<Type>,
    pub body: BoxExpr,
}

/// Represents a location in memory that can be mutated
#[derive(Debug, Clone)]
pub enum AssignTarget {
    /// `x`
    Variable(String),

    /// `target[index]`
    Index {
        target: BoxAssignTarget,
        index: BoxExpr,
    },

    /// `target.field`
    StructField {
        target: BoxAssignTarget,
        field_name: String,
    },

    /// `target.0`
    TupleField {
        target: BoxAssignTarget,
        index: usize,
    },
}

#[derive(Debug, Clone)]
pub struct TypedIdentifier {
    pub is_mut: bool,
    pub name: String,
    pub var_type: Spanned<Type>,
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
    Array(Vec<Spanned<Expression>>),
    Tuple(Vec<Spanned<Expression>>),
    Struct {
        fields: Vec<(String, Spanned<Expression>)>,
    },

    // Complex Expressions
    /// Math & Logic:  left + right
    Binary {
        op: BinaryOperation,
        left: BoxExpr,
        right: BoxExpr,
    },

    Unary {
        op: UnaryOperation,
        operand: BoxExpr,
    },

    ArrayIndex {
        target: BoxExpr,
        index: BoxExpr,
    },

    Field {
        target: BoxExpr,
        field: FieldName,
    },

    /// &x
    AddressOf {
        is_mut: bool,
        target: BoxExpr,
    },

    Wildcard,

    // Control flow
    Identifier(String),

    If {
        condition: BoxExpr,
        consequence: BoxExpr,
        alternative: Option<BoxExpr>,
    },

    Call {
        callable: BoxExpr,
        arguments: Vec<Spanned<Expression>>,
    },
    Block {
        statements: Vec<Spanned<Statement>>,
        final_expression: Option<BoxExpr>,
    },

    AnonymousFunction {
        parameters: Vec<TypedIdentifier>,
        body: BoxExpr,
    },

    Match {
        target: BoxExpr,
        options: Vec<MatchCase>,
    },

    // Keywords
    Break(Option<BoxExpr>),
    Continue,
    Return(Option<BoxExpr>),

    // Loops
    For {
        bindings: Vec<TypedIdentifier>,
        iterator: BoxExpr,
        body: BoxExpr,
    },
    While {
        condition: BoxExpr,
        body: BoxExpr,
    },
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperation {
    Not,
    Negative,
}

#[derive(Debug, Clone)]
pub enum FieldName {
    Named(String), // For structs: "age", "name"
    Index(usize),  // For tuples: 0, 1, 2
}

#[derive(Debug, Clone)]
pub struct MatchCase {
    pub pattern: Spanned<Expression>,
    pub guard: Option<Spanned<Expression>>,
    pub body: Spanned<Expression>,
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
        arguments: Vec<Spanned<Type>>,
        return_type: BoxType,
    },
    Tuple(Vec<Spanned<Type>>),

    /// Distinct unit type for void returns or discarded expressions
    Unit,

    Array {
        item: BoxType,
        length: Option<usize>,
    },

    /// Custom structs, choices, or state machines
    Custom(String),

    Reference {
        is_mut: bool,
        is_shared: bool,
        inner: BoxType,
    },
}
