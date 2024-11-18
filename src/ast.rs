#[derive(Debug)]
pub enum ASTNode {
    Function {
        name: String,
        params: Vec<String>,
        body: Vec<ASTNode>,
    },
    Variable {
        name: String,
        var_type: String,
        value: String,
    },
    Print {
        message: String,
    },
    If {
        condition: Box<ASTNode>,
        body: Vec<ASTNode>,
        else_body: Option<Vec<ASTNode>>,
    },
    While {
        condition: Box<ASTNode>,
        body: Vec<ASTNode>,
    },
    For {
        init: Option<Box<ASTNode>>,
        condition: Option<Box<ASTNode>>,
        increment: Option<Box<ASTNode>>,
        body: Vec<ASTNode>,
    },
    Import {
        module_name: String,
    },
    Literal {
        value: String,
        is_println: bool,
    },
    Number(i64),
    String(String),
}