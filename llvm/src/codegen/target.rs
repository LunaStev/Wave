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

use inkwell::module::Module;
use inkwell::targets::TargetTriple;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodegenTarget {
    LinuxX86_64,
    DarwinArm64,
}

impl CodegenTarget {
    pub fn from_triple_str(triple: &str) -> Option<Self> {
        let t = triple.to_ascii_lowercase();

        let is_x86_64 = t.starts_with("x86_64");
        let is_arm64 = t.starts_with("arm64") || t.starts_with("aarch64");
        let is_linux = t.contains("linux");
        let is_darwin = t.contains("darwin");

        if is_x86_64 && is_linux {
            return Some(Self::LinuxX86_64);
        }
        if is_arm64 && is_darwin {
            return Some(Self::DarwinArm64);
        }

        None
    }

    pub fn from_target_triple(triple: &TargetTriple) -> Option<Self> {
        let raw = triple.as_str().to_string_lossy();
        Self::from_triple_str(raw.as_ref())
    }

    pub fn from_module(module: &Module<'_>) -> Option<Self> {
        let triple = module.get_triple();
        Self::from_target_triple(&triple)
    }

    pub fn desc(self) -> &'static str {
        match self {
            Self::LinuxX86_64 => "linux x86_64",
            Self::DarwinArm64 => "darwin arm64",
        }
    }
}

pub fn require_supported_target_from_triple(triple: &TargetTriple) -> CodegenTarget {
    if let Some(t) = CodegenTarget::from_target_triple(triple) {
        return t;
    }

    let raw = triple.as_str().to_string_lossy();
    panic!(
        "unsupported target triple '{}': Wave currently supports only linux x86_64 and darwin arm64 (Windows not supported yet)",
        raw
    );
}

pub fn require_supported_target_from_module(module: &Module<'_>) -> CodegenTarget {
    if let Some(t) = CodegenTarget::from_module(module) {
        return t;
    }

    let triple = module.get_triple();
    let raw = triple.as_str().to_string_lossy();
    panic!(
        "unsupported target triple '{}': Wave currently supports only linux x86_64 and darwin arm64 (Windows not supported yet)",
        raw
    );
}
