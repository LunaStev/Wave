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

impl<'a> Lexer<'a> {
    pub(crate) fn skip_trivia(&mut self) {
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
                self.skip_multiline_comment();
                continue;
            }

            break;
        }
    }

    pub(crate) fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            let c = self.peek();
            match c {
                ' ' | '\r' | '\t' => { self.advance(); }
                '\n' => { self.line += 1; self.advance(); }
                _ => break,
            }
        }
    }

    pub(crate) fn skip_comment(&mut self) {
        while !self.is_at_end() && self.peek() != '\n' {
            self.advance();
        }
    }

    pub(crate) fn skip_multiline_comment(&mut self) {
        while !self.is_at_end() {
            if self.peek() == '*' && self.peek_next() == '/' {
                self.advance();
                self.advance();
                break;
            }

            if self.peek() == '\n' {
                self.line += 1;
            }

            self.advance();
        }

        if self.is_at_end() {
            panic!("Unterminated block comment");
        }
    }
}
