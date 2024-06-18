pub mod compiler;

pub use crate::ast::{Node, Operator};

pub trait Compile {
    type Output;

    fn from_ast(ast: Vec<Node>) ->
}