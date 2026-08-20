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

//! Lightweight Wave format-placeholder scanning without regular expressions.
//!
//! The supported form is a non-nested `{...}` pair with no escape processing.
//! Unterminated opening braces are ignored rather than counted as placeholders.

#[derive(Debug, Clone)]
pub struct Placeholder {
    pub spec: String,
}

/// Returns placeholders in source order, trimming the text inside each pair.
pub fn parse_placeholders(input: &str) -> Vec<Placeholder> {
    let bytes = input.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();

    while i < bytes.len() {
        if bytes[i] == b'{' {
            i += 1;
            let start = i;
            while i < bytes.len() && bytes[i] != b'}' {
                i += 1;
            }
            if i >= bytes.len() {
                break;
            }

            let spec = input[start..i].trim().to_string();
            out.push(Placeholder { spec });

            i += 1; // consume '}'
        } else {
            i += 1;
        }
    }

    out
}

/// Count `{...}` placeholders in the given string.
///
/// Equivalent to the regex pattern: `\{[^}]*\}`
///
/// Examples:
/// - "hello {}" -> 1
/// - "{a}{b}{c}" -> 3
/// - "{ not closed" -> 0
pub fn count_placeholders(input: &str) -> usize {
    let bytes = input.as_bytes();
    let mut i = 0;
    let mut count = 0;

    while i < bytes.len() {
        if bytes[i] == b'{' {
            let start = i;
            i += 1;

            while i < bytes.len() {
                if bytes[i] == b'}' {
                    count += 1;
                    i += 1;
                    break;
                }
                i += 1;
            }

            if i >= bytes.len() && bytes[start] == b'{' {
                break;
            }
        } else {
            i += 1;
        }
    }

    count
}
