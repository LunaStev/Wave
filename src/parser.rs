use crate::ast::ASTNode;
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

    pub fn parse(&mut self) -> Vec<ASTNode> {
        let mut nodes = Vec::new();

        while self.current_token.token_type != TokenType::EOF {
            let node = match self.current_token.token_type {
                TokenType::FUN => self.function(),
                TokenType::VAR => self.variable(),
                TokenType::IF => self.if_statement(),
                TokenType::WHILE => self.while_statement(),
                TokenType::FOR => self.for_statement(),
                TokenType::IMPORT => self.import_statement(),
                TokenType::PRINT | TokenType::PRINTLN => self.print_statement(),
                _ => {
                    self.advance();
                    continue;
                }
            };
            nodes.push(node);
        }

        nodes
    }

    fn advance(&mut self) {
        self.current_token = self.lexer.next_token();
    }

    fn function(&mut self) -> ASTNode {
        self.advance(); // `fun`

        let name = if let TokenType::IDENTIFIER(name) = &self.current_token.token_type {
            let name = name.clone();  // 먼저 불변 참조로 값 추출
            self.advance();  // advance() 호출은 그 후에
            name
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
        self.advance(); // `)`

        if self.current_token.token_type != TokenType::LBRACE {
            panic!("Expected LBRACE at the beginning of function body");
        }
        self.advance();

        let mut body = Vec::new();
        while self.current_token.token_type != TokenType::RBRACE {
            body.push(self.parse_statement());
        }
        self.advance(); // `}`

        ASTNode::Function {
            name,
            params,
            body,
        }
    }

    fn variable(&mut self) -> ASTNode {
        self.advance(); // `var`

        let name = if let TokenType::IDENTIFIER(var_name) = &self.current_token.token_type {
            let var_name = var_name.clone();  // 먼저 불변 참조로 값을 추출
            self.advance();  // advance() 호출은 그 후에
            var_name
        } else {
            panic!("Expected variable name");
        };

        if self.current_token.token_type == TokenType::COLON {
            self.advance();

            let var_type = match &self.current_token.token_type {
                TokenType::TYPE_INT(int_type) => format!("{:?}", int_type),
                TokenType::TYPE_FLOAT(float_type) => format!("{:?}", float_type),
                TokenType::TYPE_STRING => "String".to_string(),
                _ => panic!("Expected a valid type after ':'"),
            };
            self.advance();

            if self.current_token.token_type != TokenType::EQUAL {
                panic!("Expected '=' after variable name");
            }
            self.advance();

            let value = match &self.current_token.token_type {
                TokenType::NUMBER(value) => value.to_string(),
                TokenType::STRING(value) => value.clone(),
                _ => panic!("Expected a numeric or string initial value after '='"),
            };
            self.advance();

            if self.current_token.token_type != TokenType::SEMICOLON {
                panic!("Expected ';' at the end of variable declaration");
            }
            self.advance();

            ASTNode::Variable {
                name,
                var_type,
                value,
            }
        } else {
            panic!("Expected ':' after variable name");
        }
    }

    fn print_statement(&mut self) -> ASTNode {
        let is_println = matches!(self.current_token.token_type, TokenType::PRINTLN);
        self.advance(); // Move past `print` or `println`

        let value = match &self.current_token.token_type {
            TokenType::STRING(content) => {
                let content = content.clone();
                self.advance(); // Move past the string literal
                ASTNode::Literal {
                    value: content,
                    is_println,
                }
            },
            TokenType::NUMBER(num) => {
                let num = num.to_string();
                self.advance(); // Move past the number literal
                ASTNode::Literal {
                    value: num,
                    is_println,
                }
            },
            _ => panic!("Expected a literal (string or number) after print/println"),
        };

        if self.current_token.token_type != TokenType::SEMICOLON {
            panic!("Expected ';' at the end of print/println statement");
        }
        self.advance(); // Move past the semicolon

        // Return the Print node that uses the value from the Literal
        ASTNode::Print {
            message: match value {
                ASTNode::Literal { value, .. } => value,
                _ => unreachable!(), // This should never happen
            },
        }
    }


    fn if_statement(&mut self) -> ASTNode {
        self.advance(); // `if`

        if self.current_token.token_type != TokenType::LPAREN {
            panic!("Expected '(' after 'if'");
        }
        self.advance();

        // 여기서 조건 노드 파싱 로직 필요 (예: 변수 또는 표현식)
        let condition = self.parse_expression();

        if self.current_token.token_type != TokenType::RPAREN {
            panic!("Expected ')' after condition");
        }
        self.advance();

        if self.current_token.token_type != TokenType::LBRACE {
            panic!("Expected LBRACE at the beginning of if body");
        }
        self.advance();

        let mut body = Vec::new();
        while self.current_token.token_type != TokenType::RBRACE {
            body.push(self.parse_statement());
        }
        self.advance(); // `}`

        let else_body = if self.current_token.token_type == TokenType::ELSE {
            self.advance();
            if self.current_token.token_type != TokenType::LBRACE {
                panic!("Expected LBRACE at the beginning of else body");
            }
            self.advance();

            let mut else_body = Vec::new();
            while self.current_token.token_type != TokenType::RBRACE {
                else_body.push(self.parse_statement());
            }
            self.advance(); // `}`
            Some(else_body)
        } else {
            None
        };

        ASTNode::If {
            condition: Box::new(condition),
            body,
            else_body,
        }
    }

    fn while_statement(&mut self) -> ASTNode {
        self.advance(); // `while`

        if self.current_token.token_type != TokenType::LPAREN {
            panic!("Expected '(' after 'while'");
        }
        self.advance();

        let condition = self.parse_expression();

        if self.current_token.token_type != TokenType::RPAREN {
            panic!("Expected ')' after condition");
        }
        self.advance();

        if self.current_token.token_type != TokenType::LBRACE {
            panic!("Expected LBRACE at the beginning of while body");
        }
        self.advance();

        let mut body = Vec::new();
        while self.current_token.token_type != TokenType::RBRACE {
            body.push(self.parse_statement());
        }
        self.advance(); // `}`

        ASTNode::While {
            condition: Box::new(condition),
            body,
        }
    }

    fn for_statement(&mut self) -> ASTNode {
        self.advance(); // `for`

        if self.current_token.token_type != TokenType::LPAREN {
            panic!("Expected '(' after 'for'");
        }
        self.advance();

        // `for` 구문 파싱 로직 추가 필요 (초기화, 조건, 증감식 등)

        if self.current_token.token_type != TokenType::RPAREN {
            panic!("Expected ')' after for conditions");
        }
        self.advance();

        if self.current_token.token_type != TokenType::LBRACE {
            panic!("Expected LBRACE at the beginning of for body");
        }
        self.advance();

        let mut body = Vec::new();
        while self.current_token.token_type != TokenType::RBRACE {
            body.push(self.parse_statement());
        }
        self.advance(); // `}`

        ASTNode::For {
            init: None,
            condition: None,
            increment: None,
            body,
        }
    }

    fn import_statement(&mut self) -> ASTNode {
        self.advance(); // `import`

        if self.current_token.token_type != TokenType::LPAREN {
            panic!("Expected '(' after 'import'");
        }
        self.advance();

        let module_name = if let TokenType::STRING(name) = &self.current_token.token_type {
            name.clone()
        } else {
            panic!("Expected module name string after '('");
        };
        self.advance(); // 모듈명 건너뜀

        if self.current_token.token_type != TokenType::RPAREN {
            panic!("Expected ')' after module name");
        }
        self.advance(); // `)` 건너뜀

        if self.current_token.token_type != TokenType::SEMICOLON {
            panic!("Expected ';' at the end of import statement");
        }
        self.advance(); // `;` 건너뜀

        ASTNode::Import {
            module_name,
        }
    }

    pub fn parse_expression(&mut self) -> ASTNode {
        match self.current_token.token_type {
            TokenType::NUMBER(num) => {
                self.advance(); // 숫자 파싱 후 다음 토큰으로 이동
                ASTNode::Number(num)
            }
            TokenType::STRING(ref content) => {
                let content_clone = content.clone();

                self.advance();

                ASTNode::String(content_clone)
            }
            TokenType::VAR => {
                let var_name = self.current_token.lexeme.to_string();
                self.advance(); // 변수 이름 후 이동
                ASTNode::Variable {
                    name: var_name,
                    var_type: "unknown".to_string(),
                    value: "undefined".to_string(),
                }
            }
            _ => panic!("Unexpected token while parsing expression"),
        }
    }

    pub fn parse_statement(&mut self) -> ASTNode {
        match self.current_token.token_type {
            TokenType::PRINT | TokenType::PRINTLN => self.print_statement(),
            TokenType::IF => self.if_statement(),
            TokenType::WHILE => self.while_statement(),
            TokenType::FOR => self.for_statement(),
            TokenType::VAR => self.variable(),
            TokenType::FUN => self.function(),
            TokenType::IMPORT => self.import_statement(),
            _ => panic!("Unexpected token in statement parsing"),
        }
    }
}