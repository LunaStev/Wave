use std::collections::HashSet;
use std::iter::Peekable;
use std::slice::Iter;
use crate::lexer::*;
use crate::parser::ast::*;
use crate::parser::format::*;

pub fn parse(tokens: &[Token]) -> Option<ASTNode> {
    let mut tokens_iter = tokens.iter().peekable();
    parse_function(&mut tokens_iter)
}

pub fn function(function_name: String, parameters: Vec<ParameterNode>, body: Vec<ASTNode>) -> ASTNode {
    ASTNode::Function(FunctionNode {
        name: function_name,
        parameters, // No parameters
        body,       // Empty body
    })
}

pub fn param(parameter: String, param_type: String, initial_value: Option<Value>) -> ParameterNode {
    ParameterNode {
        name: parameter,
        param_type,
        initial_value,
    }
}

pub fn extract_parameters(tokens: &[Token], start: usize, end: usize) -> Vec<ParameterNode> {
    let mut params = vec![];
    let mut i = start;

    while i < end {
        if let TokenType::Var = tokens[i].token_type {
            // println!("Found 'var', stopping parameter parsing.");
            break;
        }

        let name = match &tokens[i].token_type {
            TokenType::Identifier(name) => name.clone(),
            _ => {
                i += 1;
                continue;
            }
        };
        i += 1;

        if i >= end || !matches!(tokens[i].token_type, TokenType::Colon) {
            continue;
        }
        i += 1;

        let param_type = match &tokens[i].token_type {
            TokenType::TypeInt(_) => tokens[i].lexeme.clone(),
            _ => "unknown".into(),
        };
        i += 1;

        let initial_value = if i < end && matches!(tokens[i].token_type, TokenType::Equal) {
            i += 1;
            if i < end {
                match &tokens[i].token_type {
                    TokenType::Float(value) => Some(Value::Float(*value)),
                    TokenType::Number(value) => Some(Value::Int(*value)),
                    _ => None,
                }
            } else {
                None
            }
        } else {
            None
        };

        while i < end && !matches!(tokens[i].token_type, TokenType::SemiColon) {
            i += 1;
        }
        if i < end {
            i += 1;
        }

        params.push(ParameterNode {
            name,
            param_type,
            initial_value,
        });
    }
    params
}

pub fn extract_body(tokens: &mut Peekable<Iter<Token>>) -> Vec<ASTNode> {
    let mut body = vec![];

    while let Some(token) = tokens.next() {
        match &token.token_type {
            TokenType::Eof => break,
            TokenType::Var => {
                if let Some(ast_node) = parse_var(tokens) {
                    body.push(ast_node);
                }
            }
            TokenType::Println => {
                if let Some(ast_node) = parse_println(tokens) {
                    body.push(ast_node);
                }
            }
            TokenType::Print => {
                if let Some(ast_node) = parse_print(tokens) {
                    body.push(ast_node);
                }
            }
            TokenType::If => {
                if let Some(ast_node) = parse_if(tokens) {
                    body.push(ast_node);
                }
            }
            TokenType::For => {
                if let Some(ast_node) = parse_for(tokens) {
                    body.push(ast_node);
                }
            }
            TokenType::While => {
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

    if !matches!(tokens.next().map(|t| &t.token_type), Some(TokenType::Lparen)) {
        return None;
    }

    let param_tokens = parse_parentheses(tokens);
    let parameters = extract_parameters(&param_tokens, 0, param_tokens.len());

    let param_names: HashSet<String> = parameters.iter().map(|p| p.name.clone()).collect();
    for param in &parameters {
        if param_names.contains(&param.name) {
            println!("Error: Parameter '{}' is declared multiple times", param.name);
            return None;
        }
    }

    if !matches!(tokens.next().map(|t| &t.token_type), Some(TokenType::Lbrace)) {
        return None;
    }

    let body = extract_body(tokens);
    Some(function(name, parameters, body))
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

    let type_name = match tokens.next() {
        Some(Token { lexeme, .. }) => lexeme.clone(),
        _ => {
            println!("Expected type after ':'");
            return None;
        }
    };

    let initial_value = if let Some(Token { token_type: TokenType::Equal, .. }) = tokens.peek() {
        tokens.next();
        match tokens.next() {
            Some(Token { token_type: TokenType::Float(value), .. }) => Some(Literal::Number(*value)),
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
        type_name,
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
        format!("{}\n", content) // Need clone() because it is String
    } else {
        println!("Error: Expected string literal in 'println'");
        return None;
    };

    let placeholder_count = content.matches("{}").count();

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
    tokens.next();

    Some(ASTNode::Statement(StatementNode::Println {
        format: content,
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

    let placeholder_count = content.matches("{}").count();

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
    tokens.next();

    Some(ASTNode::Statement(StatementNode::Print {
        format: content,
        args,
    }))
}

// IF parsing
fn parse_if(tokens: &mut Peekable<Iter<Token>>) -> Option<ASTNode> {
    println!("Parsing if statement");

    // Expect '(' after 'if'
    if let Some(Token { token_type: TokenType::Lparen, .. }) = tokens.next() {
        println!("Found '('");

        // Parse the condition
        let condition = parse_expression(tokens)?;
        println!("Parsed condition: {:?}", condition);

        // Expect ')' after condition
        match tokens.next() {
            Some(Token { token_type: TokenType::Rparen, .. }) => {
                println!("Found ')'");
            },
            _ => {
                println!("Error: Expected closing ')'");
                return None;
            }
        }

        // Parse the body
        let body = parse_block(tokens)?;
        println!("Parsed body: {:?}", body);

        let mut else_if_blocks = Vec::new();
        let mut else_block = None;

        // Parse else-if and else blocks
        while let Some(token) = tokens.peek() {
            if token.token_type != TokenType::Else {
                break;
            }
            tokens.next(); // Consume 'else'

            if let Some(Token { token_type: TokenType::If, .. }) = tokens.peek() {
                tokens.next(); // Consume 'if'

                // Expect '(' after 'else if'
                if let Some(Token { token_type: TokenType::Lparen, .. }) = tokens.next() {
                    let condition = parse_expression(tokens)?;

                    // Expect ')' after else-if condition
                    match tokens.next() {
                        Some(Token { token_type: TokenType::Rparen, .. }) => {},
                        _ => {
                            println!("Error: Expected closing ')' after else-if condition");
                            return None;
                        }
                    }

                    let body = parse_block(tokens)?;
                    else_if_blocks.push((condition, body));
                } else {
                    println!("Error: Expected '(' after 'else if'");
                    return None;
                }
            } else {
                else_block = Some(parse_block(tokens)?);
                break;
            }
        }

        Some(ASTNode::Statement(StatementNode::If {
            condition,
            body,
            else_if_blocks,
            else_block,
        }))
    } else {
        println!("Error: Expected '(' after 'if'");
        None
    }
}

// FOR parsing
fn parse_for(tokens: &mut Peekable<Iter<Token>>) -> Option<ASTNode> {
    if let Some(Token { token_type: TokenType::Lparen, .. }) = tokens.next() {
        // Initialization
        let initialization = if let Some(expr) = parse_expression(tokens) {
            expr
        } else {
            return None;
        };

        if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.next() {
            // Condition
            let condition = if let Some(expr) = parse_expression(tokens) {
                expr
            } else {
                return None;
            };

            if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.next() {
                // Increment
                let increment = if let Some(expr) = parse_expression(tokens) {
                    expr
                } else {
                    return None;
                };

                if let Some(Token { token_type: TokenType::Rparen, .. }) = tokens.next() {
                    let body = parse_block(tokens)?;
                    return Some(ASTNode::Statement(StatementNode::For {
                        initialization,
                        condition,
                        increment,
                        body,
                    }));
                }
            }
        }
    }
    None
}

// WHILE parsing
fn parse_while(tokens: &mut Peekable<Iter<Token>>) -> Option<ASTNode> {
    if let Some(Token { token_type: TokenType::Lparen, .. }) = tokens.next() {
        // Condition extraction
        let condition = if let Some(expr) = parse_expression(tokens) {
            expr
        } else {
            return None;
        };

        if let Some(Token { token_type: TokenType::Rparen, .. }) = tokens.next() {
            let body = parse_block(tokens)?;
            return Some(ASTNode::Statement(StatementNode::While { condition, body }));
        }
    }
    None
}

// block parsing
fn parse_block(tokens: &mut Peekable<Iter<Token>>) -> Option<Vec<ASTNode>> {
    if let Some(Token { token_type: TokenType::Lbrace, .. }) = tokens.next() {
        let mut body = vec![];

        while let Some(token) = tokens.peek() {
            if token.token_type == TokenType::Rbrace {
                tokens.next(); // Consume '}'
                break;
            }

            body.extend(extract_body(tokens)); // The part that I modified here
        }

        return Some(body);
    }
    None
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
    } else if type_str.starts_with("ptr<") {
        let inner_type_str = &type_str[4..type_str.len() - 1];
        let inner_type = parse_type(inner_type_str)?;
        Some(TokenType::TypePointer(Box::new(inner_type)))
    } else if type_str.starts_with("array<") {
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