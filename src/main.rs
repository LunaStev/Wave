mod lexer;
mod parser;
mod error;

use std::fs;
use lexer::{Lexer, Token};
use crate::parser::parse;

fn format_tokens(tokens: &Vec<Token>) -> String {
    let mut result = String::new();
    result.push_str("[\n");
    for token in tokens {
        result.push_str(&format!(
            "  {{\n    token_type: {:?},\n    lexeme: {:?},\n    line: {}\n  }},\n",
            token.token_type, token.lexeme, token.line
        ));
    }
    result.push_str("]");
    result
}

/*
fn format_parser(parser: &Parser) -> String {
    format!(
        "{{\n  lexer: {{\n    source: {:?},\n    current: {},\n    line: {}\n  }},\n  current_token: {{\n    token_type: {:?},\n    lexeme: {:?},\n    line: {}\n  }}\n}}",
        parser.lexer.source,
        parser.lexer.current,
        parser.lexer.line,
        parser.current_token.token_type,
        parser.current_token.lexeme,
        parser.current_token.line
    )
}


fn format_ast(ast: &ASTNode) -> String {
    format!(
        "{{\n  nodes: {:?}\n}}",
        ast.nodes
    )
}
 */

fn main() {
    let code = fs::read_to_string("test/test4.wave").expect("Failed to read the file");

    // Create a Lexer
    let mut lexer = Lexer::new(code.as_str());

    // Tokenize the source code
    let tokens = lexer.tokenize();

    // a formalized output
    eprintln!("Tokens: {}", format_tokens(&tokens));
    // 여러 개의 `fun` 키워드를 찾아서 함수들을 파싱
    for line in code.lines() {
        let parsed_ast = parse(line);
        println!("{:?}", parsed_ast);
    }
}