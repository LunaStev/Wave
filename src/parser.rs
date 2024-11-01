use crate::ast::{Expr, Function, Statement};
use crate::lexer::{Lexer, Token, TokenType};

#[derive(Debug)]
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current_token: Token,
}

impl<'a> Parser<'a> {
    pub fn new(lexer: Lexer<'a>) -> Self { // Modifying the parameters of the function as well
        let mut parser = Parser {
            lexer,
            current_token: Token::default(), // Initialize the default token
        };
        parser.advance(); // Initialize the current token
        parser
    }

    pub fn parse(&mut self) -> Result<Function, String> {
        if let TokenType::FUN = self.current_token.token_type {
            self.advance(); // Skip the 'fun' token

            if let TokenType::IDENTIFIER(name) = &self.current_token.token_type {
                self.advance(); // Skip function name
                if let TokenType::LPAREN = self.current_token.token_type {
                    self.advance(); // Open parentheses skip

                    if let TokenType::RPAREN = self.current_token.token_type {
                        self.advance(); // Close parentheses skip
                        if let TokenType::LBRACE = self.current_token.token_type {
                            self.advance(); // Open Brace Skip

                            let mut body = Vec::new();
                            while self.current_token.token_type != TokenType::RBRACE {
                                if let TokenType::VAR = self.current_token.token_type {
                                    self.advance(); // "Var" skip
                                    if let TokenType::IDENTIFIER(var_name) = &self.current_token.token_type {
                                        self.advance(); // Variable Name Skip
                                        if let TokenType::ASSIGN = self.current_token.token_type {
                                            self.advance(); // '=' Skip
                                            if let TokenType::NUMBER(value) = self.current_token.token_type {
                                                self.advance(); // number skip
                                                body.push(Statement::VariableDeclaration(var_name.clone(), Box::new(Expr::Number(value)), Box::new(Statement::Empty)));
                                            }
                                        }
                                    }
                                } else if let TokenType::PRINT = self.current_token.token_type {
                                    self.advance(); // 'print' Skip
                                    if let TokenType::LPAREN = self.current_token.token_type {
                                        self.advance(); // Open parentheses skip
                                        if let TokenType::IDENTIFIER(var_name) = &self.current_token.token_type {
                                            self.advance(); // Variable Name Skip
                                            if let TokenType::RPAREN = self.current_token.token_type {
                                                self.advance(); // Close parentheses
                                                body.push(Statement::Print(Box::new(Expr::Variable(var_name.clone()))));
                                            }
                                        }
                                    }
                                }

                                // Semicolon skip
                                if self.current_token.token_type == TokenType::SEMICOLON {
                                    self.advance();
                                }
                            }

                            self.advance(); // Close Bracelet Skip

                            return Ok(Function {
                                name: name.clone(),
                                body,
                            });
                        }
                    }
                }
            }
        }

        Err("Invalid function declaration".to_string())
    }

    fn advance(&mut self) {
        self.current_token = self.lexer.next_token();
    }

    fn function(&mut self) -> Statement {
        // function syntax processing
        self.advance();

        // Save the name as a variable, and release the fire.
        let name = if let TokenType::IDENTIFIER(ref name) = self.current_token.token_type {
            name.clone()
        } else {
            panic!("Expected function name after 'fun'");
        };
        self.advance(); // Where variable borrowing occurs

        let mut params = Vec::new();
        self.expect(TokenType::LPAREN);

        while self.current_token.token_type != TokenType::RPAREN {
            if let TokenType::IDENTIFIER(param_name) = &self.current_token.token_type {
                params.push(param_name.clone());
                self.advance();

                if self.current_token.token_type == TokenType::COMMA {
                    self.advance();
                }
            } else {
                panic!("Expected parameter name");
            }
        }
        self.advance(); // Move after ')'

        self.expect(TokenType::LBRACE);
        let body = self.if_statement();
        self.expect(TokenType::RBRACE);

        Statement::FunctionDeclaration(name, params, Box::new(body))
    }

    fn variable(&mut self) -> Statement {
        // Syntaxing Variables
        let var_name = if let TokenType::IDENTIFIER(ref name) = self.current_token.token_type {
            name.clone()
        } else {
            panic!("Expected variable name after 'var'");
        };
        self.advance(); // Perform variable borrowings

        self.expect(TokenType::ASSIGN);
        let value = self.expr();
        self.expect(TokenType::SEMICOLON);

        Statement::VariableDeclaration(var_name, Box::new(value), Box::new(Statement::Empty))
    }

    fn if_statement(&mut self) -> Statement {
        // if syntax processing
        self.advance(); // `if`

        self.expect(TokenType::LPAREN); // '('
        let condition = self.expr(); // conditional expression
        self.expect(TokenType::RPAREN); // ')'
        self.expect(TokenType::LBRACE); // '{'

        let then_branch = self.if_statement(); // if block
        self.expect(TokenType::RBRACE); // '}'

        let else_branch = if self.current_token.token_type == TokenType::ELSE {
            self.advance();
            self.expect(TokenType::LBRACE); // '{'
            let else_body = self.if_statement(); // else block
            self.expect(TokenType::RBRACE); // '}'
            Some(Box::new(else_body))
        } else {
            None
        };

        Statement::IfStatement(Box::new(condition), Box::new(then_branch), else_branch)
    }

    fn while_statement(&mut self) -> Statement {
        // while syntax processing
        self.advance(); // `while`

        self.expect(TokenType::LPAREN); // '('
        let condition = self.expr(); // conditional expression
        self.expect(TokenType::RPAREN); // ')'
        self.expect(TokenType::LBRACE); // '{'

        let body = self.if_statement(); // while block
        self.expect(TokenType::RBRACE); // '}'

        Statement::WhileStatement(Box::new(condition), Box::new(body))
    }

    fn import_statement(&mut self) -> Statement {
        // import parsing
        let module_name = if let TokenType::STRING(ref name) = self.current_token.token_type {
            name.clone()
        } else {
            panic!("Expected a module name string");
        };
        self.advance(); // Perform variable borrowings

        self.expect(TokenType::RPAREN); // ')'
        self.expect(TokenType::SEMICOLON); // ';'

        Statement::ImportStatement(module_name)
    }

    fn expr(&mut self) -> Expr {
        let token_type = self.current_token.token_type.clone(); // Unchangeable Borrowing
        match token_type {
            TokenType::NUMBER(value) => {
                self.advance();
                Expr::Number(value)
            },
            TokenType::STRING(string_value) => {
                self.advance();
                Expr::Identifier(string_value)
            },
            TokenType::IDENTIFIER(identifier) => {
                self.advance();
                Expr::Identifier(identifier)
            },
            _ => panic!("Unexpected expression type"),
        }
    }

    fn expect(&mut self, token_type: TokenType) {
        if self.current_token.token_type == token_type {
            self.advance();
        } else {
            panic!("Expected {:?}, cound {:?}", token_type, self.current_token.token_type)
        }
    }

}