use crate::lexer::{Lexer, Token, TokenType};

#[derive(Debug)]
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current_token: Token,
}

impl<'a> Parser<'a> {
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        let current_token = lexer.next_token();
        Parser { lexer, current_token }
    }

    pub fn parse(&mut self) {
        while self.current_token.token_type != TokenType::EOF {
            match self.current_token.token_type {
                TokenType::FUN => self.function(),
                TokenType::VAR => self.variable(),
                TokenType::IF => self.if_statement(),
                TokenType::WHILE => self.while_statement(),
                TokenType::FOR => self.for_statement(),
                TokenType::IMPORT => self.import_statement(),
                TokenType::PRINT | TokenType::PRINTLN => self.print_statement(),
                _ => self.advance(),
            }
        }
    }

    fn advance(&mut self) {
        self.current_token = self.lexer.next_token();
    }

    fn function(&mut self) {
        // 함수 구문 처리
        self.advance(); // `fun`

        if let TokenType::IDENTIFIER(name) = &self.current_token.token_type {
            println!("Parsing function: {}", name);
            self.advance();
        } else {
            panic!("Expected function name after 'fun'");
        }

        if self.current_token.token_type != TokenType::LPAREN {
            panic!("Expected '(' after function name");
        }
        self.advance();

        let mut params = Vec::new();
        while self.current_token.token_type != TokenType::RPAREN {
            if let TokenType::IDENTIFIER(param_name) = &self.current_token.token_type {
                params.push(param_name.clone());
                self.advance();

                if self.current_token.token_type == TokenType::COMMA {
                    self.advance();
                }
            } else {
                panic!("Expected parameter name in function parameter list");
            }
        }
        self.advance();

        if self.current_token.token_type != TokenType::LBRACE {
            panic!("Expected 'LBRACE' at the beginning of function body");
        }
        self.advance();

        while self.current_token.token_type != TokenType::RBRACE {
            self.advance();
        }
        self.advance();

    }

    fn variable(&mut self) {
        // 변수 구문 처리
        self.advance(); // `var`

        if let TokenType::IDENTIFIER(var_name) = &self.current_token.token_type {
            println!("Parsing variable: {}", var_name);
            self.advance();
        } else {
            panic!("Expected variable name after 'var'");
        };

        if self.current_token.token_type != TokenType::ASSIGN {
            panic!("Expected '=' after variable name");
        }
        self.advance();

        match &self.current_token.token_type {
            TokenType::NUMBER(value) => {
                println!("Initial value: {}", value);
                self.advance();
            },
            _ => panic!("Expected a numeric inital value after '='"),
        }

        if self.current_token.token_type != TokenType::SEMICOLON {
            panic!("Expected ';' at the end of variable declaration");
        }
        self.advance();

    }

    fn print_statement(&mut self) {
        self.advance();
        if let TokenType::STRING(_) = &self.current_token.token_type {
            // It's a string, proceed with extracting the string.
        } else {
            panic!("Expected a string literal after print/println");
        }
        let message = match &self.current_token.token_type {
            TokenType::STRING(s) => s,
            _ => panic!("Expected a string"),
        };

        if self.current_token.token_type == TokenType::PRINTLN {
            println!("{}", message);
        } else {
            print!("{}", message);
        }
        self.advance();
    }

    fn if_statement(&mut self) {
        // if 구문 처리
        self.advance(); // `if`

        if self.current_token.token_type != TokenType::LPAREN {
            panic!("Expected '(' after 'if'");
        }
        self.advance();

        if let TokenType::NUMBER(value) = &self.current_token.token_type {
            println!("Condition value: {}", value);
            self.advance();
        } else {
            panic!("Expected a condition after '('");
        }

        if self.current_token.token_type != TokenType::RPAREN {
            panic!("Expected ')' after condition");
        }
        self.advance();

        if self.current_token.token_type != TokenType::LBRACE {
            panic!("Expected 'LBRACE' at the beginning of if body");
        }
        self.advance();

        while self.current_token.token_type != TokenType::RBRACE {
            self.advance();
        }
        self.advance();

        if self.current_token.token_type == TokenType::ELSE {
            self.advance();

            if self.current_token.token_type != TokenType::LBRACE {
                panic!("Expected 'LBRACE' at the beginning of else body");
            }
            self.advance();

            while self.current_token.token_type != TokenType::RBRACE {
                self.advance();
            }
            self.advance();
        }
    }

    fn while_statement(&mut self) {
        // while 구문 처리
        self.advance(); // `while`

        if self.current_token.token_type != TokenType::LPAREN {
            panic!("Expected '(' after 'while'");
        }
        self.advance();

        if let TokenType::NUMBER(value) = &self.current_token.token_type {
            println!("Condition value: {}", value);
            self.advance();
        } else {
            panic!("Expected a conditon after '('");
        }

        if self.current_token.token_type != TokenType::RPAREN {
            panic!("Expected ')' after condition");
        }
        self.advance();

        if self.current_token.token_type != TokenType::LBRACE {
            panic!("Expected 'LBRACE' at the beginning of while body");
        }
        self.advance();

        while self.current_token.token_type != TokenType::RBRACE {
            self.advance();
        }
        self.advance();
    }

    fn for_statement(&mut self) {
        // for 구문 처리
        self.advance(); // 'for'

        if self.current_token.token_type != TokenType::LPAREN {
            panic!("Expected '(' after 'for'");
        }


        self.advance();
    }

    fn import_statement(&mut self) {
        // import 구문 처리
        self.advance(); // `import`

        if self.current_token.token_type != TokenType::LPAREN {
            panic!("Expected '(' after 'import'");
        }
        self.advance();

        if let TokenType::STRING(module_name) = &self.current_token.token_type {
            println!("Importing module: {}", module_name); // 모듈 이름 출력
            self.advance();
        } else {
            panic!("Expected a module name string after '('");
        }

        // ')' 토큰 체크
        if self.current_token.token_type != TokenType::RPAREN {
            panic!("Expected ')' after module name");
        }
        self.advance(); // ')' 건너뜀

        // 문장의 끝을 의미하는 `;` 체크
        if self.current_token.token_type != TokenType::SEMICOLON {
            panic!("Expected ';' at the end of import statement");
        }
        self.advance(); // `;` 건너뜀
    }
}