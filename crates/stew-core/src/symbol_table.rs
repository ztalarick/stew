use std::{collections::HashMap, fmt::Error, vec};

use crate::ast;

const GLOBAL_SCOPE_ID: usize = 0;

/// Table to resolve a name for a given scope.
/// Scopes are referenced by ScopeIds
pub struct SymbolTable {
    // ScopeIds refer to the index in this vector
    scope_tables: Vec<ScopeTable>,
}

struct ScopeTable {
    // ScopeId of the parent scope
    // None for Global Scope (top level)
    parent: Option<usize>,

    // The ScopeIds for each child scope
    children: Vec<usize>,

    // Map of name -> entity
    table: HashMap<String, SymbolInfo>,
}

// TOOD what to put in here, need anything else?
pub struct SymbolInfo {
    source_span: ast::Span,
    symbol_type: ast::Type,

    // TODO is this right?
    value: ast::Expression,
}

impl SymbolInfo {
    pub fn new(
        source_span: ast::Span,
        symbol_type: ast::Type,
        value: ast::Expression,
    ) -> Self {
        SymbolInfo { source_span, symbol_type, value }
    }
}

impl SymbolTable {
    // TODO: consider taking an AST to new function, and just building the symbol table here and exposing lookup functions
    // Allows us to hide symbol table algorithms and keep them inside this module
    // how does that work with LSPs? they want incremental stuff, hmm

    /// Intialize a new Symbol Table with global scope table
    pub fn new() -> Self {
        Self {
            scope_tables: vec![ScopeTable {
                parent: None,
                children: vec![],
                table: HashMap::new(),
            }],
        }
    }

    // TODO: Consider alternative approach; Instead of taking the parent/children scope, have a cursor which tracks the scope when building.
    // TODO: Condsider how to handle errors. What happens if it sets the parent_scope_id to itself, or it sets to an index which doesn't exist?
    // I guess if we use a speical Struct wrapper for usize ScopeId, then it would be harder to pass an incorrect number (client would have to explicitly initialize an incorrect id)

    /// Create a new scope and append to parent's children
    /// Returns the scope_id of the new scope
    pub fn new_scope(&mut self, parent_scope_id: usize) -> usize {
        self.scope_tables.push(ScopeTable {
            parent: Some(parent_scope_id),
            children: vec![],
            table: HashMap::new(),
        });
        let new_scope_id = self.scope_tables.len();

        self.scope_tables[parent_scope_id]
            .children
            .push(new_scope_id);

        return new_scope_id;
    }

    // TODO: Again consider how to handle errors, scope_id could not be a real scope

    /// Adds a new symbol to the table for given scope_id
    pub fn add_symbol(&mut self, scope_id: usize, name: String, info: SymbolInfo) -> () {
        self.scope_tables[scope_id].table.insert(name, info);
    }

    /// Lookup a name for the curent scope or any of its parents
    pub fn lookup_reference(&self, name: &String, mut scope_id: usize) -> Option<&SymbolInfo> {
        loop {
            let scope = &self.scope_tables[scope_id];

            if let Some(info) = scope.table.get(name) {
                return Some(info);
            }

            scope_id = scope.parent?;
        }
    }
}

pub enum ScopeError {
    CannotLeaveGlobalScope,
}

pub struct ScopeCursor {
    symbol_table: SymbolTable,
    current_scope: usize,
}

// TODO these function names are terrible should they be the same as symbol table?

/// Cursor to track scope while traversing
impl ScopeCursor {
    pub fn new(symbol_table: SymbolTable) -> Self {
        ScopeCursor {
            symbol_table,
            current_scope: GLOBAL_SCOPE_ID,
        }
    }

    pub fn enter_new_scope(&mut self) -> () {
        self.current_scope = self.symbol_table.new_scope(self.current_scope);
    }

    pub fn leave_scope(&mut self) -> Result<(), ScopeError> {
        let parent = self.symbol_table.scope_tables[self.current_scope]
            .parent
            .ok_or(ScopeError::CannotLeaveGlobalScope)?;
        self.current_scope = parent;
        Ok(())
    }

    pub fn lookup(&self, name: String) -> Option<&SymbolInfo> {
        self.symbol_table
            .lookup_reference(&name, self.current_scope)
    }

    pub fn insert(&mut self, name: String, info: SymbolInfo) {
        self.symbol_table.add_symbol(self.current_scope, name, info);
    }
}
