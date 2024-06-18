pub mod compiler;
mod lexer;
mod parser;
mod ast;

pub use crate::ast::{Node, Operator};

pub trait Compile {
    type Output;

    fn from_ast(ast: Vec<Node>) ->
}