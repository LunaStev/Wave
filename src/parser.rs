use crate::lexer::{FloatType, IntegerType, Lexer, Token, TokenType};
use crate::ast::{AST, ASTNode, Value};

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

    pub fn parse(&mut self) -> AST {
        let mut ast = AST::new();

        while self.current_token.token_type != TokenType::EOF {
            match self.current_token.token_type {
                TokenType::FUN => self.function(&mut ast),
                TokenType::VAR => self.variable(&mut ast),
                TokenType::IF => self.if_statement(),
                TokenType::WHILE => self.while_statement(),
                TokenType::FOR => self.for_statement(),
                TokenType::IMPORT => self.import_statement(),
                TokenType::PRINT | TokenType::PRINTLN => self.print_statement(&mut ast),
                _ => self.advance(),
            }
        }
        ast
    }

    fn advance(&mut self) {
        self.current_token = self.lexer.next_token();
    }

    fn function(&mut self, ast: &mut AST) {
        // 함수 구문 처리
        self.advance(); // `fun`

        let name = if let TokenType::IDENTIFIER(name) = &self.current_token.token_type {
            name.clone()
        } else {
            panic!("Expected function name after 'fun'");
        };
        self.advance();

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

        let mut body = Vec::new();
        // 함수 본문 처리: 중괄호 안에서 명령문들을 처리
        while self.current_token.token_type != TokenType::RBRACE {
            match self.current_token.token_type {
                TokenType::VAR => {
                    // 변수 선언 처리 (예시)
                    self.advance(); // `var`
                    let var_name = if let TokenType::IDENTIFIER(name) = &self.current_token.token_type {
                        name.clone()
                    } else {
                        panic!("Expected variable name after 'var'");
                    };
                    self.advance();

                    // 변수 타입 처리
                    if self.current_token.token_type != TokenType::COLON {
                        panic!("Expected ':' after variable name");
                    }
                    self.advance();

                    let var_type = if let TokenType::TYPE_INT(t) = &self.current_token.token_type {
                        t.to_string() // IntegerType에 Display 구현 후 to_string 사용
                    } else {
                        panic!("Expected type after ':'");
                    };
                    self.advance();

                    // 변수 값 처리
                    if self.current_token.token_type != TokenType::EQUAL {
                        panic!("Expected '=' after variable type");
                    }
                    self.advance();

                    let value = if let TokenType::NUMBER(val) = &self.current_token.token_type {
                        Value::Int(*val) // `i64` 값을 직접 사용
                    } else {
                        panic!("Expected number after '='");
                    };
                    self.advance();

                    // 변수 선언 AST 노드 추가
                    body.push(ASTNode::Variable {
                        name: var_name,
                        var_type,
                        value,
                    });
                }
                TokenType::PRINTLN => {
                    // println 처리 (예시)
                    self.advance(); // `println`
                    self.advance(); // '('

                    let message = if let TokenType::STRING(msg) = &self.current_token.token_type {
                        msg.clone()
                    } else {
                        panic!("Expected string in println");
                    };
                    self.advance();

                    // 변수 또는 값 처리
                    let expr = if let TokenType::IDENTIFIER(expr) = &self.current_token.token_type {
                        expr.clone()
                    } else {
                        panic!("Expected expression after println string");
                    };
                    self.advance();

                    // `println` 처리
                    body.push(ASTNode::Print {
                        message,
                        newline: true,
                    });
                }
                _ => {
                    // 기타 처리해야 할 문장들 추가 (예: 표현식, 조건문 등)
                    self.advance(); // 단순히 advance()로 진행할 수도 있습니다
                }
            }
        }
        self.advance(); // `RBRACE`

        let function_node = ASTNode::Function {
            name,
            params,
            body,
        };
        ast.add_node(function_node);
    }

    fn variable(&mut self, ast: &mut AST) {
        // `var` 토큰 처리
        self.advance(); // `var`

        // 변수 이름 처리
        let var_name = if let TokenType::IDENTIFIER(var_name) = &self.current_token.token_type {
            var_name.clone()
        } else {
            panic!("Expected variable name after 'var'");
        };
        self.advance();

        // 변수 타입 처리
        let var_type = if self.current_token.token_type == TokenType::COLON {
            self.advance();
            match &self.current_token.token_type {
                TokenType::TYPE_INT(IntegerType::I4) => "i4".to_string(),
                TokenType::TYPE_INT(IntegerType::I8) => "i8".to_string(),
                TokenType::TYPE_INT(IntegerType::I16) => "i16".to_string(),
                TokenType::TYPE_INT(IntegerType::I32) => "i32".to_string(),
                TokenType::TYPE_INT(IntegerType::I64) => "i64".to_string(),
                TokenType::TYPE_INT(IntegerType::I128) => "i128".to_string(),
                TokenType::TYPE_INT(IntegerType::I256) => "i256".to_string(),
                TokenType::TYPE_INT(IntegerType::I512) => "i512".to_string(),
                TokenType::TYPE_INT(IntegerType::I1024) => "i1024".to_string(),
                TokenType::TYPE_INT(IntegerType::I2048) => "i2048".to_string(),
                TokenType::TYPE_INT(IntegerType::I4096) => "i4096".to_string(),
                TokenType::TYPE_INT(IntegerType::I8192) => "i8192".to_string(),
                TokenType::TYPE_INT(IntegerType::I16384) => "i16384".to_string(),
                TokenType::TYPE_INT(IntegerType::I32768) => "i32768".to_string(),
                TokenType::TYPE_INT(IntegerType::ISZ) => "isz".to_string(),

                TokenType::TYPE_INT(IntegerType::U4) => "u4".to_string(),
                TokenType::TYPE_INT(IntegerType::U8) => "u8".to_string(),
                TokenType::TYPE_INT(IntegerType::U16) => "u16".to_string(),
                TokenType::TYPE_INT(IntegerType::U32) => "u32".to_string(),
                TokenType::TYPE_INT(IntegerType::U64) => "u64".to_string(),
                TokenType::TYPE_INT(IntegerType::U8) => "u128".to_string(),
                TokenType::TYPE_INT(IntegerType::U256) => "u256".to_string(),
                TokenType::TYPE_INT(IntegerType::U512) => "u512".to_string(),
                TokenType::TYPE_INT(IntegerType::U1024) => "u1024".to_string(),
                TokenType::TYPE_INT(IntegerType::U2048) => "u2048".to_string(),
                TokenType::TYPE_INT(IntegerType::U4096) => "u4096".to_string(),
                TokenType::TYPE_INT(IntegerType::U8192) => "u8192".to_string(),
                TokenType::TYPE_INT(IntegerType::U16384) => "u16384".to_string(),
                TokenType::TYPE_INT(IntegerType::U32768) => "u32768".to_string(),
                TokenType::TYPE_INT(IntegerType::USZ) => "usz".to_string(),

                TokenType::TYPE_FLOAT(FloatType::F32) => "f32".to_string(),
                TokenType::TYPE_FLOAT(FloatType::F64) => "f64".to_string(),
                TokenType::TYPE_FLOAT(FloatType::F128) => "f128".to_string(),
                TokenType::TYPE_FLOAT(FloatType::F256) => "f256".to_string(),
                TokenType::TYPE_FLOAT(FloatType::F512) => "f512".to_string(),
                TokenType::TYPE_FLOAT(FloatType::F1024) => "f1024".to_string(),
                TokenType::TYPE_FLOAT(FloatType::F2048) => "f2048".to_string(),
                TokenType::TYPE_FLOAT(FloatType::F4096) => "f4096".to_string(),
                TokenType::TYPE_FLOAT(FloatType::F8192) => "f8192".to_string(),
                TokenType::TYPE_FLOAT(FloatType::F16384) => "f16384".to_string(),
                TokenType::TYPE_FLOAT(FloatType::F32768) => "f32768".to_string(),
                TokenType::TYPE_STRING => "string".to_string(),
                _ => panic!("Expected a valid type after ':'"),
            }
        } else {
            panic!("Expected ':' after variable name");
        };
        self.advance();

        // 변수 값 처리
        if self.current_token.token_type != TokenType::EQUAL {
            panic!("Expected '=' after variable type");
        }
        self.advance();

        let value = match &self.current_token.token_type {
            TokenType::NUMBER(value) => Value::Int(*value), // i64 값을 Value::Int로 감싸줌
            TokenType::STRING(value) => Value::Text(value.clone()), // String 값을 Value::Text로 감싸줌
            _ => panic!("Expected a valid initial value after '='"),
        };
        self.advance();

        // 세미콜론 확인
        if self.current_token.token_type != TokenType::SEMICOLON {
            panic!("Expected ';' at the end of variable declaration");
        }
        self.advance();

        // AST에 노드 추가
        ast.add_node(ASTNode::Variable {
            name: var_name,
            var_type,
            value,
        });
    }


    fn print_statement(&mut self, ast: &mut AST) {
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

        ast.add_node(ASTNode::Print {
            message: "".to_string(),
            newline: false,
        })
    }

    fn if_statement(&mut self, ast: &mut AST) {
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
        self.advance();

        ast.add_node(ASTNode::IfStatement {
            condition: "".to_string(),
            body: vec![],
            else_body: None,
        })
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
        // `for` 구문 처리
        self.advance(); // 'for'

        if self.current_token.token_type != TokenType::LPAREN {
            panic!("Expected '(' after 'for'");
        }
        self.advance(); // '('

        // 초기화 처리
        if let TokenType::IDENTIFIER(var_name) = &self.current_token.token_type {
            println!("Initializing variable: {}", var_name);
            self.advance();

            if self.current_token.token_type != TokenType::EQUAL {
                panic!("Expected '=' in initialization");
            }
            self.advance();

            if let TokenType::NUMBER(value) = &self.current_token.token_type {
                println!("Initial value: {}", value);
                self.advance();
            } else {
                panic!("Expected a numeric value in initialization");
            }

            if self.current_token.token_type != TokenType::SEMICOLON {
                panic!("Expected ';' after initialization");
            }
            self.advance(); // ';'
        } else {
            panic!("Expected variable initialization in 'for' loop");
        }

        // 조건 처리
        if let TokenType::NUMBER(value) = &self.current_token.token_type {
            println!("Condition value: {}", value);
            self.advance();
        } else {
            panic!("Expected a condition in 'for' loop");
        }

        if self.current_token.token_type != TokenType::SEMICOLON {
            panic!("Expected ';' after condition");
        }
        self.advance(); // ';'

        // 증감 처리
        if let TokenType::IDENTIFIER(var_name) = &self.current_token.token_type {
            println!("Incrementing variable: {}", var_name);
            self.advance();

            if self.current_token.token_type != TokenType::INCREMENT
                && self.current_token.token_type != TokenType::DECREMENT {
                panic!("Expected '++' or '--' for increment/decrement");
            }
            println!("Operation: {:?}", self.current_token.token_type);
            self.advance();
        } else {
            panic!("Expected increment/decrement in 'for' loop");
        }

        if self.current_token.token_type != TokenType::RPAREN {
            panic!("Expected ')' after 'for' header");
        }
        self.advance(); // ')'

        if self.current_token.token_type != TokenType::LBRACE {
            panic!("Expected LBRACE at the beginning of 'for' body");
        }
        self.advance(); // '{'

        while self.current_token.token_type != TokenType::RBRACE {
            self.advance();
        }
        self.advance(); // '}'
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
