#![allow(clippy::only_used_in_recursion)]
use crate::{Compile, Node, Operator, Result};

pub struct Interpreter;

impl Compile for Interpreter {
    type Output = Result<i32>;

    fn from_ast(ast: Vec<Node>) -> Self::Output {
        let mut ret = 0i32;
        let evaluator = Eval::new();
    }
}