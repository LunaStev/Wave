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

#[derive(Default, Clone, Copy)]
pub struct DebugFlags {
    pub tokens: bool,
    pub ast: bool,
    pub ir: bool,
    pub mc: bool,
    pub hex: bool,
}

impl DebugFlags {
    /// --debug-wave=tokens
    /// --debug-wave=tokens,ir
    /// --debug-wave=all
    pub fn apply(&mut self, mode: &str) {
        if mode.trim().is_empty() {
            return;
        }

        for item in mode.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
            match item {
                "tokens" => self.tokens = true,
                "ast" => self.ast = true,
                "ir" => self.ir = true,
                "mc" => self.mc = true,
                "hex" => self.hex = true,
                "all" => {
                    self.tokens = true;
                    self.ast = true;
                    self.ir = true;
                    self.mc = true;
                    self.hex = true;
                }
                _ => {}
            }
        }
    }
}

#[derive(Default, Clone)]
pub struct LinkFlags {
    pub libs: Vec<String>,
    pub paths: Vec<String>,
}

pub fn validate_opt_flag(flag: &str) -> bool {
    matches!(flag, "-O0" | "-O1" | "-O2" | "-O3" | "-Os" | "-Oz" | "-Ofast")
}
