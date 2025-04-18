use std::collections::HashSet;
use std::iter::Peekable;
use std::slice::Iter;
use regex::Regex;
use crate::error::*;
use crate::lexer::*;
use crate::parser::ast::*;
use crate::parser::format::*;

pub fn parse(tokens: &Vec<Token>) -> Option<Vec<ASTNode>> {
    let mut iter = tokens.iter().peekable();
    let mut nodes = vec![];

    while let Some(token) = iter.peek() {
        match token.token_type {
            TokenType::Fun => {
                if let Some(func) = parse_function(&mut iter) {
                    nodes.push(func);
                } else {
                    println!("❌ Failed to parse function");
                    return None;
                }
            }
            TokenType::Eof => break,
            _ => {
                println!("❌ Unexpected token at top level: {:?}", token);
                return None;
            }
        }
    }

    Some(nodes)
}

pub fn function(function_name: String, parameters: Vec<ParameterNode>, body: Vec<ASTNode>, return_type: Option<WaveType>,) -> ASTNode {
    // println!("🚨 function() called with {} body items", body.len());
    ASTNode::Function(FunctionNode {
        name: function_name,
        parameters,
        body,
        return_type,
    })
}

pub fn param(parameter: String, param_type: WaveType, initial_value: Option<Value>) -> ParameterNode {
    ParameterNode {
        name: parameter,
        param_type,
        initial_value,
    }
}

pub fn parse_parameters(tokens: &mut Peekable<Iter<Token>>) -> Vec<ParameterNode> {
    let mut params = vec![];

    loop {
        let Some(token) = tokens.peek() else {
            break;
        };

        match &token.token_type {
            TokenType::Identifier(name) => {
                let name = name.clone();
                tokens.next(); // consume identifier

                if !matches!(tokens.peek().map(|t| &t.token_type), Some(TokenType::Colon)) {
                    println!("Error: Expected ':' after parameter name '{}'", name);
                    break;
                }
                tokens.next(); // consume ':'

                let token = tokens.next();
                let param_type = match &token {
                    Some(Token { token_type, .. }) => match token_type_to_wave_type(token_type) {
                        Some(wt) => wt,
                        None => {
                            println!("Error: Unsupported or unknown type token: {:?}", token_type);
                            break;
                        }
                    },
                    None => {
                        println!("Expected type after ':' for parameter '{}'", name);
                        break;
                    }
                };

                let initial_value = if matches!(tokens.peek().map(|t| &t.token_type), Some(TokenType::Equal)) {
                    tokens.next(); // consume '='
                    match tokens.next() {
                        Some(Token { token_type: TokenType::Number(n), .. }) => Some(Value::Int(*n)),
                        Some(Token { token_type: TokenType::Float(f), .. }) => Some(Value::Float(*f)),
                        Some(Token { token_type: TokenType::String(s), .. }) => Some(Value::Text(s.clone())),
                        _ => None,
                    }
                } else {
                    None
                };

                params.push(ParameterNode {
                    name,
                    param_type,
                    initial_value,
                });

                match tokens.peek().map(|t| &t.token_type) {
                    Some(TokenType::SemiColon) => {
                        tokens.next(); // consume ';'
                        continue;
                    }
                    Some(TokenType::Rparen) => {
                        tokens.next();
                        break;
                    }
                    Some(TokenType::Comma) => {
                        println!("Error: use `;` instead of `,` to separate parameters");
                        break;
                    }
                    _ => break,
                }
            }

            TokenType::Rparen => {
                tokens.next();
                break;
            }

            _ => break,
        }
    }

    params
}

fn token_type_to_wave_type(token_type: &TokenType) -> Option<WaveType> {
    match token_type {
        TokenType::TypeInt(bits) => Some(WaveType::Int(*bits)),
        TokenType::TokenTypeInt(int_type) => match int_type {
            IntegerType::I8 => Some(WaveType::Int(8)),
            IntegerType::I16 => Some(WaveType::Int(16)),
            IntegerType::I32 => Some(WaveType::Int(32)),
            IntegerType::I64 => Some(WaveType::Int(64)),
            IntegerType::I128 => Some(WaveType::Int(128)),
            IntegerType::I256 => Some(WaveType::Int(256)),
            IntegerType::I512 => Some(WaveType::Int(512)),
            IntegerType::I1024 => Some(WaveType::Int(1024)),
            _ => panic!("Unhandled integer type: {:?}", int_type),
        },
        TokenType::TypeUint(bits) => Some(WaveType::Uint(*bits)),
        TokenType::TokenTypeUint(uint_type) => match uint_type {
            UnsignedIntegerType::U8 => Some(WaveType::Uint(8)),
            UnsignedIntegerType::U16 => Some(WaveType::Uint(16)),
            UnsignedIntegerType::U32 => Some(WaveType::Uint(32)),
            UnsignedIntegerType::U64 => Some(WaveType::Uint(64)),
            UnsignedIntegerType::U128 => Some(WaveType::Uint(128)),
            UnsignedIntegerType::U256 => Some(WaveType::Uint(256)),
            UnsignedIntegerType::U512 => Some(WaveType::Uint(512)),
            UnsignedIntegerType::U1024 => Some(WaveType::Uint(1024)),
            _ => panic!("Unhandled uint type: {:?}", uint_type),
        },
        TokenType::TokenTypeFloat(float_type) => match float_type {
            FloatType::F32 => Some(WaveType::Float(32)),
            FloatType::F64 => Some(WaveType::Float(64)),
            FloatType::F128 => Some(WaveType::Float(128)),
            FloatType::F256 => Some(WaveType::Float(256)),
            FloatType::F512 => Some(WaveType::Float(512)),
            FloatType::F1024 => Some(WaveType::Float(1024)),
            _ => panic!("Unhandled float type: {:?}", float_type),
        },
        TokenType::TypeFloat(bits) => Some(WaveType::Float(*bits)),
        TokenType::TypeBool => Some(WaveType::Bool),
        TokenType::TypeChar => Some(WaveType::Char),
        TokenType::TypeByte => Some(WaveType::Byte),
        TokenType::TypeString => Some(WaveType::String),
        TokenType::TypePointer(inner) => {
            token_type_to_wave_type(inner).map(|t| WaveType::Pointer(Box::new(t)))
        }
        TokenType::TypeArray(inner, size) => {
            token_type_to_wave_type(inner).map(|t| WaveType::Array(Box::new(t), *size))
        }
        _ => None,
    }
}

pub fn extract_body(tokens: &mut Peekable<Iter<Token>>) -> Option<Vec<ASTNode>> {
    let mut body = vec![];

    if tokens.peek()?.token_type != TokenType::Lbrace {
        println!("❌ Expected '{{' at the beginning of function body");
        return None;
    }
    tokens.next(); // consume '{'

    while let Some(token) = tokens.peek() {
        match &token.token_type {
            TokenType::Rbrace => {
                tokens.next();
                break;
            }
            TokenType::Eof => {
                println!("❌ Unexpected EOF inside function body");
                return None;
            }
            TokenType::Var => {
                tokens.next(); // consume 'var'
                body.push(parse_var(tokens)?);
            }
            TokenType::Println => {
                tokens.next(); // consume 'println'
                body.push(parse_println(tokens)?);
            }
            TokenType::Print => {
                tokens.next();
                body.push(parse_print(tokens)?);
            }
            TokenType::If => {
                tokens.next();
                body.push(parse_if(tokens)?);
            }
            TokenType::For => {
                tokens.next();
                body.push(parse_for(tokens)?);
            }
            TokenType::While => {
                tokens.next();
                body.push(parse_while(tokens)?);
            }
            TokenType::Identifier(name) => {
                let token = token.clone();
                tokens.next();

                if let Some(Token { token_type: TokenType::Lparen, .. }) = tokens.peek() {
                    let expr = parse_function_call(Some(name.clone()), tokens)?;
                    if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
                        tokens.next(); // consume ';'
                    }
                    body.push(ASTNode::Statement(StatementNode::Expression(expr)));
                } else {
                    let token_clone = Token {
                        token_type: TokenType::Identifier(name.clone()),
                        lexeme: name.clone(),
                        line: token.line,
                    };
                    body.push(parse_assignment(tokens, &token_clone)?);
                }
            }
            TokenType::Break => {
                tokens.next(); // consume 'break'
                if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
                    tokens.next(); // consume ;
                }
                body.push(ASTNode::Statement(StatementNode::Break));
            }
            TokenType::Continue => {
                tokens.next(); // consume 'break'
                if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
                    tokens.next(); // consume ;
                }
                body.push(ASTNode::Statement(StatementNode::Continue));
            }
            TokenType::Return => {
                tokens.next(); // consume 'return'

                let expr = if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
                    tokens.next(); // return;
                    None
                } else {
                    let value = parse_expression(tokens)?;
                    if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
                        tokens.next();
                    }
                    Some(value)
                };

                body.push(ASTNode::Statement(StatementNode::Return(expr)));
            }
            _ => {
                // println!("⚠️ Unexpected token inside function body: {:?}", token);
                tokens.next(); // consume and skip
            }
        }
    }

    Some(body)
}

pub fn parse_function_call(name: Option<String>, tokens: &mut Peekable<Iter<Token>>) -> Option<Expression> {
    let name = name?;

    if tokens.peek()?.token_type != TokenType::Lparen {
        println!("❌ Expected '(' after function name '{}'", name);
        return None;
    }
    tokens.next(); // consume '('

    let mut args = vec![];

    while let Some(token) = tokens.peek() {
        if token.token_type == TokenType::Rparen {
            tokens.next(); // consume ')'
            break;
        }

        let arg = parse_expression(tokens)?;
        args.push(arg);

        match tokens.peek().map(|t| &t.token_type) {
            Some(TokenType::Comma) => {
                tokens.next(); // consume ','
            }
            Some(TokenType::Rparen) => continue,
            _ => {
                println!("❌ Unexpected token in function arguments: {:?}", tokens.peek());
                return None;
            }
        }
    }

    Some(Expression::FunctionCall {
        name,
        args,
    })
}

fn parse_parentheses(tokens: &mut Peekable<Iter<Token>>) -> Vec<Token> {
    let mut param_tokens = vec![];
    let mut paren_depth = 1;

    while let Some(token) = tokens.next() {
        match token.token_type {
            TokenType::Lparen => paren_depth += 1,
            TokenType::Rparen => {
                paren_depth -= 1;
                if paren_depth == 0 {
                    break;
                }
            }
            _ => {}
        }
        param_tokens.push(token.clone());
    }
    param_tokens
}

// FUN parsing
fn parse_function(tokens: &mut Peekable<Iter<Token>>) -> Option<ASTNode> {
    tokens.next();

    let name = match tokens.next() {
        Some(Token { token_type: TokenType::Identifier(name), .. }) => name.clone(),
        _ => return None,
    };

    if tokens.peek()?.token_type != TokenType::Lparen {
        return None;
    }

    tokens.next(); // consume '('
    let parameters = parse_parameters(tokens);

    let mut param_names = HashSet::new();
    for param in &parameters {
        if !param_names.insert(param.name.clone()) {
            println!("Error: Parameter '{}' is declared multiple times", param.name);
            return None;
        }
    }

    let return_type = if let Some(Token { token_type: TokenType::Arrow, .. }) = tokens.peek() {
        tokens.next(); // consume '->'

        match tokens.next() {
            Some(Token { token_type, .. }) => {
                token_type_to_wave_type(token_type)
            }
            None => {
                println!("Error: Expected type after '->'");
                None
            }
        }
    } else {
        None
    };

    let body = extract_body(tokens);
    Some(function(name, parameters, body?, return_type))
}

// VAR parsing
fn parse_var(tokens: &mut Peekable<Iter<'_, Token>>) -> Option<ASTNode> {
    let name = match tokens.next() {
        Some(Token { token_type: TokenType::Identifier(name), .. }) => name.clone(),
        _ => {
            println!("Expected identifier");
            return None;
        }
    };

    if !matches!(tokens.next().map(|t| &t.token_type), Some(TokenType::Colon)) {
        println!("Expected ':' after identifier");
        return None;
    }

    let type_token = match tokens.next() {
        Some(token) => token.clone(),
        _ => {
            println!("Expected type after ':'");
            return None;
        }
    };

    let wave_type = match token_type_to_wave_type(&type_token.token_type) {
        Some(t) => t,
        None => {
            println!("Unknown or unsupported type: {}", type_token.lexeme);
            return None;
        }
    };

    let initial_value = if let Some(Token { token_type: TokenType::Equal, .. }) = tokens.peek() {
        tokens.next();
        match tokens.next() {
            Some(Token { token_type: TokenType::Number(value), .. }) => Some(Literal::Number(*value)),
            Some(Token { token_type: TokenType::Float(value), .. }) => Some(Literal::Float(*value)),
            Some(Token { token_type: TokenType::String(value), .. }) => Some(Literal::String(value.clone())),
            _ => None,
        }
    } else {
        None
    };

    if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
        tokens.next(); // Consume ';'
    }

    Some(ASTNode::Variable(VariableNode {
        name,
        type_name: wave_type,
        initial_value,
    }))
}

// PRINTLN parsing
fn parse_println(tokens: &mut Peekable<Iter<Token>>) -> Option<ASTNode> {
    if tokens.peek()?.token_type != TokenType::Lparen {
        println!("Error: Expected '(' after 'println'");
        return None;
    }
    tokens.next(); // Consume '('

    let content = if let Some(Token { token_type: TokenType::String(content), .. }) = tokens.next() {
        content.clone()
    } else {
        println!("Error: Expected string literal in 'println'");
        return None;
    };

    let placeholder_count = Regex::new(r"\{[^}]*\}")
        .unwrap()
        .find_iter(&content)
        .count();

    if placeholder_count == 0 {
        if tokens.peek()?.token_type != TokenType::Rparen {
            println!("Error: Expected closing ')'");
            return None;
        }
        tokens.next(); // Consume ')'

        return Some(ASTNode::Statement(StatementNode::Println(
            format!("{}\n", content),
        )));
    }

    let mut args = Vec::new();
    while let Some(Token { token_type: TokenType::Comma, .. }) = tokens.peek() {
        tokens.next(); // Consume ','
        if let Some(expr) = parse_expression(tokens) {
            args.push(expr);
        } else {
            println!("Error: Failed to parse expression in 'println'");
            return None;
        }
    }

    if tokens.peek()?.token_type != TokenType::Rparen {
        println!("Error: Expected closing ')'");
        return None;
    }
    tokens.next(); // Consume ')'

    if placeholder_count != args.len() {
        println!(
            "Error: Expected {} arguments, found {}",
            placeholder_count,
            args.len()
        );
        return None;
    }

    Some(ASTNode::Statement(StatementNode::PrintlnFormat {
        format: format!("{}\n", content),
        args,
    }))
}

// PRINT parsing
fn parse_print(tokens: &mut Peekable<Iter<Token>>) -> Option<ASTNode> {
    if tokens.peek()?.token_type != TokenType::Lparen {
        println!("Error: Expected '(' after 'println'");
        return None;
    }
    tokens.next(); // Consume '('

    let content = if let Some(Token { token_type: TokenType::String(content), .. }) = tokens.next() {
        content.clone() // Need clone() because it is String
    } else {
        println!("Error: Expected string literal in 'println'");
        return None;
    };

    let placeholder_count = Regex::new(r"\{[^}]*\}")
        .unwrap()
        .find_iter(&content)
        .count();

    if placeholder_count == 0 {
        // No format → Print just a string
        if tokens.peek()?.token_type != TokenType::Rparen {
            println!("Error: Expected closing ')'");
            return None;
        }
        tokens.next(); // Consume ')'

        return Some(ASTNode::Statement(StatementNode::Print(
            format!("{}", content),
        )));
    }

    let mut args = Vec::new();
    while let Some(Token { token_type: TokenType::Comma, .. }) = tokens.peek() {
        tokens.next(); // Consume ','
        if let Some(expr) = parse_expression(tokens) {
            args.push(expr);
        } else {
            println!("Error: Failed to parse expression in 'println'");
            return None;
        }
    }
    tokens.next();

    if tokens.peek()?.token_type != TokenType::Rparen {
        println!("Error: Expected closing ')'");
        return None;
    }
    tokens.next(); // Consume ')'

    if placeholder_count != args.len() {
        println!(
            "Error: Expected {} arguments, found {}",
            placeholder_count,
            args.len()
        );
        return None;
    }

    Some(ASTNode::Statement(StatementNode::PrintFormat {
        format: content,
        args,
    }))
}

fn skip_whitespace(tokens: &mut Peekable<Iter<Token>>) {
    while let Some(token) = tokens.peek() {
        if token.token_type == TokenType::Whitespace {
            tokens.next();
        } else {
            break;
        }
    }
}

// IF parsing
fn parse_if(tokens: &mut Peekable<Iter<Token>>) -> Option<ASTNode> {
    if tokens.peek()?.token_type != TokenType::Lparen {
        println!("Error: Expected '(' after 'if'");
        return None;
    }
    tokens.next(); // Consume '('

    let condition = match parse_expression(tokens) {
        Some(expr) => {
            expr
        }
        None => {
            return None;
        }
    };

    if tokens.peek()?.token_type != TokenType::Rparen {
        println!("Error: Expected ')' after 'if' condition");
        return None;
    }
    tokens.next(); // Consume ')'

    // Expect '{' after condition
    if tokens.peek()?.token_type != TokenType::Lbrace {
        println!("Error: Expected '{{' after 'if' condition");
        return None;
    }
    tokens.next(); // Consume '{'

    let body = parse_block(tokens)?;

    let mut else_if_blocks: Vec<ASTNode> = Vec::new();
    let mut else_block = None;

    while let Some(token) = tokens.peek() {
        if token.token_type != TokenType::Else {
            break;
        }
        tokens.next(); // Consume 'else'

        // Check if it comes right after else
        if let Some(Token { token_type: TokenType::If, .. }) = tokens.peek() {
            tokens.next();
            let parsed = parse_if(tokens);

            match parsed {
                Some(ASTNode::Statement(stmt @ StatementNode::If { .. })) => {
                    else_if_blocks.push(ASTNode::Statement(stmt));
                }
                Some(other) => {
                    return None;
                }
                None => {
                    return None;
                }
            }

            continue;
        }

        // Handle 'else' case
        if tokens.peek()?.token_type != TokenType::Lbrace {
            println!("Error: Expected '{{' after 'else'");
            return None;
        }
        tokens.next(); // Consume '{'
        else_block = Some(Box::new(parse_block(tokens)?));
        break;
    }

    let result = ASTNode::Statement(StatementNode::If {
        condition,
        body,
        else_if_blocks: if else_if_blocks.is_empty() {
            None
        } else {
            Some(Box::new(else_if_blocks))
        },
        else_block,
    });

    Some(result)
}

// FOR parsing
fn parse_for(tokens: &mut Peekable<Iter<Token>>) -> Option<ASTNode> {
    /*
    // Check 'for' keyword and see if there is '()
    if tokens.peek()?.token_type != TokenType::Lparen {
        println!("Error: Expected '(' after 'if'");
        return None;
    }
    tokens.next(); // '(' Consumption

    // Conditional parsing (where condition must be made ASTNode)
    let initialization = parse_expression(tokens)?; // Parsing conditions with expressions
    let condition = parse_expression(tokens)?;
    let increment = parse_expression(tokens)?;
    let body = parse_expression(tokens)?;

    if tokens.peek()?.token_type != TokenType::Rparen {
        println!("Error: Expected ')' after condition");
        return None;
    }
    tokens.next(); // ')' Consumption

    Some(ASTNode::Statement(StatementNode::For {
        initialization,
        condition,
        increment,
        body,
    }))
     */
    None
}

// WHILE parsing
fn parse_while(tokens: &mut Peekable<Iter<Token>>) -> Option<ASTNode> {
    if tokens.peek()?.token_type != TokenType::Lparen {
        println!("Error: Expected '(' after 'while'");
        return None;
    }
    tokens.next(); // Consume '('

    let condition = parse_expression(tokens)?;

    if tokens.peek()?.token_type != TokenType::Rparen {
        println!("Error: Expected ')' after 'while' condition");
        return None;
    }
    tokens.next(); // Consume ')'

    if tokens.peek()?.token_type != TokenType::Lbrace {
        println!("Error: Expected '{{' after 'while'");
        return None;
    }
    tokens.next(); // Consume '{'

    let body = parse_block(tokens)?;

    Some(ASTNode::Statement(StatementNode::While { condition, body }))
}

fn parse_assignment(tokens: &mut Peekable<Iter<Token>>, first_token: &Token) -> Option<ASTNode> {
    let var_name = match &first_token.token_type {
        TokenType::Identifier(name) => name.clone(),
        _ => return None,
    };

    if let Some(Token { token_type: TokenType::Equal, .. }) = tokens.peek() {
        tokens.next(); // consume '='

        let value = parse_expression(tokens)?;

        if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
            tokens.next(); // consume ';'
        }

        return Some(ASTNode::Statement(StatementNode::Assign {
            variable: var_name,
            value,
        }));
    }

    None
}

// block parsing
fn parse_block(tokens: &mut Peekable<Iter<Token>>) -> Option<Vec<ASTNode>> {
    let mut body = vec![];

    while let Some(token) = tokens.next() {
        if token.token_type == TokenType::Rbrace {
            break;
        }

        let node = match token.token_type {
            TokenType::Var => parse_var(tokens),
            TokenType::Println => parse_println(tokens),
            TokenType::Print => parse_print(tokens),
            TokenType::If => parse_if(tokens),
            TokenType::For => parse_for(tokens),
            TokenType::While => parse_while(tokens),
            TokenType::Identifier(_) => parse_assignment(tokens, token),
            TokenType::Break => {
                if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
                    tokens.next();
                }
                Some(ASTNode::Statement(StatementNode::Break))
            }
            TokenType::Continue => {
                if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
                    tokens.next();
                }
                Some(ASTNode::Statement(StatementNode::Continue))
            }
            TokenType::Return => {
                let expr = if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
                    tokens.next(); // consume ;
                    None
                } else {
                    let value = parse_expression(tokens)?;
                    if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
                        tokens.next(); // consume ;
                    }
                    Some(value)
                };
                Some(ASTNode::Statement(StatementNode::Return(expr)))
            }
            _ => None
        };

        if let Some(ast_node) = node {
            body.push(ast_node);
        }
    }

    Some(body)
}

pub fn parse_type(type_str: &str) -> Option<TokenType> {
    if type_str.starts_with('i') {
        let bits = type_str[1..].parse::<u16>().ok()?;
        Some(TokenType::TypeInt(bits))
    } else if type_str.starts_with('u') {
        let bits = type_str[1..].parse::<u16>().ok()?;
        Some(TokenType::TypeUint(bits))
    } else if type_str.starts_with('f') {
        let bits = type_str[1..].parse::<u16>().ok()?;
        Some(TokenType::TypeFloat(bits))
    } else if type_str == "bool" {
        Some(TokenType::TypeBool)
    } else if type_str == "char" {
        Some(TokenType::TypeChar)
    } else if type_str == "byte" {
        Some(TokenType::TypeByte)
    } else if type_str == "str" {
        Some(TokenType::TypeString)
    } else if type_str.starts_with("ptr<") && type_str.ends_with('>') {
        let inner_type_str = &type_str[4..type_str.len() - 1];
        let inner_type = parse_type(inner_type_str)?;
        Some(TokenType::TypePointer(Box::new(inner_type)))
    } else if type_str.starts_with("array<") && type_str.ends_with('>') {
        let parts: Vec<&str> = type_str[6..type_str.len() - 1].split(',').collect();
        if parts.len() != 2 {
            return None;
        }
        let inner_type = parse_type(parts[0].trim())?;
        let size = parts[1].trim().parse::<u32>().ok()?;
        Some(TokenType::TypeArray(Box::new(inner_type), size))
    } else {
        None
    }
}

fn validate_type(expected: &TokenType, actual: &TokenType) -> bool {
    match (expected, actual) {
        (TokenType::TypeInt(_), TokenType::TypeInt(_)) => true,
        (TokenType::TypeUint(_), TokenType::TypeUint(_)) => true,
        (TokenType::TypeFloat(_), TokenType::TypeFloat(_)) => true,
        (TokenType::TypeBool, TokenType::TypeBool) => true,
        (TokenType::TypeChar, TokenType::TypeChar) => true,
        (TokenType::TypeByte, TokenType::TypeByte) => true,
        (TokenType::TypePointer(inner1), TokenType::TypePointer(inner2)) => {
            validate_type(&**inner1, &**inner2) // Double dereference to get TokenType
        }
        (TokenType::TypeArray(inner1, size1), TokenType::TypeArray(inner2, size2)) => {
            validate_type(&**inner1, &**inner2) && size1 == size2 // Double dereference to get TokenType
        }
        (TokenType::TypeString, TokenType::TypeString) => true,
        _ => false,
    }
}