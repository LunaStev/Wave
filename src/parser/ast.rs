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
    Expression(Expression),
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
    pub initial_value: Option<Value>,
}

#[derive(Debug, Clone)]
pub enum FormatPart {
    Literal(String),
    Placeholder,
}

#[derive(Debug, Clone)]
pub enum Expression {
    Literal(Literal),
    Variable(String),
    BinaryExpression {
        left: Box<Expression>,
        operator: Operator,
        right: Box<Expression>,
    },
    Grouped(Box<Expression>),
}

#[derive(Debug, Clone)]
pub enum Literal {
    Number(f64),
    String(String),
}

#[derive(Debug, Clone)]
pub enum Operator {
    Add,
    Subtract,
    Multiply,
    Divide,
    GreaterEqual,
    LessEqual,
    Greater,
    Less,
    Equal,
    NotEqual,
    LogicalAnd,
    BitwiseAnd,
    LogicalOr,
    BitwiseOr,
}

#[derive(Debug, Clone)]
pub enum StatementNode {
    Print(String),
    PrintFormat {
        format: String,
        args: Vec<Expression>,
    },
    Println(String),
    PrintlnFormat {
        format: String,
        args: Vec<Expression>,
    },
    Variable(String),
    If {
        condition: Expression,
        body: Vec<ASTNode>,
        else_if_blocks: Option<Box<Vec<ASTNode>>>,
        else_block: Option<Box<Vec<ASTNode>>>,
    },
    For {
        initialization: Expression,
        condition: Expression,
        increment: Expression,
        body: Vec<ASTNode>,
    },
    While {
        condition: Expression,
        body: Vec<ASTNode>,
    },
}

#[derive(Debug, Clone)]
pub struct VariableNode {
    pub name: String,
    pub type_name: String,
    pub initial_value: Option<Literal>,
}