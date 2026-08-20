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

//! Canonical architecture names used by target-attribute preprocessing.
//!
//! Common host/toolchain aliases normalize to stable Wave spellings. Unknown
//! values are preserved in lowercase so future targets can still participate in
//! string-based conditions before receiving a dedicated enum variant.

mod aarch64;
mod riscv64;
mod x86_64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    X86_64,
    Aarch64,
    Riscv64,
}

impl Architecture {
    pub const fn name(self) -> &'static str {
        match self {
            Self::X86_64 => x86_64::NAME,
            Self::Aarch64 => aarch64::NAME,
            Self::Riscv64 => riscv64::NAME,
        }
    }

    pub fn from_name(value: &str) -> Option<Self> {
        let value = value.trim().to_ascii_lowercase();
        if x86_64::recognizes(&value) {
            Some(Self::X86_64)
        } else if aarch64::recognizes(&value) {
            Some(Self::Aarch64)
        } else if riscv64::recognizes(&value) {
            Some(Self::Riscv64)
        } else {
            None
        }
    }
}

pub fn canonical_name(value: &str) -> String {
    Architecture::from_name(value)
        .map(|arch| arch.name().to_string())
        .unwrap_or_else(|| value.trim().to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn architecture_aliases_have_stable_canonical_names() {
        for (input, expected) in [
            ("x86_64", "x86_64"),
            ("AMD64", "x86_64"),
            ("aarch64", "aarch64"),
            ("arm64", "aarch64"),
            ("riscv64", "riscv64"),
            ("unknown-arch", "unknown-arch"),
        ] {
            assert_eq!(canonical_name(input), expected);
        }
    }
}
