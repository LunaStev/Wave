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

use lexer::Token;
use lexer::token::TokenType;
use crate::ast::{Expression, IncDecKind, Literal, Operator};
use crate::expr::is_assignable;
use crate::expr::primary::parse_primary_expression;

pub fn parse_unary_expression<'a, T>(tokens: &mut std::iter::Peekable<T>) -> Option<Expression>
where
    T: Iterator<Item = &'a Token>,
{
    if let Some(token) = tokens.peek() {
        match token.token_type {
            TokenType::Not => {
                tokens.next();
                let inner = parse_unary_expression(tokens)?;
                return Some(Expression::Unary {
                    operator: Operator::Not,
                    expr: Box::new(inner),
                });
            }
            TokenType::BitwiseNot => {
                tokens.next();
                let inner = parse_unary_expression(tokens)?;
                return Some(Expression::Unary {
                    operator: Operator::BitwiseNot,
                    expr: Box::new(inner),
                });
            }
            TokenType::AddressOf => {
                tokens.next();
                let inner = parse_unary_expression(tokens)?;
                return Some(Expression::AddressOf(Box::new(inner)));
            }
            TokenType::Deref => {
                tokens.next();
                let inner = parse_unary_expression(tokens)?;
                return Some(Expression::Deref(Box::new(inner)));
            }
            TokenType::Increment => {
                let tok = tokens.next()?; // '++'
                let inner = parse_unary_expression(tokens)?;
                if !is_assignable(&inner) {
                    println!("Error: ++ target must be assignable (line {})", tok.line);
                    return None;
                }
                return Some(Expression::IncDec {
                    kind: IncDecKind::PreInc,
                    target: Box::new(inner),
                });
            }
            TokenType::Decrement => {
                let tok = tokens.next()?; // '--'
                let inner = parse_unary_expression(tokens)?;
                if !is_assignable(&inner) {
                    println!("Error: -- target must be assignable (line {})", tok.line);
                    return None;
                }
                return Some(Expression::IncDec {
                    kind: IncDecKind::PreDec,
                    target: Box::new(inner),
                });
            }
            TokenType::Minus => {
                let _tok = tokens.next()?; // '-'
                let inner = parse_unary_expression(tokens)?;

                match inner {
                    Expression::Literal(Literal::Int(s)) => {
                        return Some(Expression::Literal(Literal::Int(format!("-{}", s))));
                    }
                    Expression::Literal(Literal::Float(f)) => {
                        return Some(Expression::Literal(Literal::Float(-f)));
                    }

                    other => {
                        return Some(Expression::Unary {
                            operator: Operator::Neg,
                            expr: Box::new(other),
                        })
                    }
                }
            }

            TokenType::Plus => {
                tokens.next(); // consume '+'
                let inner = parse_unary_expression(tokens)?;
                return Some(inner);
            }
            _ => {}
        }
    }

    parse_primary_expression(tokens)
}