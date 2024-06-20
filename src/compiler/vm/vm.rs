use crate::compiler::vm::opcode::*;
use crate::compiler::vm::Bytecode;
use crate::Node;

const STACK_SIZE: usize = 512;

pub struct VM {
    bytecode: Bytecode,
    stack: [Node; STACK_SIZE],
    stack_ptr: usize,
}

impl VM {
    pub fn new(bytecode: Bytecode) -> Self {
        Self {
            bytecode,
            stack: unsafe {
                std::mem::zeroed()
            },
            stack_ptr: 0,
        }
    }

    pub fn run(&mut self) {
        let mut ip = 0;
        while ip < self.bytecode.instructions.len() {
            let inst_addr = ip;
            ip += 1;

            match self.bytecode.instructions[inst_addr] {
                0x01 => {
                    let const_idx = convert_two_u8s_to_usize(
                        self.bytecode.instructions[ip],
                        self.bytecode.instructions[ip + 1],
                    );
                    ip += 2;
                    self.push(self.bytecode.constants[const_idx].clone());
                }

                0x02 => {
                    self.pop();
                }

                0x03 => {
                    match (self.pop(), self.pop()) {
                        (Node::Int(rhs), Node::Int(lhs)) => self.push(Node::Int(lhs + rhs)),
                        _ => panic!("Unknown types to OpAdd"),
                    }
                }

                0x04 => {
                    match (self.pop(), self.pop()) {
                        (Node::Int(rhs), Node::Int(lhs)) => self.push(Node::Int(lhs - rhs)),
                        _ => panic!("Unknown types to OpSub"),
                    }
                }

                0x0A => {
                    match self.pop() {
                        Node::Int(num) => self.push(Node::Int(num)),
                        _ => panic!("Unknown arg type to OpPlus"),
                    }
                }
                0x0B => {
                    match self.pop() {
                        Node::Int(num) => self.push(Node::Int(-num)),
                        _ => panic!("Unknown arg type to OpMinus"),
                    }
                } _ => panic!("Unknown instruction"),
            }
        }
    }

    pub fn push(&mut self, node: Node) {
        self.stack[self.stack_ptr] = node;
        self.stack_ptr += 1;
    }

    pub fn pop(&mut self) -> Node {
        let node = self.stack[self.stack_ptr - 1].clone();
        self.stack_ptr -= 1;
        node
    }

    pub fn pop_last(&self) -> &Node {
        &self.stack[self.stack_ptr]
    }
}

#[cfg(test)]
mod tests {
    
}