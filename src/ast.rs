#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Text(String),
}

#[derive(Debug, Clone)]
pub enum ASTNode {
    Function {
        name: String,
        params: Vec<String>,
        body: Vec<ASTNode>,
    },
    Variable {
        name: String,
        var_type: String,
        value: Value,
    },
    IfStatement {
        condition: String,
        body: Vec<ASTNode>,
        else_body: Option<Vec<ASTNode>>,
    },
    WhileLoop {
        condition: String,
        body: Vec<ASTNode>,
    },
    ForLoop {
        init: Box<ASTNode>,
        condition: String,
        increment: Box<ASTNode>,
        body: Vec<ASTNode>,
    },
    Import {
        module_name: String,
    },
    Print {
        message: String,
        newline: bool,
    },
    Literal {
        value: String,
    },
    Expression {
        operator: String,
        left: Box<ASTNode>,
        right: Box<ASTNode>,
    },
}

#[derive(Debug, Clone)]
pub struct AST {
    pub nodes: Vec<ASTNode>,
}

impl AST {
    pub fn new() -> Self {
        AST { nodes: Vec::new() }
    }

    pub fn add_node(&mut self, node: ASTNode) {
        eprintln!("Adding node to AST: {:?}", node);
        self.nodes.push(node);
    }
}
