//! Stew Core
//!
//! This module contains the AST definition, Semantic analysis and the interpreter

pub mod ast;
mod symbol_table;

// TODO find somewhere to put this

mod populate_symbol_table {
    use crate::ast;
    use crate::symbol_table;
    

    pub fn build_symbol_table(program: &ast::Program) -> symbol_table::SymbolTable {
        let mut symbol_table = symbol_table::SymbolTable::new();
        let mut scope_cursor = symbol_table::ScopeCursor::new(symbol_table);


        program.statements.iter().map(|statement| {
            build_symbol_table_for_statement(
                &mut scope_cursor, 
                statement
            );
            statement
        });

        symbol_table
    }

    fn build_symbol_table_for_statement(
        scope_cursor: &mut symbol_table::ScopeCursor,
        statement: &ast::Spanned<ast::Statement>
    ) -> () {
        match statement.node {
            ast::Statement::ExprStatement(expr) => {
                build_symbol_table_for_expression(
                    scope_cursor,
                    &expr
                )
            }
            
            // This is a complete mess.
            // So many Box / Type / Spanned - its too much

            // Should I have a function to map expression -> type?

            ast::Statement::Function { name, parameters, return_type, body } => {
                scope_cursor.insert(
                    name,
                    symbol_table::SymbolInfo::new(
                        statement.span,
                        ast::Type::Function { 
                            // TODO: loosing is_mut data here - something is wrong
                            arguments: parameters.iter().map(|parameter| parameter.var_type.node).collect(),
                            return_type: Box<return_type> 
                        },
                        body
                    ));
            }

            ast::Statement::LetDeclaration { declaration, value } => {

            }

            ast::Statement::Assignment { target, value } => {

            }

            ast::Statement::StructDefinition => {

            } 

            ast::Statement::ChoiceDefinition { name, choices } => {

            }

            ast::Statement::Machine { name, init, events, states, state_handlers } => {

            }
        }
    }

    fn build_symbol_table_for_expression (
        scope_cursor: &mut symbol_table::ScopeCursor,
        expression: &ast::BoxExpr
    ) -> () {
        match expression.node {

        }
    }

}
