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

//! WebAssembly target options and the deliberately empty native-register set.
//!
//! Wave inline assembly names physical registers, while WebAssembly exposes a
//! stack machine rather than a native register file. Returning no registers
//! makes existing inline-assembly validation reject such blocks for wasm32
//! and wasm64.

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-wasm"))]
pub(crate) const CPUS: &[&str] = &["generic"];
#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-wasm"))]
pub(crate) const FEATURES: &[&str] = &[
    "bulk-memory",
    "mutable-globals",
    "nontrapping-fptoint",
    "reference-types",
    "sign-ext",
    "simd128",
];
#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-wasm"))]
pub(crate) const DEFAULT_CPU: &str = "generic";
#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-wasm"))]
pub(crate) const DEFAULT_FEATURES: &[&str] = &[];

pub(crate) fn register_group(_token: &str) -> Option<String> {
    None
}

pub(crate) fn operand_register_group(_token: &str) -> Option<String> {
    None
}

pub(crate) fn register_width_bits(_token: &str) -> Option<u32> {
    None
}

pub(crate) fn default_clobbers() -> Vec<String> {
    Vec::new()
}

pub(crate) fn allocatable_registers() -> Vec<String> {
    Vec::new()
}

pub(crate) fn normalize_special_clobber(_token: &str) -> Option<String> {
    None
}

pub(crate) fn stack_analysis(line: &str) -> super::StackAnalysis {
    let mut out = super::StackAnalysis::default();
    if !line.trim().is_empty() {
        out.unknown_stack_write = true;
    }
    out
}
