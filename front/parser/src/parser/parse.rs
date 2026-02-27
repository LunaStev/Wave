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
pub struct ParseDiagnostic {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub expected: Vec<String>,
    pub found: Option<String>,
    pub context: Option<String>,
    pub help: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Syntax(ParseDiagnostic),
    Semantic(ParseDiagnostic),
}

impl ParseError {
    pub fn syntax(message: impl Into<String>) -> Self {
        Self::Syntax(ParseDiagnostic {
            message: message.into(),
            line: 0,
            column: 0,
            expected: Vec::new(),
            found: None,
            context: None,
            help: None,
            note: None,
        })
    }

    pub fn syntax_at(token: Option<&Token>, message: impl Into<String>) -> Self {
        let mut err = Self::syntax(message);
        if let Some(tok) = token {
            err = err.with_line_col(tok.line, 1);
        }
        err
    }

    pub fn semantic(message: impl Into<String>) -> Self {
        Self::Semantic(ParseDiagnostic {
            message: message.into(),
            line: 0,
            column: 0,
            expected: Vec::new(),
            found: None,
            context: None,
            help: None,
            note: None,
        })
    }

    fn diag_mut(&mut self) -> &mut ParseDiagnostic {
        match self {
            ParseError::Syntax(d) | ParseError::Semantic(d) => d,
        }
    }

    fn token_desc(token: &Token) -> String {
        if token.lexeme.is_empty() {
            format!("{:?}", token.token_type)
        } else {
            format!("{:?} (`{}`)", token.token_type, token.lexeme)
        }
    }

    pub fn with_line_col(mut self, line: usize, column: usize) -> Self {
        let d = self.diag_mut();
        d.line = line;
        d.column = column;
        self
    }

    pub fn with_expected(mut self, expected: impl Into<String>) -> Self {
        self.diag_mut().expected.push(expected.into());
        self
    }

    pub fn with_expected_many<I, S>(mut self, expected: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.diag_mut().expected = expected.into_iter().map(|s| s.into()).collect();
        self
    }

    pub fn with_found(mut self, found: impl Into<String>) -> Self {
        self.diag_mut().found = Some(found.into());
        self
    }

    pub fn with_found_token(mut self, token: Option<&Token>) -> Self {
        if let Some(tok) = token {
            let d = self.diag_mut();
            if d.line == 0 {
                d.line = tok.line;
            }
            if d.column == 0 {
                d.column = 1;
            }
            d.found = Some(Self::token_desc(tok));
        }
        self
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.diag_mut().context = Some(context.into());
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.diag_mut().help = Some(help.into());
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.diag_mut().note = Some(note.into());
        self
    }

    pub fn message(&self) -> &str {
        match self {
            ParseError::Syntax(d) | ParseError::Semantic(d) => &d.message,
        }
    }

    pub fn line(&self) -> usize {
        match self {
            ParseError::Syntax(d) | ParseError::Semantic(d) => d.line,
        }
    }

    pub fn column(&self) -> usize {
        match self {
            ParseError::Syntax(d) | ParseError::Semantic(d) => d.column,
        }
    }

    pub fn expected(&self) -> &[String] {
        match self {
            ParseError::Syntax(d) | ParseError::Semantic(d) => &d.expected,
        }
    }

    pub fn found(&self) -> Option<&str> {
        match self {
            ParseError::Syntax(d) | ParseError::Semantic(d) => d.found.as_deref(),
        }
    }

    pub fn context(&self) -> Option<&str> {
        match self {
            ParseError::Syntax(d) | ParseError::Semantic(d) => d.context.as_deref(),
        }
    }

    pub fn help(&self) -> Option<&str> {
        match self {
            ParseError::Syntax(d) | ParseError::Semantic(d) => d.help.as_deref(),
        }
    }

    pub fn note(&self) -> Option<&str> {
        match self {
            ParseError::Syntax(d) | ParseError::Semantic(d) => d.note.as_deref(),
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
                let anchor = (*token).clone();
                iter.next();
                if let Some(path) = parse_import(&mut iter) {
                    nodes.push(path);
                } else {
                    return Err(
                        ParseError::syntax_at(Some(&anchor), "failed to parse import declaration")
                            .with_context("top-level import")
                            .with_expected("import(\"path\");")
                            .with_found_token(iter.peek().copied())
                            .with_help("imports must use parentheses and end with ';'"),
                    );
                }
            }
            TokenType::Extern => {
                let anchor = (*token).clone();
                iter.next();
                if let Some(extern_nodes) = parse_extern(&mut iter) {
                    nodes.extend(extern_nodes);
                } else {
                    return Err(
                        ParseError::syntax_at(Some(&anchor), "failed to parse extern declaration")
                            .with_context("top-level extern block/declaration")
                            .with_expected_many([
                                "extern(c) fun name(...);",
                                "extern(c) { fun a(...); fun b(...); }",
                            ])
                            .with_found_token(iter.peek().copied())
                            .with_help("check ABI syntax, function signature, and separators"),
                    );
                }
            }
            TokenType::Const => {
                let anchor = (*token).clone();
                iter.next();
                if let Some(var) = parse_const(&mut iter) {
                    nodes.push(var);
                } else {
                    return Err(
                        ParseError::syntax_at(Some(&anchor), "failed to parse const declaration")
                            .with_context("top-level constant declaration")
                            .with_expected("const name: type = value;")
                            .with_found_token(iter.peek().copied())
                            .with_help("const declarations require explicit type and initializer"),
                    );
                }
            }
            TokenType::Proto => {
                let anchor = (*token).clone();
                iter.next();
                if let Some(proto_impl) = parse_proto(&mut iter) {
                    nodes.push(proto_impl);
                } else {
                    return Err(
                        ParseError::syntax_at(Some(&anchor), "failed to parse proto implementation")
                            .with_context("top-level proto block")
                            .with_expected("proto Type { fun method(...); }")
                            .with_found_token(iter.peek().copied())
                            .with_help("check braces and method declarations inside proto"),
                    );
                }
            }
            TokenType::Type => {
                let anchor = (*token).clone();
                iter.next(); // consume 'type'
                if let Some(node) = parse_type_alias(&mut iter) {
                    nodes.push(node);
                } else {
                    return Err(
                        ParseError::syntax_at(Some(&anchor), "failed to parse type alias")
                            .with_context("top-level type alias")
                            .with_expected("type Name = ExistingType;")
                            .with_found_token(iter.peek().copied())
                            .with_help("type aliases must include `=` and end with ';'"),
                    );
                }
            }
            TokenType::Enum => {
                let anchor = (*token).clone();
                iter.next(); // consume 'enum'
                if let Some(node) = parse_enum(&mut iter) {
                    nodes.push(node);
                } else {
                    return Err(
                        ParseError::syntax_at(Some(&anchor), "failed to parse enum declaration")
                            .with_context("top-level enum declaration")
                            .with_expected("enum Name -> i32 { A = 0, B = 1 }")
                            .with_found_token(iter.peek().copied())
                            .with_help("check enum repr type, braces, and variant values"),
                    );
                }
            }
            TokenType::Struct => {
                let anchor = (*token).clone();
                iter.next();
                if let Some(struct_node) = parse_struct(&mut iter) {
                    nodes.push(struct_node);
                } else {
                    return Err(
                        ParseError::syntax_at(Some(&anchor), "failed to parse struct declaration")
                            .with_context("top-level struct declaration")
                            .with_expected("struct Name { field: type; fun method(...) { ... } }")
                            .with_found_token(iter.peek().copied())
                            .with_help("check field separators (`;`) and method bodies"),
                    );
                }
            }
            TokenType::Fun => {
                let anchor = (*token).clone();
                if let Some(func) = parse_function(&mut iter) {
                    nodes.push(func);
                } else {
                    return Err(
                        ParseError::syntax_at(Some(&anchor), "failed to parse function declaration")
                            .with_context("top-level function")
                            .with_expected_many([
                                "fun name(params) { ... }",
                                "fun name(params) -> return_type { ... }",
                            ])
                            .with_found_token(iter.peek().copied())
                            .with_help("check parameter syntax, return type arrow, and function body braces"),
                    );
                }
            }
            TokenType::Eof => break,
            _ => {
                return Err(
                    ParseError::syntax_at(Some(token), "unexpected token at top level")
                        .with_context("top-level items")
                        .with_expected_many([
                            "import",
                            "extern",
                            "const",
                            "type",
                            "enum",
                            "struct",
                            "proto",
                            "fun",
                        ])
                        .with_found_token(Some(token))
                        .with_help("only declarations are allowed at top level"),
                );
            }
        }
    }

    if let Err(e) = validate_program(&nodes) {
        return Err(
            ParseError::semantic(e)
                .with_context("semantic validation")
                .with_help("fix mutability, scope, and expression validity issues"),
        );
    }

    Ok(nodes)
}
