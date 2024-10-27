use crate::ast::{ASTNode};
use crate::lexer::Token;

pub fn parse(tokens: &[Token]) -> Result<ASTNode, String> {
    let mut iter = tokens.iter().peekable();

    if let Some(Token::Fun) = iter.next() {
        if let Some(Token::Identifier(name)) = iter.next() {
            if iter.next() == Some(&Token::LParen) && iter.next() == Some(&Token::RParen) && iter.next() == Some(&Token::LBrace) {
                let mut body = Vec::new();

                while let Some(token) = iter.peek() {
                    match token {
                        Token::Println => {
                            iter.next();
                            if let Some(Token::StringLiteral(content)) = iter.next() {
                                body.push(ASTNode::Println(content.clone()));
                            }
                        }
                        _ => break,
                    }
                }

                if iter.next() == Some(&Token::RBrace) {
                    return Ok(ASTNode::Function { name: name.clone(), params: vec![], body });
                }
            }
        }
    }
    Err("구문 오류".to_string())
}
