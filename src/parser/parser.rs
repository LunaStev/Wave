use std::collections::HashSet;
use std::iter::Peekable;
use std::slice::Iter;
use crate::error::*;
use crate::lexer::*;
use crate::parser::ast::*;
use crate::parser::format::*;

pub fn parse(tokens: &[Token]) -> Option<ASTNode> {
    let mut tokens_iter = tokens.iter().peekable();
    parse_function(&mut tokens_iter)
}

pub fn function(function_name: String, parameters: Vec<ParameterNode>, body: Vec<ASTNode>) -> ASTNode {
    // println!("🚨 function() called with {} body items", body.len());
    ASTNode::Function(FunctionNode {
        name: function_name,
        parameters,
        body,
    })
}

pub fn param(parameter: String, param_type: String, initial_value: Option<Value>) -> ParameterNode {
    ParameterNode {
        name: parameter,
        param_type,
        initial_value,
    }
}

pub fn parse_parameters(tokens: &mut Peekable<Iter<Token>>) -> Vec<ParameterNode> {
    let mut params = vec![];

    while let Some(token) = tokens.peek() {
        match &token.token_type {
            TokenType::Identifier(name) => {
                let name = name.clone();
                tokens.next(); // consume identifier

                if !matches!(tokens.peek().map(|t| &t.token_type), Some(TokenType::Colon)) {
                    println!("Error: Expected ':' after parameter name '{}'", name);
                    break;
                }
                tokens.next(); // consume ':'

                let param_type = match tokens.next() {
                    Some(Token { lexeme, .. }) => lexeme.clone(),
                    _ => {
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
                        tokens.next();
                        continue;
                    }
                    Some(TokenType::Rparen) => break,
                    _ => break,
                }
            }
            _ => break,
        }
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
            TokenType::Identifier(_) => {
                if let Some(ast_node) = parse_assignment(tokens, token) {
                    body.push(ast_node);
                }
            }
            TokenType::Break => {
                if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
                    tokens.next(); // consume ;
                }
                body.push(ASTNode::Statement(StatementNode::Break));
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

    if tokens.peek()?.token_type != TokenType::Lparen {
        return None;
    }

    if tokens.peek()?.token_type != TokenType::Lparen {
        return None;
    }

    let mut param_names = HashSet::new();
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
        content.clone()
    } else {
        println!("Error: Expected string literal in 'println'");
        return None;
    };

    let placeholder_count = content.matches("{}").count();

    if placeholder_count == 0 {
        // No format → Println that just outputs string
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

    let placeholder_count = content.matches("{}").count();

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
            TokenType::If => {
                parse_if(tokens)
            },
            TokenType::For => parse_for(tokens),
            TokenType::While => parse_while(tokens),
            TokenType::Identifier(_) => parse_assignment(tokens, token),
            TokenType::Break => {
                if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
                    tokens.next();
                }
                Some(ASTNode::Statement(StatementNode::Break))
            }
            _ => {
                None
            }
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