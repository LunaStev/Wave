use crate::lexer::{FloatType, IntegerType, Lexer, Token, TokenType};
use crate::parser::ast::{AST, ASTNode, Value};

#[derive(Debug)]
pub struct Parser<'a> {
    pub lexer: Lexer<'a>,
    pub current_token: Token,
}

impl<'a> Parser<'a> {
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        let current_token = lexer.next_token();
        Parser { lexer, current_token }
    }

    pub fn parse(&mut self) -> AST {
        let mut ast = AST::new();
        eprintln!("Start parsing...");

        while self.current_token.token_type != TokenType::EOF {
            eprintln!("Current Token: {:?}", self.current_token);

            match self.current_token.token_type {
                TokenType::FUN => {
                    eprintln!("Parsing function...");
                    self.function(&mut ast)
                },
                TokenType::VAR => {
                    eprintln!("Parsing variable...");
                    self.variable(&mut ast)
                },
                TokenType::IF => {
                    eprintln!("Parsing if statement...");
                    self.if_statement(&mut ast)
                },
                TokenType::WHILE => {
                    eprintln!("Parsing while statement...");
                    self.while_statement(&mut ast)
                },
                TokenType::FOR => {
                    eprintln!("Parsing for statement...");
                    self.for_statement()
                },
                TokenType::IMPORT => {
                    eprintln!("Parsing import statement...");
                    self.import_statement(&mut ast)
                },
                TokenType::PRINT | TokenType::PRINTLN => {
                    eprintln!("Parsing print statement...");
                    self.print_statement(&mut ast)
                },
                _ => {
                    eprintln!("Unknown token: {:?}", self.current_token.token_type);
                    self.advance()
                },
            }
        }

        if ast.nodes.is_empty() {
            eprintln!("Warning: The AST is empty. No nodes were parsed.");
        } else {
            eprintln!("AST has nodes: {:?}", ast.nodes);
        }

        ast
    }

    fn advance(&mut self) {
        eprintln!("Advancing from token: {:?}", self.current_token);
        self.current_token = self.lexer.next_token();
        eprintln!("Advanced to token: {:?}", self.current_token);
    }

    pub fn function(&mut self, ast: &mut AST) {

    }


    fn variable(&mut self, ast: &mut AST) {

    }


    fn print_statement(&mut self, ast: &mut AST) {

    }

    fn if_statement(&mut self, ast: &mut AST) {

    }

    params
}

pub fn parse(input: &str) {
    let tokens = Lexer::tokenize(input);

    let mut token_vec = tokens;
    let params = extract_parameters(&mut token_vec);
}
