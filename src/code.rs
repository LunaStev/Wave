use std::collections::HashMap;
use crate::ast::{Expr, Statement};

pub struct Env {
    vars: HashMap<String, i32>,
}

impl Env {
    pub fn new() -> Self {
        Env {
            vars: HashMap::new(),
        }
    }

    pub fn set(&mut self, name: String, value: i32) {
        self.vars.insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<i32> {
        self.vars.get(name).cloned()
    }
}

pub fn codegen(stmt: (), env: &mut Env) {
    match stmt {
        Statement::FunctionDeclaration(_, exprs) => {
            for expr in exprs {
                match expr {
                    Expr::Print(msg) => {
                        print!("{}", msg); // print doesn't add newline
                    }
                    Expr::Println(msg) => {
                        println!("{}", msg); // println adds newline
                    }
                    Expr::Variable(name, value) => {
                        env.set(name, value);
                    }
                }
            }
        }
    }
}