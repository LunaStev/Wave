#[derive(Debug)]
pub enum ASTNode {
    Function {
        name: String,
        params: Vec<String>,
        body: Vec<ASTNode>,
    },
    Println(String),
    // 추가 AST 노드들
}
