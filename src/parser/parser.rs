use crate::lexer::{Lexer, TokenType};
use crate::parser::ast::{ASTNode, FunctionNode, ParameterNode};

pub fn parse_function(input: &str) -> ASTNode {
    let name = extract_function_name(input);

    let params = extract_parameters(input);

    let body = vec![];

    let is_entry_point = name == "main";

    ASTNode::Function(FunctionNode {
        name,
        params,
        body,
        is_entry_point,
    })
}

fn extract_function_name(input: &str) -> String {
    input[4..input.find("(").unwrap()].trim().to_string()
}

fn extract_parameters(tokens: &str) -> Vec<ParameterNode> {
    let mut params = Vec::new();

    if let Some(token) = tokens.get(0) {
        if token.token_type == TokenType::LPAREN {
            tokens.remove(0);
            if let Some(next_token) = tokens.get(0) {
                if next_token.token_type == TokenType::RPAREN {
                    tokens.remove(0);
                    return params;
                }
            }

        }
    }

    params
}

pub fn parse(input: &str) {
    let tokens = Lexer::tokenize(input);

    let mut token_vec = tokens;
    let params = extract_parameters(&mut token_vec);
}
