use crate::lexer::TokenType;

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Text(String),
}

#[derive(Debug, Clone)]
pub enum ASTNode {
    Function(FunctionNode),
    Program(ParameterNode),
    Statement(StatementNode),
    Variable(VariableNode),
}

#[derive(Debug, Clone)]
pub struct FunctionNode {
    pub name: String,
    pub parameters: Vec<ParameterNode>,
    pub body: Vec<ASTNode>,
}

#[derive(Debug, Clone)]
pub struct ParameterNode {
    pub name: String,
    pub param_type: String, // For simplicity, assuming type as string.
    pub initial_value: Option<String>,
}

#[derive(Debug, Clone)]
pub enum StatementNode {
    Print(String),
    Println(String),
    Variable(String),
    If { condition: String, body: Vec<ASTNode> },
    For { iterator: String, body: Vec<ASTNode> },
    While { condition: String, body: Vec<ASTNode> },
}

#[derive(Debug, Clone)]
pub struct VariableNode {
    pub name: String,
    pub type_name: String,
    pub initial_value: Option<String>,
}

/*
#[derive(Debug, Clone)]
pub struct AST {
    pub nodes: Vec<ASTNode>,
}

impl AST {
    pub fn new() -> Self {
        AST {
            nodes: Vec::new()
        }
    }

    pub fn add_node(&mut self, node: ASTNode) {
        eprintln!("Adding node to AST: {:?}", node);
        self.nodes.push(node);
    }
}

 */