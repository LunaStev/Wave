use crate::lexer::{FloatType, IntegerType, Lexer, Token, TokenType};
use crate::ast::{AST, ASTNode, Value};

#[derive(Debug)]
pub struct Parser<'a> {
    pub(crate) lexer: Lexer<'a>,
    pub(crate) current_token: Token,
}

impl<'a> Parser<'a> {
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        let current_token = lexer.next_token();
        Parser { lexer, current_token }
    }

    pub fn parse(&mut self) -> AST {
        let mut ast = AST::new();

        while self.current_token.token_type != TokenType::EOF {
            eprintln!("Current Token: {:?}", self.current_token); // Adding Debug Messages
            match self.current_token.token_type {
                TokenType::FUN => {
                    eprintln!("Parsing function..."); // Adding Debug Messages
                    self.function(&mut ast)
                },
                TokenType::VAR => {
                    eprintln!("Parsing variable..."); // Adding Debug Messages
                    self.variable(&mut ast)
                },
                TokenType::IF => {
                    eprintln!("Parsing if statement..."); // Adding Debug Messages
                    self.if_statement(&mut ast)
                },
                TokenType::WHILE => {
                    eprintln!("Parsing while statement..."); // Adding Debug Messages
                    self.while_statement(&mut ast)
                },
                TokenType::FOR => {
                    eprintln!("Parsing for statement..."); // Adding Debug Messages
                    self.for_statement()
                },
                TokenType::IMPORT => {
                    eprintln!("Parsing import statement..."); // Adding Debug Messages
                    self.import_statement(&mut ast)
                },
                TokenType::PRINT | TokenType::PRINTLN => {
                    eprintln!("Parsing print statement..."); // Adding Debug Messages
                    self.print_statement(&mut ast)
                },
                _ => {
                    eprintln!("Unknown token: {:?}", self.current_token.token_type); // Adding Debug Messages
                    self.advance()
                },
            }
        }

        if ast.nodes.is_empty() {
            eprintln!("Warning: The AST is empty. No nodes were parsed.");
        }

        ast
    }

    fn advance(&mut self) {
        self.current_token = self.lexer.next_token();
    }

    fn function(&mut self, ast: &mut AST) {
        eprintln!("Start parsing function...");
        self.advance(); // `fun`

        let name = if let TokenType::IDENTIFIER(name) = &self.current_token.token_type {
            name.clone()
        } else {
            panic!("Expected function name after 'fun', but got {:?}", self.current_token);
        };
        eprintln!("Function name: {}", name);
        self.advance();

        if self.current_token.token_type != TokenType::LPAREN {
            panic!("Expected '(' after function name");
        }
        self.advance();

        let mut params = Vec::new();
        while self.current_token.token_type != TokenType::RPAREN {
            eprintln!("Function param: {:?}", self.current_token);
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
        self.advance(); // ')'

        if self.current_token.token_type != TokenType::LBRACE {
            panic!("Expected 'LBRACE' at the beginning of function body");
        }
        self.advance();

        let mut body = Vec::new();
        // 함수 본문 처리: 중괄호 안에서 명령문들을 처리
        while self.current_token.token_type != TokenType::RBRACE {
            eprintln!("Parsing statement in function body: {:?}", self.current_token);
            match self.current_token.token_type {
                TokenType::VAR => self.variable(ast),
                TokenType::PRINTLN => self.print_statement(ast),
                _ => self.advance(),
            }
        }
        self.advance(); // `RBRACE`

        ast.add_node(ASTNode::Function {
            name,
            params,
            body,
        });

        ast;
    }

    fn variable(&mut self, ast: &mut AST) {
        // `var` 토큰 처리
        self.advance(); // `var`

        // 변수 이름 처리
        let var_name = if let TokenType::IDENTIFIER(var_name) = &self.current_token.token_type {
            var_name.clone()
        } else {
            eprintln!("Error: Expected variable name after 'var', but got {:?}", self.current_token.token_type);
            return;
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
                _ => {
                    eprintln!("Error: Expected variable type after ':', but got {:?}", self.current_token.token_type);
                    return;
                }
            }
        } else {
            eprintln!("Error: Expected variable type after ':', but got {:?}", self.current_token.token_type);
            return;
        };
        self.advance();

        // 변수 값 처리
        if self.current_token.token_type != TokenType::EQUAL {
            panic!("Expected '=' after variable type");
        }
        self.advance();

        let value = match &self.current_token.token_type {
            TokenType::NUMBER(value) => Value::Int(*value as i64), // i64 값을 Value::Int로 감싸줌
            TokenType::STRING(value) => Value::Text(value.clone()), // String 값을 Value::Text로 감싸줌
            _ => {
                eprintln!("Error: Expected a balue for the variable, but got {:?}", self.current_token.token_type);
                return;
            }
        };
        self.advance();

        // 세미콜론 확인
        if self.current_token.token_type != TokenType::SEMICOLON {
            eprintln!("Error: Expected ';' at the end of variable declaration, but got {:?}", self.current_token.token_type);
            return;
        }
        self.advance();

        eprintln!("Adding variable node: {:?} of type {:?} with calue {:?}", var_name, var_type, value);

        // AST에 노드 추가
        ast.add_node(ASTNode::Variable {
            name: var_name,
            var_type,
            value,
        });
    }


    fn print_statement(&mut self, ast: &mut AST) {
        self.advance(); // println
        self.advance(); // '('

        let message = if let TokenType::STRING(msg) = &self.current_token.token_type {
            msg.clone()
        } else {
            eprintln!("Error: Expected string after 'println', but got {:?}", self.current_token.token_type);
            return;
        };
        self.advance(); // advance to after the message

        if self.current_token.token_type != TokenType::RPAREN {
            eprintln!("Error: Expected ')' after the string, but got {:?}", self.current_token.token_type);
            return;
        }
        self.advance(); // skip ')'

        eprintln!("Adding print node with message: {:?}", message);

        ast.add_node(ASTNode::Print {
            message,
            newline: true,
        });

        ast;
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
        });

        ast;
    }

    fn while_statement(&mut self, ast: &mut AST) {
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

        ast.add_node(ASTNode::WhileLoop {
            condition: "".to_string(),
            body: vec![]
        });
    }

    fn for_statement(&mut self /*, ast: &mut AST */) {
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

        /*
        ast.add_node(ASTNode::ForLoop {
            init: Box::new(()),
            condition: "".to_string(),
            increment: Box::new(()),
            body: vec![],
        })
        */
    }

    fn import_statement(&mut self, ast: &mut AST) {
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

        ast.add_node(ASTNode::Import {
            module_name: "".to_string()
        });

        ast;
    }
}
