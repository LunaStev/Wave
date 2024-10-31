mod lexer;
mod parser;
mod ast;
mod error;

use lexer::Lexer;
use parser::Parser;

fn main() {
    let source = r#"
    fun main() {
        var x = 42;
        print(x);
    }
    "#;

    let mut lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.tokenize().into_iter().collect();
    println!("Tokens: {:?}", tokens);

    let mut parser = Parser::new(lexer);
    println!("Parser: {:?}", parser)
}
