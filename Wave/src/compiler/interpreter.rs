#![allow(clippy::only_used_in_recursion)]
use crate::{Compile, Node, Operator, Result};

pub struct Interpreter;

impl Compile for Interpreter {
    type Output = Result<i32>;

    fn from_ast(ast: Vec<Node>) -> Self::Output {
        let mut ret = 0i32;
        let evaluator = Eval::new();
        for node in ast {
            ret += evaluator.dval(&node);
        } Ok(ret)
    }
}

struct Eval;

impl Eval {
    pub fn new() -> Self {
        Self
    }

    pub fn eval(&self, node: &Node) -> i32 {
        match node {
            Node::Int(n) => *n,
            Node::UnaryExpr {op, child } => {
                let child = self.eval(child);
                match op {
                    Operator::PLUS => child,
                    Operator::MINUS => -child,
                }
            }
            Node::BinaryExpr { op, lhs, rhs } => {
                let lhs_ret = self.eval(lhs);
                let rhs_ret = self.eval(rhs);

                match op {
                    Operator::PLUS => lhs_ret + rhs_ret,
                    Operator::MINUS => lhs_ret - rhs_ret,
                }
            }

        }
    }
}

#[cfg(test)]
mod tests {

}