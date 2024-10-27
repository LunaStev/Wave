use crate::ast::ASTNode;

pub fn check(ast: &ASTNode) -> Result<(), String> {
    match ast {
        ASTNode::Function { body, .. } => {
            for stmt in body {
                match stmt {
                    ASTNode::Println(content) => {
                        if content.is_empty() {
                            return Err("Println에 빈 문자열이 전달되었습니다.".to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    Ok(())
}
