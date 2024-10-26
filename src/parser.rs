use std::string::ParseError;
use crate::lexer::{Lexer, Token, TokenType};

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current_token: Token,
}

#[derive(Debug)]
pub enum AstType {
    Function { name: String, body: Vec<AstType> },
    Variable { name: String, value: i32 },
    Print { expr: String },
    If { condition: Box<AstType>, then_branch: Vec<AstType>, else_branch: Option<Vec<AstType>> },
    While { condition: Box<AstType>, body: Vec<AstType> },
    Import { module_name: String },
}

#[derive(Debug)]
pub enum ParseErrorType {
    UnexpectedToken(String),
    EndOfInput,
} // 실제 에러 타입을 구체적으로 정의할 수 있음

impl<'a> Parser<'a> {
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        let current_token = lexer.next_token();
        Parser { lexer, current_token }
    }

    pub fn parse(&mut self) -> Result<AstType, ParseErrorType> {
        let mut statements = Vec::new();

        while self.current_token.token_type != TokenType::EOF {
            match self.current_token.token_type {
                TokenType::FUN => statements.push(self.function()?),
                TokenType::VAR => statements.push(self.variable()?),
                TokenType::IF => statements.push(self.if_statement()?),
                TokenType::WHILE => statements.push(self.while_statement()?),
                TokenType::IMPORT => statements.push(self.import_statement()?),
                _ => self.advance(),
            }
        }
        Ok(AstType::Function { name: "main".to_string(),body: statements })
    }

    fn advance(&mut self) {
        self.current_token = self.lexer.next_token();
    }

    fn function(&mut self) -> Result<AstType, ParseErrorType> {
        // 함수 구문 처리
        self.advance(); // `fun`

        let name = if let TokenType::IDENTIFIER(name) = &self.current_token.token_type {
            println!("Parsing function: {}", name);
            self.advance();
            name.clone()
        } else {
            panic!("Expected function name after 'fun'");
        };

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

        Ok(AstType::Function { name: name.clone(), body })
    }

    fn variable(&mut self) -> Result<AstType, ParseErrorType> {
        // 변수 구문 처리
        self.advance(); // `var`

        let name = if let TokenType::IDENTIFIER(var_name) = &self.current_token.token_type {
            println!("Parsing variable: {}", var_name);
            self.advance();
            name.clone()
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

        Ok(AstType::Function { name: name.clone(), body })
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