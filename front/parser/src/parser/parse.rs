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
use crate::ast::ASTNode;
use crate::parser::decl::*;
use crate::parser::functions::parse_function;
use crate::parser::items::*;
use crate::verification::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Syntax(String),
    Semantic(String),
}

impl ParseError {
    pub fn message(&self) -> &str {
        match self {
            ParseError::Syntax(msg) | ParseError::Semantic(msg) => msg,
        }
    }
}

pub fn parse(tokens: &[Token]) -> Result<Vec<ASTNode>, ParseError> {
    let mut iter = tokens.iter().peekable();
    let mut nodes = vec![];

    while let Some(token) = iter.peek() {
        match token.token_type {
            TokenType::Whitespace | TokenType::Newline => {
                iter.next();
                continue;
            }
            TokenType::Import => {
                iter.next();
                if let Some(path) = parse_import(&mut iter) {
                    nodes.push(path);
                } else {
                    return Err(ParseError::Syntax("failed to parse import item".to_string()));
                }
            }
            TokenType::Extern => {
                iter.next();
                if let Some(extern_nodes) = parse_extern(&mut iter) {
                    nodes.extend(extern_nodes);
                } else {
                    return Err(ParseError::Syntax("failed to parse extern declaration".to_string()));
                }
            }
            TokenType::Const => {
                iter.next();
                if let Some(var) = parse_const(&mut iter) {
                    nodes.push(var);
                } else {
                    return Err(ParseError::Syntax("failed to parse const declaration".to_string()));
                }
            }
            TokenType::Proto => {
                iter.next();
                if let Some(proto_impl) = parse_proto(&mut iter) {
                    nodes.push(proto_impl);
                } else {
                    return Err(ParseError::Syntax("failed to parse proto impl".to_string()));
                }
            }
            TokenType::Type => {
                iter.next(); // consume 'type'
                if let Some(node) = parse_type_alias(&mut iter) {
                    nodes.push(node);
                } else {
                    return Err(ParseError::Syntax("failed to parse type alias".to_string()));
                }
            }
            TokenType::Enum => {
                iter.next(); // consume 'enum'
                if let Some(node) = parse_enum(&mut iter) {
                    nodes.push(node);
                } else {
                    return Err(ParseError::Syntax("failed to parse enum".to_string()));
                }
            }
            TokenType::Struct => {
                iter.next();
                if let Some(struct_node) = parse_struct(&mut iter) {
                    nodes.push(struct_node);
                } else {
                    return Err(ParseError::Syntax("failed to parse struct".to_string()));
                }
            }
            TokenType::Fun => {
                if let Some(func) = parse_function(&mut iter) {
                    nodes.push(func);
                } else {
                    return Err(ParseError::Syntax("failed to parse function".to_string()));
                }
            }
            TokenType::Eof => break,
            _ => {
                return Err(ParseError::Syntax(format!(
                    "unexpected token at top level: {:?}",
                    token.token_type
                )));
            }
        }
    }

    if let Err(e) = validate_program(&nodes) {
        return Err(ParseError::Semantic(e));
    }

    Ok(nodes)
}
