mod lexer;
mod parser;
mod ast;
mod error;

use lexer::Lexer;
use parser::Parser;

fn main() {
    // Sample code to parse and run
    let code_a = r#"
        fun main() {
            var a: isz = 30;
            var a: i4 = 30;
            var a: i8 = 30;
            var a: i16 = 30;
            var a: i32 = 30;
            var a: i64 = 30;
            var a: i128 = 30;
            var a: i512 = 30;
            var a: i1024 = 30;
            var a: i2048 = 30;
            var a: i4090 = 30;
            var a: i8192 = 30;
            var a: i16384 = 30;
            var a: i32768 = 30;
            var b: usz = 30;
            var b: u4 = 30;
            var b: u8 = 30;
            var b: u16 = 30;
            var b: u32 = 30;
            var b: u64 = 30;
            var b: u128 = 30;
            var b: u512 = 30;
            var b: u1024 = 30;
            var b: u2048 = 30;
            var b: u4090 = 30;
            var b: u8192 = 30;
            var b: u16384 = 30;
            var b: u32768 = 30;
            var c: f32 = 30;
            var c: f64 = 30;
            var c: f128 = 30;
            var c: f512 = 30;
            var c: f1024 = 30;
            var c: f2048 = 30;
            var c: f4090 = 30;
            var c: f8192 = 30;
            var c: f16384 = 30;
            var c: f32768 = 30;
            var d: str = "Hel";
        }
    "#;

    // Create a Lexer
    let mut lexer = Lexer::new(code_a);

    // Tokenize the source code
    let tokens = lexer.tokenize();

    // Create a Parser
    // let mut parser = Parser::new(lexer);

    // Start parsing the tokens
    // parser.parse();
    println!("{:?}", tokens);
}