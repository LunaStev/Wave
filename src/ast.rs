#[derive(Debug)]
pub enum Expr {
    Number(i64),
    Variable(String),
    StringLiteral(String),
    Identifier(String),
    Binary(Box<Expr>, Operator, Box<Expr>),
}

#[derive(Debug)]
pub enum Statement {
    VariableDeclaration(String, Box<Expr>, Box<Statement>),
    FunctionDeclaration(String, Vec<String>, Box<Statement>),
    IfStatement(Box<Expr>, Box<Statement>, Option<Box<Statement>>),
    WhileStatement(Box<Expr>, Box<Statement>),
    ImportStatement(String),
    Print(Box<Expr>),
    Empty,
}

#[derive(Debug)]
pub enum Operator {
    Plus,
    Minus,
    Star,
    Div,
    Assign,
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub body: Vec<Statement>,
}