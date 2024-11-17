#[derive(Debug)]
pub enum Expr {
    Number(i64),
    Variable(String, i32),
    StringLiteral(String),
    Identifier(String),
    Print(String),
    Println(String),
    Binary(Box<Expr>, Operator, Box<Expr>),
}

#[derive(Debug)]
pub enum Statement {
    VariableDeclaration(String, Box<Expr>, Box<Statement>),
    FunctionDeclaration(String, Vec<String>),
    IfStatement(Box<Expr>, Box<Statement>, Option<Box<Statement>>),
    WhileStatement(Box<Expr>, Box<Statement>),
    ImportStatement(String),
    Print(Box<Expr>),
    Println(Box<Expr>),
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