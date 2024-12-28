use std::fs;
use crate::ast::AST;
use crate::lexer::Lexer;
use crate::main;
use crate::parser::Parser;

pub(crate) fn function_node() {
    let code = fs::read_to_string("test2.wave").expect("Failed to read the file");
    let mut parser = Parser {
        lexer: Lexer {
            source: &code,
            current: 0,
            line: 0,
        },
        current_token: Default::default(),
    };

    let mut ast = AST { nodes: vec![] };

    parser.function(&mut ast);

    // AST에 추가된 함수 노드를 출력
    println!("{:#?}", ast);
}