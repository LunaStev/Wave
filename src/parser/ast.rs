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
