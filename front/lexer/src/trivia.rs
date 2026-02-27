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

use super::Lexer;
use error::WaveErrorKind;
use error::WaveError;

impl<'a> Lexer<'a> {
    pub(crate) fn skip_trivia(&mut self) -> Result<(), WaveError> {
        loop {
            self.skip_whitespace();

            if self.is_at_end() {
                break;
            }

            // line comment //
            if self.peek() == '/' && self.peek_next() == '/' {
                self.advance(); // '/'
                self.advance(); // '/'
                self.skip_comment();
                continue;
            }

            // block comment /* */
            if self.peek() == '/' && self.peek_next() == '*' {
                self.advance(); // '/'
                self.advance(); // '*'
                self.skip_multiline_comment()?;
                continue;
            }

            break;
        }

        Ok(())
    }

    pub(crate) fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            let c = self.peek();
            match c {
                ' ' | '\r' | '\t' => { self.advance(); }
                '\n' => {
                    self.advance();
                    self.line += 1;
                    self.line_start = self.current;
                }
                _ => break,
            }
        }
    }

    pub(crate) fn skip_comment(&mut self) {
        while !self.is_at_end() && self.peek() != '\n' {
            self.advance();
        }
    }

    pub(crate) fn skip_multiline_comment(&mut self) -> Result<(), WaveError> {
        let mut depth: u32 = 1;

        while !self.is_at_end() {
            if self.peek() == '/' && self.peek_next() == '*' {
                self.advance();
                self.advance();
                depth += 1;
                continue;
            }

            if self.peek() == '*' && self.peek_next() == '/' {
                self.advance();
                self.advance();
                depth -= 1;
                if depth == 0 {
                    return Ok(());
                }
                continue;
            }

            if self.peek() == '\n' {
                self.advance();
                self.line += 1;
                self.line_start = self.current;
                continue;
            }

            self.advance();
        }

        Err(
            self.make_error_here(
                WaveErrorKind::UnterminatedComment,
                "unterminated block comment; expected closing `*/`",
            )
            .with_code("E1002")
            .with_label("block comment starts here and never closes")
            .with_help("add `*/` to close the block comment")
            .with_suggestion("if you intended a line comment, use `// ...`"),
        )
    }
}
