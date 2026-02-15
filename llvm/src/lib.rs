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

pub mod expression;
pub mod backend;
pub mod codegen;
pub mod statement;
pub mod importgen;


pub fn backend() -> Option<String> {
    unsafe {
        let mut major: u32 = 0;
        let mut minor: u32 = 0;
        let mut patch: u32 = 0;

        Some(format!("LLVM {}.{}.{}", major, minor, patch))
    }
}

