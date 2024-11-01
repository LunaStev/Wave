use crate::ast::{Expr, Function, Statement};
use crate::lexer::{Lexer, Token, TokenType};

#[derive(Debug)]
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current_token: Token,
}

impl<'a> Parser<'a> {
    pub fn new(lexer: Lexer<'a>) -> Self { // 함수의 매개변수도 수정
        let mut parser = Parser {
            lexer,
            current_token: Token::default(), // 기본 토큰 초기화
        };
        parser.advance(); // 현재 토큰을 초기화
        parser
    }

    pub fn parse(&mut self) -> Result<Function, String> {
        if let TokenType::FUN = self.current_token.token_type {
            self.advance(); // 'fun' 토큰 스킵

            if let TokenType::IDENTIFIER(name) = &self.current_token.token_type {
                self.advance(); // 함수 이름 스킵
                if let TokenType::LPAREN = self.current_token.token_type {
                    self.advance(); // 여는 괄호 스킵

                    if let TokenType::RPAREN = self.current_token.token_type {
                        self.advance(); // 닫는 괄호 스킵
                        if let TokenType::LBRACE = self.current_token.token_type {
                            self.advance(); // 여는 중괄호 스킵

                            let mut body = Vec::new();
                            while self.current_token.token_type != TokenType::RBRACE {
                                if let TokenType::VAR = self.current_token.token_type {
                                    self.advance(); // 'var' 스킵
                                    if let TokenType::IDENTIFIER(var_name) = &self.current_token.token_type {
                                        self.advance(); // 변수 이름 스킵
                                        if let TokenType::ASSIGN = self.current_token.token_type {
                                            self.advance(); // '=' 스킵
                                            if let TokenType::NUMBER(value) = self.current_token.token_type {
                                                self.advance(); // 숫자 스킵
                                                body.push(Statement::VariableDeclaration(var_name.clone(), Box::new(Expr::Number(value)), Box::new(Statement::Empty)));
                                            }
                                        }
                                    }
                                } else if let TokenType::PRINT = self.current_token.token_type {
                                    self.advance(); // 'print' 스킵
                                    if let TokenType::LPAREN = self.current_token.token_type {
                                        self.advance(); // 여는 괄호 스킵
                                        if let TokenType::IDENTIFIER(var_name) = &self.current_token.token_type {
                                            self.advance(); // 변수 이름 스킵
                                            if let TokenType::RPAREN = self.current_token.token_type {
                                                self.advance(); // 닫는 괄호 스킵
                                                body.push(Statement::Print(Box::new(Expr::Variable(var_name.clone()))));
                                            }
                                        }
                                    }
                                }

                                // 세미콜론 스킵
                                if self.current_token.token_type == TokenType::SEMICOLON {
                                    self.advance();
                                }
                            }

                            self.advance(); // 닫는 중괄호 스킵

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
        // 함수 구문 처리
        self.advance();

        // name을 변수로 저장하여 불변 빌림을 해제합니다.
        let name = if let TokenType::IDENTIFIER(ref name) = self.current_token.token_type {
            name.clone()
        } else {
            panic!("Expected function name after 'fun'");
        };
        self.advance(); // 가변 빌림이 발생하는 부분

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
        self.advance(); // ')' 이후로 이동

        self.expect(TokenType::LBRACE);
        let body = self.if_statement();
        self.expect(TokenType::RBRACE);

        Statement::FunctionDeclaration(name, params, Box::new(body))
    }

    fn variable(&mut self) -> Statement {
        // 변수 구문 처리
        let var_name = if let TokenType::IDENTIFIER(ref name) = self.current_token.token_type {
            name.clone()
        } else {
            panic!("Expected variable name after 'var'");
        };
        self.advance(); // 가변 빌림 수행

        self.expect(TokenType::ASSIGN);
        let value = self.expr();
        self.expect(TokenType::SEMICOLON);

        Statement::VariableDeclaration(var_name, Box::new(value), Box::new(Statement::Empty))
    }

    fn if_statement(&mut self) -> Statement {
        // if 구문 처리
        self.advance(); // `if`

        self.expect(TokenType::LPAREN); // '('
        let condition = self.expr(); // 조건 표현식
        self.expect(TokenType::RPAREN); // ')'
        self.expect(TokenType::LBRACE); // '{'

        let then_branch = self.if_statement(); // if 블록
        self.expect(TokenType::RBRACE); // '}'

        let else_branch = if self.current_token.token_type == TokenType::ELSE {
            self.advance();
            self.expect(TokenType::LBRACE); // '{'
            let else_body = self.if_statement(); // else 블록
            self.expect(TokenType::RBRACE); // '}'
            Some(Box::new(else_body))
        } else {
            None
        };

        Statement::IfStatement(Box::new(condition), Box::new(then_branch), else_branch)
    }

    fn while_statement(&mut self) -> Statement {
        // while 구문 처리
        self.advance(); // `while`

        self.expect(TokenType::LPAREN); // '('
        let condition = self.expr(); // 조건 표현식
        self.expect(TokenType::RPAREN); // ')'
        self.expect(TokenType::LBRACE); // '{'

        let body = self.if_statement(); // while 블록
        self.expect(TokenType::RBRACE); // '}'

        Statement::WhileStatement(Box::new(condition), Box::new(body))
    }

    fn import_statement(&mut self) -> Statement {
        // import 구문 처리
        let module_name = if let TokenType::STRING(ref name) = self.current_token.token_type {
            name.clone()
        } else {
            panic!("Expected a module name string");
        };
        self.advance(); // 가변 빌림 수행

        self.expect(TokenType::RPAREN); // ')'
        self.expect(TokenType::SEMICOLON); // ';'

        Statement::ImportStatement(module_name)
    }

    fn expr(&mut self) -> Expr {
        let token_type = self.current_token.token_type.clone(); // 불변 빌림 해제
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