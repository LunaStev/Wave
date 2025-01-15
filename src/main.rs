mod lexer;
mod parser;
mod error;

use std::{env, fs, process};
use lexer::{Lexer, Token};
use crate::lexer::TokenType;
use crate::parser::{extract_body, extract_parameters, function};

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

fn format_ast(ast: &AST) -> String {
    format!(
        "{{\n  nodes: {:?}\n}}",
        ast.nodes
    )
}
 */

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <path_to_wave_file>", args[0]);
        process::exit(1);
    }

    let file_path = &args[1];

    let code = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Error reading file {}: {}", file_path, err);
            process::exit(1);
        }
    };

    let mut lexer = Lexer::new(code.as_str());

    let tokens = lexer.tokenize();
    eprintln!("Tokens: {}", format_tokens(&tokens));

    let function_name = tokens.iter()
        .find(|token| matches!(token.token_type, TokenType::IDENTIFIER(_)))
        .map(|token| token.lexeme.clone())
        .unwrap_or_default();

    let params = extract_parameters(&tokens);

    let mut peekable_tokens = tokens.iter().peekable();
    let body = extract_body(&mut peekable_tokens);

    let ast = function(function_name, params, body);

    eprintln!("AST: {:?}", &ast);
}