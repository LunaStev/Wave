use std::collections::HashSet;
use std::iter::Peekable;
use std::slice::Iter;
use regex::Regex;
use ::lexer::*;
use parser::ast::*;
use crate::*;
use crate::parser::format::*;

pub fn parse(tokens: &Vec<Token>) -> Option<Vec<ASTNode>> {
    let mut iter = tokens.iter().peekable();
    let mut nodes = vec![];

    while let Some(token) = iter.peek() {
        match token.token_type {
            TokenType::Import => {
                iter.next();
                if let Some(path) = parse_import(&mut iter) {
                    nodes.push(path);
                } else {
                    return None;
                }
            }
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
                let param_type = match token {
                    Some(Token { token_type: TokenType::Identifier(type_name), .. }) => {
                        if let Some(Token { token_type: TokenType::Lchevr, .. }) = tokens.peek() {
                            tokens.next(); // consume '<'

                            let mut inner = String::new();
                            let mut depth = 1;

                            while let Some(t) = tokens.next() {
                                match &t.token_type {
                                    TokenType::Lchevr => {
                                        depth += 1;
                                        inner.push('<');
                                    },
                                    TokenType::Rchevr => {
                                        depth -= 1;
                                        if depth == 0 {
                                            break;
                                        } else {
                                            inner.push('>');
                                        }
                                    },
                                    _ => inner.push_str(&t.lexeme),
                                }
                            }

                            let full_type_str = format!("{}<{}>", type_name, inner);
                            let parsed_type = parse_type(&full_type_str);

                            match parsed_type {
                                Some(ref parsed) => match token_type_to_wave_type(parsed) {
                                    Some(wt) => wt,
                                    None => {
                                        println!("❌ Failed to convert generic type to WaveType: {}", full_type_str);
                                        break;
                                    }
                                },
                                None => {
                                    println!("❌ Unknown generic type: {}", full_type_str);
                                    break;
                                }
                            }
                        } else {
                            match parse_type(type_name).and_then(|tt| token_type_to_wave_type(&tt)) {
                                Some(wt) => wt,
                                None => {
                                    println!("❌ Unknown type: {}", type_name);
                                    break;
                                }
                            }
                        }
                    },
                    Some(Token { token_type, .. }) => match token_type_to_wave_type(token_type) {
                        Some(wt) => wt,
                        None => {
                            println!("❌ Unsupported or unknown type token: {:?}", token_type);
                            break;
                        }
                    },
                    None => {
                        println!("❌ Expected type after ':' for parameter '{}'", name);
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
                    Some(TokenType::Comma) => {
                        tokens.next();
                        continue;
                    }
                    Some(TokenType::Rparen) => {
                        tokens.next();
                        break;
                    }
                    Some(TokenType::SemiColon) => {
                        println!("Error: use `,` instead of `;` to separate parameters");
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
            TokenType::Whitespace => {
                tokens.next(); // ignore
            }
            TokenType::Rbrace => {
                tokens.next();
                break;
            }
            TokenType::Eof => {
                println!("❌ Unexpected EOF inside function body");
                return None;
            }
            TokenType::Asm => {
                tokens.next();
                body.push(parse_asm_block(tokens)?);
            }
            TokenType::Var => {
                tokens.next(); // consume 'var'
                body.push(parse_var(tokens)?);
            }
            TokenType::Let => {
                tokens.next(); // consume 'let'
                body.push(parse_let(tokens)?);
            }
            TokenType::Println => {
                tokens.next(); // consume 'println'
                let node = parse_println(tokens)?;
                // 세미콜론 처리 추가
                if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
                    tokens.next();
                }
                body.push(node);
            }
            TokenType::Print => {
                tokens.next(); // consume 'print'
                let node = parse_print(tokens)?;
                // 세미콜론 처리 추가
                if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
                    tokens.next();
                }
                body.push(node);
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
            TokenType::Identifier(_) => {
                if let Some(expr) = parse_expression(tokens) {
                    if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
                        tokens.next(); // consume ';'
                    }
                    body.push(ASTNode::Statement(StatementNode::Expression(expr)));
                } else {
                    println!("❌ Failed to parse expression starting with identifier");
                    return None;
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
                tokens.next(); // consume 'continue'
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
                    let value = match parse_expression(tokens) {
                        Some(v) => v,
                        None => {
                            println!("Error: Expected valid expression after 'return'");
                            return None;
                        }
                    };

                    if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
                        tokens.next();
                    } else {
                        println!("Error: Missing semicolon after return expression");
                        return None;
                    }
                    Some(value)
                };

                body.push(ASTNode::Statement(StatementNode::Return(expr)));
            }
            TokenType::Deref => {
                let token = token.clone();
                tokens.next();
                body.push(parse_assignment(tokens, &token)?);
            }
            _ => {
                if let Some(expr) = parse_expression(tokens) {
                    if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
                        tokens.next(); // consume ;
                    }
                    body.push(ASTNode::Statement(StatementNode::Expression(expr)));
                } else {
                    tokens.next(); // fallback skip
                }
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

        let type_token = match tokens.next() {
            Some(t) => t,
            None => {
                println!("Error: Expected type after '->'");
                return None;
            }
        };

        if let TokenType::Identifier(ref name) = type_token.token_type {
            if let Some(Token { token_type: TokenType::Lchevr, .. }) = tokens.peek() {
                tokens.next(); // consume '<'

                let mut inner = String::new();
                let mut depth = 1;

                while let Some(t) = tokens.next() {
                    match &t.token_type {
                        TokenType::Lchevr => {
                            depth += 1;
                            inner.push('<');
                        }
                        TokenType::Rchevr => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            } else {
                                inner.push('>');
                            }
                        }
                        _ => inner.push_str(&t.lexeme),
                    }
                }

                let full_type_str = format!("{}<{}>", name, inner);
                let parsed = parse_type(&full_type_str);
                if let Some(t) = parsed {
                    token_type_to_wave_type(&t)
                } else {
                    println!("Unknown generic type: {}", full_type_str);
                    None
                }
            } else {
                match parse_type(name).and_then(|t| token_type_to_wave_type(&t)) {
                    Some(t) => Some(t),
                    None => {
                        println!("Unknown return type: {}", name);
                        None
                    }
                }
            }
        } else {
            token_type_to_wave_type(&type_token.token_type)
        }
    } else {
        None
    };

    let body = extract_body(tokens)?;
    Some(ASTNode::Function(FunctionNode {
        name,
        parameters,
        body,
        return_type,
    }))
}

// VAR parsing
fn parse_var(tokens: &mut Peekable<Iter<'_, Token>>) -> Option<ASTNode> {
    let mutability = Mutability::Var;

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

    let wave_type = if let TokenType::Identifier(ref name) = type_token.token_type {
        if let Some(Token { token_type: TokenType::Lchevr, .. }) = tokens.peek() {
            tokens.next(); // consume '<'

            let mut inner = String::new();
            let mut depth = 1;

            while let Some(t) = tokens.next() {
                match &t.token_type {
                    TokenType::Lchevr => {
                        depth += 1;
                        inner.push('<');
                    },
                    TokenType::Rchevr => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        } else {
                            inner.push('>');
                        }
                    },
                    _ => inner.push_str(&t.lexeme),
                }
            }

            let full_type_str = format!("{}<{}>", name, inner);
            let parsed_type = parse_type(&full_type_str);

            if parsed_type.is_none() {
                println!("Unknown generic type: {}", full_type_str);
                return None;
            }

            match token_type_to_wave_type(&parsed_type.unwrap()) {
                Some(wt) => wt,
                None => {
                    println!("Failed to convert to WaveType: {}", full_type_str);
                    return None;
                }
            }
        } else {
            match parse_type(&name).and_then(|tt| token_type_to_wave_type(&tt)) {
                Some(wt) => wt,
                None => {
                    println!("Unknown type: {}", name);
                    return None;
                }
            }
        }
    } else {
        match token_type_to_wave_type(&type_token.token_type) {
            Some(t) => t,
            None => {
                println!("Unknown or unsupported type: {}", type_token.lexeme);
                return None;
            }
        }
    };

    let initial_value = if let Some(Token { token_type: TokenType::Equal, .. }) = tokens.peek() {
        tokens.next(); // consume '='
        let expr = parse_expression(tokens)?;
        Some(expr)
    } else {
        None
    };

    if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
        tokens.next(); // Consume ';'
    }

    if let (WaveType::Array(_, expected_len), Some(Expression::ArrayLiteral(elements))) = (&wave_type, &initial_value) {
        if *expected_len != elements.len() as u32 {
            println!(
                "❌ Error: Array length mismatch. Expected {}, but got {} elements",
                expected_len,
                elements.len()
            );
            return None;
        }
    }

    Some(ASTNode::Variable(VariableNode {
        name,
        type_name: wave_type,
        initial_value,
        mutability,
    }))
}

fn parse_let(tokens: &mut Peekable<Iter<'_, Token>>) -> Option<ASTNode> {
    let mut mutability = Mutability::Let;

    if let Some(Token { token_type: TokenType::Mut, .. }) = tokens.peek() {
        tokens.next(); // consume `mut`
        mutability = Mutability::LetMut;
    }

    let name = match tokens.next() {
        Some(Token { token_type: TokenType::Identifier(name), .. }) => name.clone(),
        _ => {
            println!("Expected identifier after `let`");
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

    let wave_type = if let TokenType::Identifier(ref name) = type_token.token_type {
        if let Some(Token { token_type: TokenType::Lchevr, .. }) = tokens.peek() {
            tokens.next(); // consume '<'

            let mut inner = String::new();
            let mut depth = 1;

            while let Some(t) = tokens.next() {
                match &t.token_type {
                    TokenType::Lchevr => {
                        depth += 1;
                        inner.push('<');
                    },
                    TokenType::Rchevr => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        } else {
                            inner.push('>');
                        }
                    },
                    _ => inner.push_str(&t.lexeme),
                }
            }

            let full_type_str = format!("{}<{}>", name, inner);
            let parsed_type = parse_type(&full_type_str);

            if parsed_type.is_none() {
                println!("Unknown generic type: {}", full_type_str);
                return None;
            }

            match token_type_to_wave_type(&parsed_type.unwrap()) {
                Some(wt) => wt,
                None => {
                    println!("Failed to convert to WaveType: {}", full_type_str);
                    return None;
                }
            }
        } else {
            match parse_type(&name).and_then(|tt| token_type_to_wave_type(&tt)) {
                Some(wt) => wt,
                None => {
                    println!("Unknown type: {}", name);
                    return None;
                }
            }
        }
    } else {
        match token_type_to_wave_type(&type_token.token_type) {
            Some(t) => t,
            None => {
                println!("Unknown or unsupported type: {}", type_token.lexeme);
                return None;
            }
        }
    };

    let initial_value = if let Some(Token { token_type: TokenType::Equal, .. }) = tokens.peek() {
        tokens.next(); // consume '='
        let expr = parse_expression(tokens)?; // 반드시 expression 파서 있어야 함
        Some(expr)
    } else {
        None
    };

    if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
        tokens.next(); // Consume ';'
    }

    if let (WaveType::Array(_, expected_len), Some(Expression::ArrayLiteral(elements))) = (&wave_type, &initial_value) {
        if *expected_len != elements.len() as u32 {
            println!(
                "❌ Error: Array length mismatch. Expected {}, but got {} elements",
                expected_len,
                elements.len()
            );
            return None;
        }
    }

    Some(ASTNode::Variable(VariableNode {
        name,
        type_name: wave_type,
        initial_value,
        mutability,
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

fn parse_import(tokens: &mut Peekable<Iter<Token>>) -> Option<ASTNode> {
    if tokens.peek()?.token_type != TokenType::Lparen {
        println!("Error: Expected '(' after 'import'");
        return None;
    }
    tokens.next();

    let import_path = match tokens.next() {
        Some(Token { token_type: TokenType::String(s), .. }) => s.clone(),
        other => {
            println!("Error: Expected string literal in import, found {:?}", other);
            return None;
        }
    };

    if tokens.peek()?.token_type != TokenType::Rparen {
        println!("Error: Expected ')' after 'import' condition");
        return None;
    }
    tokens.next();

    if tokens.peek()?.token_type != TokenType::SemiColon {
        println!("Error: Expected ';' after 'import' condition");
        return None;
    }
    tokens.next();

    Some(ASTNode::Statement(StatementNode::Import(import_path)))
}

fn parse_asm_block(tokens: &mut Peekable<Iter<Token>>) -> Option<ASTNode> {
    if tokens.peek()?.token_type != TokenType::Lbrace {
        println!("Expected '{{' after 'asm'");
        return None;
    }
    tokens.next();

    let mut instructions = vec![];
    let mut inputs = vec![];
    let mut outputs = vec![];

    while let Some(token) = tokens.next() {
        match &token.token_type {
            TokenType::Rbrace => break,

            TokenType::In | TokenType::Out => {
                let is_input = matches!(token.token_type, TokenType::In);

                if tokens.next().map(|t| t.token_type.clone()) != Some(TokenType::Lparen) {
                    println!("Expected '(' after in/out");
                    return None;
                }

                let reg_token = tokens.next();
                let reg = match reg_token {
                    Some(Token { token_type: TokenType::String(s), .. }) => s.clone(),
                    Some(Token { token_type: TokenType::Identifier(s), .. }) => s.clone(),
                    Some(other) => {
                        println!("Expected register string or identifier, got {:?}", other.token_type);
                        return None;
                    }
                    None => {
                        println!("Expected register in in/out(...)");
                        return None;
                    }
                };

                if tokens.next().map(|t| t.token_type.clone()) != Some(TokenType::Rparen) {
                    println!("Expected ')' after in/out");
                    return None;
                }

                let value_token = tokens.next();
                let value = match value_token {
                    Some(Token { token_type: TokenType::Identifier(s), .. }) => s.clone(),
                    Some(Token { token_type: TokenType::Number(n), .. }) => n.to_string(),
                    Some(Token { token_type: TokenType::String(n), .. }) => n.to_string(),
                    Some(other) => {
                        println!("Expected identifier or number after in/out(...), got {:?}", other.token_type);
                        return None;
                    }
                    None => {
                        println!("Expected value after in/out(...)");
                        return None;
                    }
                };

                if is_input {
                    inputs.push((reg.clone(), value));
                } else {
                    outputs.push((reg.clone(), value));
                }
            }

            TokenType::String(s) => {
                instructions.push(s.clone());
            }

            other => {
                println!("Unexpected token in asm expression {:?}", other);
            }
        }
    }

    Some(ASTNode::Statement(StatementNode::AsmBlock {
        instructions,
        inputs,
        outputs,
    }))
}

fn parse_assignment(tokens: &mut Peekable<Iter<Token>>, first_token: &Token) -> Option<ASTNode> {
    let left_expr = match parse_expression_from_token(first_token, tokens) {
        Some(expr) => expr,
        None => {
            println!("Error: Failed to parse left-hand side of assignment. Token: {:?}", first_token.token_type);
            return None;
        }
    };

    let assign_op = match tokens.peek()?.token_type {
        TokenType::PlusEq => {
            tokens.next();
            Some(AssignOperator::AddAssign)
        }
        TokenType::MinusEq => {
            tokens.next();
            Some(AssignOperator::SubAssign)
        }
        TokenType::StarEq => {
            tokens.next();
            Some(AssignOperator::MulAssign)
        }
        TokenType::DivEq => {
            tokens.next();
            Some(AssignOperator::DivAssign)
        }
        TokenType::RemainderEq => {
            tokens.next();
            Some(AssignOperator::RemAssign)
        }
        TokenType::Equal => {
            tokens.next();
            None
        }
        _ => return None,
    };

    let right_expr = parse_expression(tokens)?;

    if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
        tokens.next();
    }

    match (assign_op, &left_expr) {
        (Some(op), Expression::Variable(name)) => Some(ASTNode::Expression(Expression::AssignOperation {
            target: Box::new(Expression::Variable(name.clone())),
            operator: op,
            value: Box::new(right_expr),
        })),
        (None, Expression::Variable(name)) => Some(ASTNode::Statement(StatementNode::Assign {
            variable: name.clone(),
            value: right_expr,
        })),
        (None, Expression::Deref(_)) => Some(ASTNode::Statement(StatementNode::Assign {
            variable: "deref".to_string(),
            value: Expression::BinaryExpression {
                left: Box::new(left_expr),
                operator: Operator::Assign,
                right: Box::new(right_expr),
            },
        })),
        (_, _) => {
            println!("Error: Unsupported assignment left expression: {:?}", left_expr);
            None
        }
    }
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
            TokenType::Continue => {
                if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
                    tokens.next(); // consume ;
                }
                Some(ASTNode::Statement(StatementNode::Continue))
            }
            TokenType::Break => {
                if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
                    tokens.next(); // consume ;
                }
                Some(ASTNode::Statement(StatementNode::Break))
            }
            TokenType::Identifier(_) => parse_assignment(tokens, token),
            TokenType::Return => {
                let expr = if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
                    tokens.next(); // consume ;
                    None
                } else {
                    if let Some(peek_token) = tokens.peek() {
                        match peek_token.token_type {
                            TokenType::Break | TokenType::Continue | TokenType::Return => {
                                println!("Error: Invalid return expression starting with {:?}", peek_token.token_type);
                                tokens.next(); // consume the invalid token
                                if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
                                    tokens.next(); // consume ;
                                }
                                None
                            }
                            _ => {
                                let value = parse_expression(tokens)?;
                                if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
                                    tokens.next(); // consume ;
                                }
                                Some(value)
                            }
                        }
                    } else {
                        None
                    }
                };
                Some(ASTNode::Statement(StatementNode::Return(expr)))
            }
            _ => {
                if matches!(token.token_type, TokenType::Continue | TokenType::Break | TokenType::Return) {
                    None
                } else if let Some(expr) = parse_expression(tokens) {
                    if let Some(Token { token_type: TokenType::SemiColon, .. }) = tokens.peek() {
                        tokens.next();
                    }
                    Some(ASTNode::Statement(StatementNode::Expression(expr)))
                } else {
                    if !matches!(token.token_type, TokenType::SemiColon) {
                        println!("Error: Expected primary expression, found {:?}", token.token_type);
                    }
                    while let Some(tok) = tokens.next() {
                        if tok.token_type == TokenType::SemiColon {
                            break;
                        }
                    }
                    None
                }
            }
        };

        if let Some(ast_node) = node {
            body.push(ast_node);
        }
    }

    Some(body)
}

pub fn parse_type(type_str: &str) -> Option<TokenType> {
    let type_str = type_str.trim();

    if let Some(lt_index) = type_str.find('<') {
        if !type_str.ends_with('>') {
            return None;
        }

        let base = &type_str[..lt_index];
        let inner = &type_str[lt_index + 1..type_str.len() - 1];

        if base == "array" {
            let mut depth = 0;
            let mut split_pos = None;

            for (i, c) in inner.char_indices() {
                match c {
                    '<' => depth += 1,
                    '>' => depth -= 1,
                    ',' if depth == 0 => {
                        split_pos = Some(i);
                        break;
                    }
                    _ => {}
                }
            }

            let split_pos = split_pos?;
            let elem_type_str = inner[..split_pos].trim();
            let size_str = inner[split_pos + 1..].trim();

            let elem_type = parse_type(elem_type_str)?;
            let size = size_str.parse::<u32>().ok()?;

            return Some(TokenType::TypeArray(Box::new(elem_type), size));
        }

        if base == "ptr" {
            let inner_type = parse_type(inner)?;
            return Some(TokenType::TypePointer(Box::new(inner_type)));
        }

        return None;
    }

    if type_str.starts_with('i') {
        let bits = type_str[1..].parse::<u16>().ok()?;
        return Some(TokenType::TypeInt(bits));
    } else if type_str.starts_with('u') {
        let bits = type_str[1..].parse::<u16>().ok()?;
        return Some(TokenType::TypeUint(bits));
    } else if type_str.starts_with('f') {
        let bits = type_str[1..].parse::<u16>().ok()?;
        return Some(TokenType::TypeFloat(bits));
    } else if type_str == "bool" {
        return Some(TokenType::TypeBool);
    } else if type_str == "char" {
        return Some(TokenType::TypeChar);
    } else if type_str == "byte" {
        return Some(TokenType::TypeByte);
    } else if type_str == "str" {
        return Some(TokenType::TypeString);
    }

    None
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