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

pub mod backend;
pub mod codegen;
pub mod expression;
pub mod importgen;
pub mod statement;

pub fn backend() -> Option<String> {
    let (major, minor, patch) = (0_u32, 0_u32, 0_u32);
    Some(format!("LLVM {}.{}.{}", major, minor, patch))
}
