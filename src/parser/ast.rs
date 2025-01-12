#[derive(Debug, Clone)]
pub enum Value {

}

#[derive(Debug, Clone)]
pub enum ASTNode {
    Function(FunctionNode)
}

#[derive(Debug, Clone)]
pub struct FunctionNode {
    pub name: String,
    pub params: Vec<ParameterNode>,
    pub body: Vec<ASTNode>,
    pub is_entry_point: bool,
}

#[derive(Debug, Clone)]
pub struct ParameterNode {
    pub name: String,
    pub param_type: String,
}