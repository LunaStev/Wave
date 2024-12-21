mod lexer;
mod parser;
mod ast;
mod error;

use std::fs;
use lexer::Lexer;
use parser::Parser;
use crate::ast::AST;
use crate::lexer::Token;

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

fn main() {
    // Sample code to parse and run
    // Code part 1
    let code_a = fs::read_to_string("test.wave").expect("Failed to read the file");

    /*
    // Code part 2 (the part causing error)
    let code_b = r#"
    fun main() {
        var a: i32 = 10;
        println("Hello World {}", a);
        if (a == 10) {
            println("10은 a랑 같다.");
        } else if (a > 10) {
            println("10은 a보다 작다.");
        } else {
            println("10은 a보다 크다.");
        }
    }
    "#;
    */

    // Create a Lexer
    let mut lexer = Lexer::new(code_a.as_str());

    // Tokenize the source code
    let tokens = lexer.tokenize();

    // Create a Parser
    let mut parser = Parser::new(lexer);

    // Parse the AST
    let ast = parser.parse();

    // 형식화된 출력
    println!("Tokens: {}", format_tokens(&tokens));
    println!("\nParser: {}", format_parser(&parser));
    println!("\nAST: {}", format_ast(&ast));
}