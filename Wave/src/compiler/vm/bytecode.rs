use std::arch::x86_64::_mm_or_ps;
use crate::compiler::vm::{make_op, OpCode };
use crate::{ Compile, Node, Operator };
use crate::compiler::vm::OpCode::{OpAdd, OpMinus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bytecode {
    pub instructions: Vec<u8>,
    pub constants: Vec<Node>,
}

impl Bytecode {
    fn new() -> Self {
        Self {
            instructions: Vec::new(),
            constants: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct Interpreter {
    bytecode: Bytecode,
}

impl Compile for Interpreter {
    type Output = Bytecode;

    fn from_ast(ast: Vec<Node>) -> Self::Output {
        let mut interpreter = Interpreter {
            bytecode: Bytecode::new(),
        };

        for node in ast {
            println!("comiling node {:?}", node);
            interpreter.interpret_node(node);
            interpreter.add_instruction(OpCode::OpPop);
        } interpreter.bytecode
    }
}

impl Interpreter {
    fn add_constant(&mut self, node: Node) -> u16 {
        self .bytecode.constants.push(node);
        (self.bytecode.constants.len() - 1) as u16
    }

    fn add_instruction(&mut self, op_code: OpCode) -> u16 {
        let position_of_new_instruction = self.bytecode.instructions.len() as u16;
        self.bytecode.instructions.extend(make_op(op_code));
        println!(
            "added instructions {:?} from opcode {:?}",
            self.bytecode.instructions,
            op_code.clone()
        );
        position_of_new_instruction
    }

    fn interpret_node(&mut self, node: Node) {
        match node {
            Node::Int(num) => {
            let const_index = self.add_constant(Node::Int(num));
                self.add_instruction((OpCode::OpConstant(const_index)));
            }

            Node::UnaryExpr { op, child } => {
                self.interpret_node(*child);
                match op {
                    Operator::PLUS => self.add_instruction(OpCode::OpPlus),
                    Operator::MINUS => self.add_instruction(OpCode::OpMinus),
                };
            }

            Node::BinaryExpr { op, lhs, rhs} => {
                self.interpret_node(*lhs);
                self.interpret_node(*rhs);

                match op {
                    Operator::PLUS => self.add_instruction(OpCode::OpAdd),
                    Operator::MINUS => self.add_instruction(OpCode::OpSub),
                };
            }
        };
    }
}

#[cfg(test)]
mod tests {

}