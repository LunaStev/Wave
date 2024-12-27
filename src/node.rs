use crate::ast::AST;
use crate::lexer::Lexer;
use crate::parser::Parser;

pub(crate) fn function_node() {
    let function = Parser::function(&mut Parser {
        lexer: Lexer {
            source: "",
            current: 0,
            line: 0,
        },
        current_token: Default::default()
        }, &mut AST {
            nodes: vec![]
        }
    );
    println!("{:?}", function);
}