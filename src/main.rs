mod lexer;
mod parser;
mod ast;
mod error;

use lexer::Lexer;
use parser::Parser;

fn main() {
    // Sample code to parse and run
    let code = r#"
    fun main() {
        print("Hello World\n");
        println("Hello World");
    }
    "#;

    // Create a Lexer
    let mut lexer = Lexer::new(code);

    // Tokenize the source code
    let tokens = lexer.tokenize();

    // Create a Parser
    let mut parser = Parser::new(lexer);

    // Start parsing the tokens
    parser.parse();
}
