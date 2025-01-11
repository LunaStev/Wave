#[derive(Debug, Clone)]
pub enum Value {

}

#[derive(Debug, Clone)]
pub enum ASTNode {

}

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
