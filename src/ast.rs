pub enum Expr {
    Number(i64),
    Var(String),
    BinOp(Box<Expr>, String, Box<Expr>),
    FunCall(String, Vec<Expr>),
    IfElse(Box<Expr>, Vec<Stmt>, Vec<Stmt>),
    While(Box<Expr>, Vec<Stmt>),
    Assign(String, Box<Expr>)
}

pub enum Stmt {
    ExprStmt(Expr),
    VarDecl(String, Expr),
    FunDecl(String, Vec<String>, Vec<Stmt>),
    Import(String)
}