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
// AI TRAINING NOTICE: Prohibited without prior written permission. No use for machine learning or generative AI training, fine-tuning, distillation, embedding, or dataset creation.

//! Parsing of `print`, `println`, and `input` statements.
//!
//! Placeholder counts are checked while parsing so the AST distinguishes plain
//! literal output from formatted calls. Argument types and C format conversion
//! remain semantic/backend responsibilities.

use crate::ast::{ASTNode, StatementNode};
use crate::expr::parse_expression;
use crate::parser::ParseError;
use lexer::token::TokenType;
use lexer::Token;
use std::iter::Peekable;
use std::slice::Iter;
use utils::formatx::*;

pub fn parse_println(tokens: &mut Peekable<Iter<Token>>) -> Result<ASTNode, ParseError> {
    parse_io(tokens, "println")
}

pub fn parse_print(tokens: &mut Peekable<Iter<Token>>) -> Result<ASTNode, ParseError> {
    parse_io(tokens, "print")
}

pub fn parse_input(tokens: &mut Peekable<Iter<Token>>) -> Result<ASTNode, ParseError> {
    parse_io(tokens, "input")
}

fn parse_io(tokens: &mut Peekable<Iter<Token>>, name: &str) -> Result<ASTNode, ParseError> {
    let anchor = tokens.peek().copied();
    crate::expr::expect_token(tokens, anchor, TokenType::Lparen, "'('", name)?;
    let content_token = tokens.peek().copied();
    let Some(Token { token_type: TokenType::String(bytes), .. }) = content_token else {
        return Err(ParseError::expected_at(content_token, anchor, "format string literal", name));
    };
    let mut content = bytes.clone();
    tokens.next();
    validate_placeholder_text(&content, content_token, name == "input")?;
    let count = count_placeholders(&content);
    let mut args = Vec::new();
    while tokens.peek().is_some_and(|t| t.token_type == TokenType::Comma) {
        tokens.next();
        args.push(parse_expression(tokens)?);
    }
    crate::expr::expect_token(tokens, anchor, TokenType::Rparen, "')'", name)?;
    crate::parser::stmt::expect_semicolon(tokens, anchor, name)?;
    if count != args.len() {
        return Err(ParseError::syntax_at(
            content_token,
            format!("expected {count} format arguments, found {}", args.len()),
        )
        .with_context(format!("{name} format arguments"))
        .with_help("supply one argument for each format placeholder"));
    }
    if name == "input" {
        return Ok(ASTNode::Statement(StatementNode::Input { format: content, args }));
    }
    if name == "println" {
        content.push(b'\n');
    }
    Ok(ASTNode::Statement(match (name, count) {
        ("println", 0) => StatementNode::Println(content),
        ("println", _) => StatementNode::PrintlnFormat { format: content, args },
        (_, 0) => StatementNode::Print(content),
        _ => StatementNode::PrintFormat { format: content, args },
    }))
}

// Map decoded byte ranges back to the original literal spelling. Each decoded
// byte keeps its complete source character/escape, including escaped braces.
fn placeholder_span(
    token: Option<&Token>,
    range: std::ops::Range<usize>,
) -> Option<error::SourceSpan> {
    let token = token?;
    let mut span = token.span.clone()?;
    let raw = &token.lexeme;
    let mut mapping = Vec::new();
    let mut position = 1;
    while position + 1 < raw.len() {
        let start = position;
        let decoded_len;
        if raw.as_bytes()[position] == b'\\' {
            position += if raw.as_bytes().get(position + 1) == Some(&b'x') { 4 } else { 2 };
            decoded_len = 1;
        } else {
            decoded_len = raw[position..].chars().next()?.len_utf8();
            position += decoded_len;
        }
        for _ in 0..decoded_len {
            mapping.push(start..position);
        }
    }
    let start = mapping.get(range.start)?.start;
    let end = mapping.get(range.end.checked_sub(1)?)?.end;
    span.start += start;
    span.end = token.span.as_ref()?.start + end;
    span.column += raw[..start].chars().count();
    span.end_column = token.span.as_ref()?.column + raw[..end].chars().count();
    span.end_line = span.line;
    span.focus = None;
    Some(span)
}

fn validate_placeholder_text(
    content: &[u8],
    token: Option<&Token>,
    input: bool,
) -> Result<(), ParseError> {
    let invalid = |message: String, range: std::ops::Range<usize>| {
        ParseError::syntax_at(token, message)
            .with_context("format placeholder")
            .with_found_token(token)
            .with_span(placeholder_span(token, range).as_ref())
            .with_help(if input {
                "input supports only `{}` placeholders"
            } else {
                "use `{}`, `{d}`, `{x}`, `{c}`, `{s}`, `{p}`, or fixed float precision `{:.N}` (0..=2147483647)"
            })
    };
    let parts =
        format_fragments_located(content).map_err(|e| invalid(e.message.into(), e.range))?;
    for (part, range) in parts {
        if let FormatFragment::Placeholder(spec) = part {
            let precision = float_precision(spec).map_err(|e| invalid(e.into(), range.clone()))?;
            if !(spec.is_empty()
                || !input && (matches!(spec, "c" | "x" | "p" | "s" | "d") || precision.is_some()))
            {
                return Err(invalid(format!("unsupported format placeholder `{{{spec}}}`"), range));
            }
        }
    }
    Ok(())
}
