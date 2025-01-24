use std::iter::Peekable;
use std::slice::Iter;
use crate::lexer::*;
use crate::parser::ast::*;

pub fn function(function_name: String, parameters: Vec<ParameterNode>, body: Vec<ASTNode>) -> ASTNode {
    ASTNode::Function(FunctionNode {
        name: function_name,
        parameters, // No parameters
        body,       // Empty body
    })
}

pub fn param(parameter: String, param_type: String, initial_value: Option<String>) -> ParameterNode {
    ParameterNode {
        name: parameter,
        param_type,
        initial_value,
    }
}

pub fn extract_parameters(tokens: &Vec<Token>) -> Vec<ParameterNode> {
    let mut params = vec![];
    let mut i = 0;

    while i < tokens.len() {
        if matches!(tokens[i].token_type, TokenType::VAR) {
            // parameter name
            let name = if let Some(TokenType::IDENTIFIER(name)) = tokens.get(i + 1).map(|t| &t.token_type) {
                name.clone()
            } else {
                continue; // Skip if no name exists
            };

            // Type parsing
            let param_type = if let Some(TokenType::COLON) = tokens.get(i + 2).map(|t| &t.token_type) {
                tokens[i + 3].lexeme.clone()
            } else {
                "unknown".to_string()
            };

            // Initial value parsing
            let initial_value = if let Some(TokenType::EQUAL) = tokens.get(i + 4).map(|t| &t.token_type) {
                Some(tokens[i + 5].lexeme.clone())
            } else {
                None
            };

            // Add parameters to the list
            params.push(ParameterNode { name, param_type, initial_value });
            i += 6; // After processing the parameters, move to the next token
        } else {
            i += 1; // If it's not VAR, move on
        }
    }

    params
}

pub fn extract_body<'a>(tokens: &mut Peekable<Iter<'a, Token>>) -> Vec<ASTNode> {
    let mut body = vec![];

    while let Some(token) = tokens.next() {
        match &token.token_type {
            TokenType::EOF => break,
            TokenType::VAR => {
                if let Some(ast_node) = parse_var(tokens) {
                    body.push(ast_node);
                }
            }
            TokenType::PRINTLN => {
                if let Some(ast_node) = parse_println(tokens) {
                    body.push(ast_node);
                }
            }
            TokenType::PRINT => {
                if let Some(ast_node) = parse_print(tokens) {
                    body.push(ast_node);
                }
            }
            TokenType::IF => {
                if let Some(ast_node) = parse_if(tokens) {
                    body.push(ast_node);
                }
            }
            TokenType::FOR => {
                if let Some(ast_node) = parse_for(tokens) {
                    body.push(ast_node);
                }
            }
            TokenType::WHILE => {
                if let Some(ast_node) = parse_while(tokens) {
                    body.push(ast_node);
                }
            }
            _ => {
                // Ignore unprocessed tokens
            }
        }
    }

    body
}

// VAR parsing
fn parse_var(tokens: &mut Peekable<Iter<'_, Token>>) -> Option<ASTNode> {
    if let Some(Token { token_type: TokenType::IDENTIFIER(name), .. }) = tokens.next() {
        if let Some(Token { token_type: TokenType::COLON, .. }) = tokens.next() {
            if let Some(Token { token_type, .. }) = tokens.next() {
                let type_name = match token_type {
                    TokenType::TypeInt(size) => match size {
                        IntegerType::I4 => "i4".to_string(),
                        IntegerType::I8 => "i8".to_string(),
                        IntegerType::I16 => "i16".to_string(),
                        IntegerType::I32 => "i32".to_string(),
                        IntegerType::I64 => "i64".to_string(),
                        IntegerType::I128 => "i128".to_string(),
                        IntegerType::I256 => "i256".to_string(),
                        IntegerType::I512 => "i512".to_string(),
                        IntegerType::I1024 => "i1024".to_string(),
                        IntegerType::I2048 => "i2048".to_string(),
                        IntegerType::I4096 => "i4096".to_string(),
                        IntegerType::I8192 => "i8192".to_string(),
                        IntegerType::I16384 => "i16384".to_string(),
                        IntegerType::I32768 => "i32768".to_string(),
                        IntegerType::ISZ => "isz".to_string(),
                        _ => return None,
                    },
                    TokenType::TypeUint(size) => match size {
                        UnsignedIntegerType::U4 => "u4".to_string(),
                        UnsignedIntegerType::U8 => "u8".to_string(),
                        UnsignedIntegerType::U16 => "u16".to_string(),
                        UnsignedIntegerType::U32 => "u32".to_string(),
                        UnsignedIntegerType::U64 => "u64".to_string(),
                        UnsignedIntegerType::U128 => "u128".to_string(),
                        UnsignedIntegerType::U128 => "u128".to_string(),
                        UnsignedIntegerType::U256 => "u256".to_string(),
                        UnsignedIntegerType::U512 => "u512".to_string(),
                        UnsignedIntegerType::U1024 => "u1024".to_string(),
                        UnsignedIntegerType::U2048 => "u2048".to_string(),
                        UnsignedIntegerType::U4096 => "u4096".to_string(),
                        UnsignedIntegerType::U8192 => "u8192".to_string(),
                        UnsignedIntegerType::U16384 => "u16384".to_string(),
                        UnsignedIntegerType::U32768 => "u32768".to_string(),
                        UnsignedIntegerType::USZ => "usz".to_string(),
                        _ => return None,
                    },
                    TokenType::TypeFloat(size) => match size {
                        FloatType::F32 => "f32".to_string(),
                        FloatType::F64 => "f64".to_string(),
                        FloatType::F128 => "f128".to_string(),
                        FloatType::F128 => "f128".to_string(),
                        FloatType::F256 => "f256".to_string(),
                        FloatType::F512 => "f512".to_string(),
                        FloatType::F1024 => "f1024".to_string(),
                        FloatType::F2048 => "f2048".to_string(),
                        FloatType::F4096 => "f4096".to_string(),
                        FloatType::F8192 => "f8192".to_string(),
                        FloatType::F16384 => "f16384".to_string(),
                        FloatType::F32768 => "f32768".to_string(),
                        _ => return None,
                    },
                    _ => return None,
                };

                if let Some(Token { token_type: TokenType::EQUAL, .. }) = tokens.next() {
                    if let Some(Token { token_type: TokenType::NUMBER(value), .. }) = tokens.next() {
                        return Some(ASTNode::Variable(VariableNode {
                            name: name.clone(),
                            type_name,
                            initial_value: Some(value.to_string()),
                        }));
                    }

                    if let Some(Token { token_type: TokenType::FLOAT(value), .. }) = tokens.next() {
                        return Some(ASTNode::Variable(VariableNode {
                            name: name.clone(),
                            type_name,
                            initial_value: Some(value.to_string()),
                        }));
                    }

                    if let Some(Token { token_type: TokenType::STRING(value), .. }) = tokens.next() {
                        return Some(ASTNode::Variable(VariableNode {
                            name: name.clone(),
                            type_name,
                            initial_value: Some(value.parse().unwrap()),
                        }));
                    }
                }
            }
        }
    }
    None
}

// PRINTLN parsing
fn parse_println(tokens: &mut Peekable<Iter<Token>>) -> Option<ASTNode> {
    if let Some(Token { token_type: TokenType::LPAREN, .. }) = tokens.next() {
        if let Some(Token { token_type: TokenType::STRING(ref content), .. }) = tokens.next() {
            if let Some(Token { token_type: TokenType::RPAREN, .. }) = tokens.next() {
                return Some(ASTNode::Statement(StatementNode::Println(content.clone())));
            }
        }
    }
    None
}

// PRINT parsing
fn parse_print(tokens: &mut Peekable<Iter<Token>>) -> Option<ASTNode> {
    if let Some(Token { token_type: TokenType::LPAREN, .. }) = tokens.next() {
        if let Some(Token { token_type: TokenType::STRING(ref content), .. }) = tokens.next() {
            if let Some(Token { token_type: TokenType::RPAREN, .. }) = tokens.next() {
                return Some(ASTNode::Statement(StatementNode::Print(content.clone())));
            }
        }
    }
    None
}

// IF parsing
fn parse_if(tokens: &mut Peekable<Iter<Token>>) -> Option<ASTNode> {
    if let Some(Token { token_type: TokenType::LPAREN, .. }) = tokens.next() {
        // Condition extraction (simple handling)
        let condition = if let Some(Token { lexeme, .. }) = tokens.next() {
            lexeme.clone()
        } else {
            return None;
        };

        if let Some(Token { token_type: TokenType::RPAREN, .. }) = tokens.next() {
            let body = parse_block(tokens)?;
            return Some(ASTNode::Statement(StatementNode::If { condition, body }));
        }
    }
    None
}

// FOR parsing
fn parse_for(tokens: &mut Peekable<Iter<Token>>) -> Option<ASTNode> {
    if let Some(Token { token_type: TokenType::LPAREN, .. }) = tokens.next() {
        let iterator = if let Some(Token { lexeme, .. }) = tokens.next() {
            lexeme.clone()
        } else {
            return None;
        };

        if let Some(Token { token_type: TokenType::RPAREN, .. }) = tokens.next() {
            let body = parse_block(tokens)?;
            return Some(ASTNode::Statement(StatementNode::For { iterator, body }));
        }
    }
    None
}

// WHILE parsing
fn parse_while(tokens: &mut Peekable<Iter<Token>>) -> Option<ASTNode> {
    if let Some(Token { token_type: TokenType::LPAREN, .. }) = tokens.next() {
        let condition = if let Some(Token { lexeme, .. }) = tokens.next() {
            lexeme.clone()
        } else {
            return None;
        };

        if let Some(Token { token_type: TokenType::RPAREN, .. }) = tokens.next() {
            let body = parse_block(tokens)?;
            return Some(ASTNode::Statement(StatementNode::While { condition, body }));
        }
    }
    None
}

// block parsing
fn parse_block(tokens: &mut Peekable<Iter<Token>>) -> Option<Vec<ASTNode>> {
    if let Some(Token { token_type: TokenType::LBRACE, .. }) = tokens.next() {
        let mut body = vec![];

        while let Some(token) = tokens.peek() {
            if let TokenType::RBRACE = token.token_type {
                tokens.next(); // } consumption
                break;
            }

            body.extend(extract_body(tokens)); // The part that I modified here
        }

        return Some(body);
    }
    None
}


/*
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
        eprintln!("Start parsing function...");

        if self.current_token.token_type != TokenType::FUN {
            panic!("Expected 'fun', but got {:?}", self.current_token);
        }
        self.advance(); // Consume 'fun'

        let name = if let TokenType::IDENTIFIER(name) = &self.current_token.token_type {
            name.clone()
        } else {
            panic!("Expected function name, but got {:?}", self.current_token);
        };
        eprintln!("Function name parsed: {}", name);
        self.advance(); // Consume function name

        if self.current_token.token_type != TokenType::LPAREN {
            panic!("Expected '(' after function name, but got {:?}", self.current_token);
        }
        self.advance(); // Consume '('

        // Parse parameters
        let mut params = Vec::new();
        if self.current_token.token_type != TokenType::RPAREN {
            while self.current_token.token_type != TokenType::RPAREN {
                if let TokenType::IDENTIFIER(param) = &self.current_token.token_type {
                    params.push(param.clone()); // Collect parameter names
                } else {
                    panic!("Expected parameter name, but got {:?}", self.current_token);
                }
                self.advance(); // Consume parameter name
                if self.current_token.token_type == TokenType::COMMA {
                    self.advance(); // Skip comma, if there is another parameter
                }
            }
        }
        eprintln!("Function parameters parsed: {:?}", params);
        self.advance(); // Consume ')'

        if self.current_token.token_type != TokenType::LBRACE {
            panic!("Expected '{{' to start function body, but got {:?}", self.current_token);
        }
        self.advance(); // Consume '{'

        // Parse function body
        let mut body = Vec::new();
        while self.current_token.token_type != TokenType::RBRACE {
            eprintln!("Parsing statement in function body: {:?}", self.current_token);
            /*
            match self.current_token.token_type {
                TokenType::PRINTLN => {
                    self.advance(); // 'println' Consumption

                    // Check LPAREN
                    if self.current_token.token_type != TokenType::LPAREN {
                        panic!("Expected '(' after 'println', but got {:?}", self.current_token);
                    }
                    self.advance(); // '(' Consumption

                    // Check STRING
                    let message = if let TokenType::STRING(literal) = &self.current_token.token_type {
                        literal.clone() // Copy String Value
                    } else {
                        panic!("Expected string literal, but got {:?}", self.current_token);
                    };
                    self.advance(); // String literal consumption

                    // Check RPAREN
                    if self.current_token.token_type != TokenType::RPAREN {
                        panic!("Expected ')' after string literal, but got {:?}", self.current_token);
                    }
                    self.advance(); // ')' Consumption

                    // Check SEMICOLON
                    if self.current_token.token_type != TokenType::SEMICOLON {
                        panic!("Expected ';' after 'println' statement, but got {:?}", self.current_token);
                    }
                    self.advance(); // ';' Consumption

                    // Add Print Node to AST
                    body.push(ASTNode::Print {
                        message,
                        newline: true,
                    });
                }
                _ => {
                    eprintln!("Unknown token in function body: {:?}", self.current_token);
                    self.advance();
                }
            }
             */
        }
        self.advance(); // Consume '}'

        eprintln!("Function body parsed: {:?}", body);

        // Create the Function ASTNode with the parsed name, params, and body
        let node = ASTNode::Function {
            name,
            params,
            body,
        };
        eprintln!("Adding function node to AST: {:?}", node);
        ast.add_node(node);
    }


    fn variable(&mut self, ast: &mut AST) {
        // Processing 'var' tokens
        if self.current_token.token_type != TokenType::VAR {
            eprintln!("Error: Expected 'var', but got {:?}", self.current_token.token_type);
            return
        }
        self.advance(); // `var`

        let is_immutable = if self.current_token.token_type == TokenType::IMM {
            self.advance(); // 'imm'
            true
        } else {
            false
        };

        // Processing variable names
        let var_name = if let TokenType::IDENTIFIER(var_name) = &self.current_token.token_type {
            var_name.clone()
        } else {
            eprintln!("Error: Expected variable name after 'var', but got {:?}", self.current_token.token_type);
            return;
        };
        self.advance();

        // Processing variable types
        let var_type = if self.current_token.token_type == TokenType::COLON {
            self.advance();
            match &self.current_token.token_type {
                TokenType::TypeInt(IntegerType::I4) => "i4".to_string(),
                TokenType::TypeInt(IntegerType::I8) => "i8".to_string(),
                TokenType::TypeInt(IntegerType::I16) => "i16".to_string(),
                TokenType::TypeInt(IntegerType::I32) => "i32".to_string(),
                TokenType::TypeInt(IntegerType::I64) => "i64".to_string(),
                TokenType::TypeInt(IntegerType::I128) => "i128".to_string(),
                TokenType::TypeInt(IntegerType::I256) => "i256".to_string(),
                TokenType::TypeInt(IntegerType::I512) => "i512".to_string(),
                TokenType::TypeInt(IntegerType::I1024) => "i1024".to_string(),
                TokenType::TypeInt(IntegerType::I2048) => "i2048".to_string(),
                TokenType::TypeInt(IntegerType::I4096) => "i4096".to_string(),
                TokenType::TypeInt(IntegerType::I8192) => "i8192".to_string(),
                TokenType::TypeInt(IntegerType::I16384) => "i16384".to_string(),
                TokenType::TypeInt(IntegerType::I32768) => "i32768".to_string(),
                TokenType::TypeInt(IntegerType::ISZ) => "isz".to_string(),

                TokenType::TypeInt(IntegerType::U4) => "u4".to_string(),
                TokenType::TypeInt(IntegerType::U8) => "u8".to_string(),
                TokenType::TypeInt(IntegerType::U16) => "u16".to_string(),
                TokenType::TypeInt(IntegerType::U32) => "u32".to_string(),
                TokenType::TypeInt(IntegerType::U64) => "u64".to_string(),
                TokenType::TypeInt(IntegerType::U8) => "u128".to_string(),
                TokenType::TypeInt(IntegerType::U256) => "u256".to_string(),
                TokenType::TypeInt(IntegerType::U512) => "u512".to_string(),
                TokenType::TypeInt(IntegerType::U1024) => "u1024".to_string(),
                TokenType::TypeInt(IntegerType::U2048) => "u2048".to_string(),
                TokenType::TypeInt(IntegerType::U4096) => "u4096".to_string(),
                TokenType::TypeInt(IntegerType::U8192) => "u8192".to_string(),
                TokenType::TypeInt(IntegerType::U16384) => "u16384".to_string(),
                TokenType::TypeInt(IntegerType::U32768) => "u32768".to_string(),
                TokenType::TypeInt(IntegerType::USZ) => "usz".to_string(),

                TokenType::TypeFloat(FloatType::F32) => "f32".to_string(),
                TokenType::TypeFloat(FloatType::F64) => "f64".to_string(),
                TokenType::TypeFloat(FloatType::F128) => "f128".to_string(),
                TokenType::TypeFloat(FloatType::F256) => "f256".to_string(),
                TokenType::TypeFloat(FloatType::F512) => "f512".to_string(),
                TokenType::TypeFloat(FloatType::F1024) => "f1024".to_string(),
                TokenType::TypeFloat(FloatType::F2048) => "f2048".to_string(),
                TokenType::TypeFloat(FloatType::F4096) => "f4096".to_string(),
                TokenType::TypeFloat(FloatType::F8192) => "f8192".to_string(),
                TokenType::TypeFloat(FloatType::F16384) => "f16384".to_string(),
                TokenType::TypeFloat(FloatType::F32768) => "f32768".to_string(),
                TokenType::TypeString => "string".to_string(),
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

        // Processing variable values
        if self.current_token.token_type != TokenType::EQUAL {
            panic!("Expected '=' after variable type");
        }
        self.advance();

        let value = match &self.current_token.token_type {
            TokenType::NUMBER(value) => Value::Int(*value as i64), // Enclosing the i64 value with Value::Int
            TokenType::STRING(value) => Value::Text(value.clone()), // Envelopes String value with Value::Text
            _ => {
                eprintln!("Error: Expected a balue for the variable, but got {:?}", self.current_token.token_type);
                return;
            }
        };
        self.advance();

        // Check semicolon
        if self.current_token.token_type != TokenType::SEMICOLON {
            eprintln!("Error: Expected ';' at the end of variable declaration, but got {:?}", self.current_token.token_type);
            return;
        }
        self.advance();

        eprintln!("Adding variable node: {:?} of type {:?} with calue {:?}", var_name, var_type, value);

        // Add a Node to AST
        ast.add_node(ASTNode::Variable {
            name: var_name,
            var_type,
            value,
            is_immutable,
        });
    }


    fn print_statement(&mut self, ast: &mut AST) {
        if self.current_token.token_type != TokenType::PRINTLN {
            panic!("Expected 'println' keyword to start function, but got {:?}", self.current_token);
        }
        self.advance(); // println

        if self.current_token.token_type != TokenType::LPAREN {
            panic!("Expected 'fun' keyword to start function, but got {:?}", self.current_token);
        }
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
    }

    fn if_statement(&mut self, ast: &mut AST) {
        // if syntax processing
        if self.current_token.token_type != TokenType::IF {
            panic!("Expected 'if'");
        }
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
    }

    fn while_statement(&mut self, ast: &mut AST) {
        // while syntax processing
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
        // Syntax 'for'
        self.advance(); // 'for'

        if self.current_token.token_type != TokenType::LPAREN {
            panic!("Expected '(' after 'for'");
        }
        self.advance(); // '('

        // Initialization Processing
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

        // conditioning
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

        // an increase/decrease process
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
        // import parsing
        if self.current_token.token_type != TokenType::IMPORT {
            panic!("Expected 'import'");
        }
        self.advance(); // `import`

        if self.current_token.token_type != TokenType::LPAREN {
            panic!("Expected '(' after 'import'");
        }
        self.advance();

        if let TokenType::STRING(module_name) = &self.current_token.token_type {
            println!("Importing module: {}", module_name); // Module Name Output
            self.advance();
        } else {
            panic!("Expected a module name string after '('");
        }

        // Check ')' token
        if self.current_token.token_type != TokenType::RPAREN {
            panic!("Expected ')' after module name");
        }
        self.advance(); // Skip ')'

        // a ';' check that means the end of a sentence
        if self.current_token.token_type != TokenType::SEMICOLON {
            panic!("Expected ';' at the end of import statement");
        }
        self.advance(); // `;` Skip

        ast.add_node(ASTNode::Import {
            module_name: "".to_string()
        });
    }
}

 */