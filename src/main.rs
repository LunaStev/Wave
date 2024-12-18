mod lexer;
mod parser;
mod ast;
mod error;

use lexer::Lexer;
use parser::Parser;

fn main() {
    // Sample code to parse and run
    let code_a = "fun main() {    var a: i32 = 10;    println();    }";

    // Create a Lexer
    let mut lexer = Lexer::new(code_a);

    // Tokenize the source code
    let tokens = lexer.tokenize();

    // Create a Parser
    let mut parser = Parser::new(lexer);

    let ast = parser.parse();

    println!("{:?}", tokens);
    println!("{:?}", parser);
    println!("{:?}", ast);
}