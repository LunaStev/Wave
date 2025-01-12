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

fn extract_function_name(input: &str) -> String {
    input[4..input.find("(").unwrap()].trim().to_string()
}

fn extract_parameters(tokens: &str) -> Vec<ParameterNode> {
    let mut params = Vec::new();

    if let Some(token) = tokens.get(0) {
        if token.token_type == TokenType::LPAREN {
            tokens.remove(0);
            if let Some(next_token) = tokens.get(0) {
                if next_token.token_type == TokenType::RPAREN {
                    tokens.remove(0);
                    return params;
                }
            }

        }
    }

    params
}

pub fn parse(input: &str) {
    let tokens = Lexer::tokenize(input);

    let mut token_vec = tokens;
    let params = extract_parameters(&mut token_vec);
}
