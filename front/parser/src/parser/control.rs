// This file is part of the Wave language project.
// Copyright (c) 2024–2026 Wave Foundation
// Copyright (c) 2024–2026 LunaStev and contributors
//
// This Source Code Form is subject to the terms of the
// Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file,
// You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::iter::Peekable;
use std::slice::Iter;
use lexer::Token;
use lexer::token::TokenType;
use crate::ast::{ASTNode, Expression, Mutability, StatementNode, VariableNode};
use crate::expr::parse_expression;
use crate::parser::stmt::parse_block;
use crate::parser::types::parse_type_from_stream;

pub fn parse_if(tokens: &mut Peekable<Iter<Token>>) -> Option<ASTNode> {
    if tokens.peek()?.token_type != TokenType::Lparen {
        println!("Error: Expected '(' after 'if'");
        return None;
    }
    tokens.next(); // Consume '('

    let condition = parse_expression(tokens)?;

    if tokens.peek()?.token_type != TokenType::Rparen {
        println!("Error: Expected ')' after 'if' condition");
        return None;
    }
    tokens.next(); // Consume ')'

    if tokens.peek()?.token_type != TokenType::Lbrace {
        println!("Error: Expected '{{' after 'if' condition");
        return None;
    }
    tokens.next(); // Consume '{'
    let body = parse_block(tokens)?;

    let mut else_if_blocks: Vec<(Expression, Vec<ASTNode>)> = Vec::new(); // Changed to store conditions and bodies
    let mut else_block = None;

    while let Some(token) = tokens.peek() {
        if token.token_type != TokenType::Else {
            break;
        }
        tokens.next(); // consume 'else'

        if let Some(Token {
                        token_type: TokenType::If,
                        ..
                    }) = tokens.peek()
        {
            tokens.next(); // consume 'if'

            if tokens.peek()?.token_type != TokenType::Lparen {
                println!("Error: Expected '(' after 'else if'");
                return None;
            }
            tokens.next();
            let else_if_condition = parse_expression(tokens)?;

            if tokens.peek()?.token_type != TokenType::Rparen {
                println!("Error: Expected ')' after 'else if' condition");
                return None;
            }
            tokens.next();

            if tokens.peek()?.token_type != TokenType::Lbrace {
                println!("Error: Expected '{{' after 'else if'");
                return None;
            }
            tokens.next();
            let else_if_body = parse_block(tokens)?;

            // Store condition and body directly instead of nested If node
            else_if_blocks.push((else_if_condition, else_if_body));
        } else {
            if tokens.peek()?.token_type != TokenType::Lbrace {
                println!("Error: Expected '{{' after 'else'");
                return None;
            }
            tokens.next();
            else_block = Some(Box::new(parse_block(tokens)?));
            break;
        }
    }

    Some(ASTNode::Statement(StatementNode::If {
        condition,
        body,
        else_if_blocks: if else_if_blocks.is_empty() {
            None
        } else {
            Some(Box::new(else_if_blocks))
        },
        else_block,
    }))
}

fn is_typed_for_initializer(tokens: &Peekable<Iter<Token>>) -> bool {
    let mut look = tokens.clone();
    matches!(
        look.next().map(|t| &t.token_type),
        Some(TokenType::Identifier(_))
    ) && matches!(look.next().map(|t| &t.token_type), Some(TokenType::Colon))
}

fn parse_typed_for_initializer(
    tokens: &mut Peekable<Iter<Token>>,
    mutability: Mutability,
) -> Option<ASTNode> {
    let name = match tokens.next() {
        Some(Token {
                 token_type: TokenType::Identifier(name),
                 ..
             }) => name.clone(),
        _ => {
            println!("Error: Expected identifier in for-loop initializer");
            return None;
        }
    };

    if tokens.peek()?.token_type != TokenType::Colon {
        println!("Error: Expected ':' after '{}' in for-loop initializer", name);
        return None;
    }
    tokens.next(); // consume ':'

    let type_name = match parse_type_from_stream(tokens) {
        Some(ty) => ty,
        None => {
            println!("Error: Expected type in for-loop initializer");
            return None;
        }
    };

    let initial_value = if tokens.peek()?.token_type == TokenType::Equal {
        tokens.next(); // consume '='
        Some(parse_expression(tokens)?)
    } else {
        None
    };

    Some(ASTNode::Variable(VariableNode {
        name,
        type_name,
        initial_value,
        mutability,
    }))
}

fn parse_for_initializer(tokens: &mut Peekable<Iter<Token>>) -> Option<ASTNode> {
    match tokens.peek().map(|t| &t.token_type) {
        Some(TokenType::Var) => {
            tokens.next(); // consume `var`
            parse_typed_for_initializer(tokens, Mutability::Var)
        }
        Some(TokenType::Let) => {
            tokens.next(); // consume `let`
            let mutability = if matches!(tokens.peek().map(|t| &t.token_type), Some(TokenType::Mut))
            {
                tokens.next(); // consume `mut`
                Mutability::LetMut
            } else {
                Mutability::Let
            };
            parse_typed_for_initializer(tokens, mutability)
        }
        Some(TokenType::Const) => {
            tokens.next(); // consume `const`
            parse_typed_for_initializer(tokens, Mutability::Const)
        }
        _ if is_typed_for_initializer(tokens) => parse_typed_for_initializer(tokens, Mutability::Var),
        _ => {
            let expr = parse_expression(tokens)?;
            Some(ASTNode::Statement(StatementNode::Expression(expr)))
        }
    }
}

// FOR parsing
pub fn parse_for(tokens: &mut Peekable<Iter<Token>>) -> Option<ASTNode> {
    if tokens.peek()?.token_type != TokenType::Lparen {
        println!("Error: Expected '(' after 'for'");
        return None;
    }
    tokens.next(); // Consume '('

    let initialization = parse_for_initializer(tokens)?;

    if tokens.peek()?.token_type != TokenType::SemiColon {
        println!("Error: Expected ';' after for-loop initializer");
        return None;
    }
    tokens.next(); // Consume ';'

    let condition = parse_expression(tokens)?;

    if tokens.peek()?.token_type != TokenType::SemiColon {
        println!("Error: Expected ';' after for-loop condition");
        return None;
    }
    tokens.next(); // Consume ';'

    let increment = parse_expression(tokens)?;

    if tokens.peek()?.token_type != TokenType::Rparen {
        println!("Error: Expected ')' after for-loop increment");
        return None;
    }
    tokens.next(); // Consume ')'

    if tokens.peek()?.token_type != TokenType::Lbrace {
        println!("Error: Expected '{{' after 'for' header");
        return None;
    }
    tokens.next(); // Consume '{'

    let body = parse_block(tokens)?;

    Some(ASTNode::Statement(StatementNode::For {
        initialization: Box::new(initialization),
        condition,
        increment,
        body,
    }))
}

// WHILE parsing
pub fn parse_while(tokens: &mut Peekable<Iter<Token>>) -> Option<ASTNode> {
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
