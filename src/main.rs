mod lexer;
mod parser;
mod ast;
mod error;
mod ast_generator;

use lexer::Lexer;
use parser::Parser;

fn main() {
    // Sample code to parse and run
    let code_a = r#"
    fun main() {
        var a: i32 = 10;
        println("World {}", a);
    }
    "#;

    // Create a Lexer
    let mut lexer = Lexer::new(code_a);

    // Tokenize the source code
    let tokens = lexer.tokenize();

    // Create a Parser
    let mut parser = Parser::new(lexer);

    println!("{:?}", tokens);
    println!("{:?}", parser);
}