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

use inkwell::module::Module;
use inkwell::targets::TargetTriple;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodegenTarget {
    LinuxX86_64,
    LinuxArm64,
    DarwinX86_64,
    DarwinArm64,
    WindowsX86_64Gnu,
    FreestandingX86_64,
    FreestandingArm64,
    FreestandingRISCV64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetSpec {
    pub triple: &'static str,
    pub codegen: CodegenTarget,
    pub arch: &'static str,
    pub vendor: &'static str,
    pub os: &'static str,
    pub env: &'static str,
    pub object_format: &'static str,
    pub hosted: bool,
    pub cpus: &'static [&'static str],
    pub features: &'static [&'static str],
    pub abis: &'static [&'static str],
}

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-x86"))]
const X86_CPUS: &[&str] = &["generic", "x86-64", "x86-64-v2", "x86-64-v3"];
#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-x86"))]
const X86_FEATURES: &[&str] = &["sse2", "sse4.1", "avx", "avx2"];

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-aarch64"))]
const AARCH64_CPUS: &[&str] = &["generic", "cortex-a53", "cortex-a72"];
#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-aarch64"))]
const DARWIN_AARCH64_CPUS: &[&str] = &["generic", "apple-m1"];
#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-aarch64"))]
const AARCH64_FEATURES: &[&str] = &["neon", "fp-armv8", "crypto"];

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-riscv"))]
const RISCV64_CPUS: &[&str] = &["generic", "generic-rv64", "rocket-rv64", "sifive-u74"];
#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-riscv"))]
const RISCV64_FEATURES: &[&str] = &["m", "a", "f", "d", "c"];
#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-riscv"))]
const RISCV64_ABIS: &[&str] = &["lp64", "lp64f", "lp64d"];

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-x86"))]
const LINUX_X86_64: TargetSpec = TargetSpec {
    triple: "x86_64-unknown-linux-gnu",
    codegen: CodegenTarget::LinuxX86_64,
    arch: "x86_64",
    vendor: "unknown",
    os: "linux",
    env: "gnu",
    object_format: "elf",
    hosted: true,
    cpus: X86_CPUS,
    features: X86_FEATURES,
    abis: &[],
};

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-x86"))]
const DARWIN_X86_64: TargetSpec = TargetSpec {
    triple: "x86_64-apple-darwin",
    codegen: CodegenTarget::DarwinX86_64,
    arch: "x86_64",
    vendor: "apple",
    os: "macos",
    env: "",
    object_format: "macho",
    hosted: true,
    cpus: X86_CPUS,
    features: X86_FEATURES,
    abis: &[],
};

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-x86"))]
const WINDOWS_W64_X86_64_GNU: TargetSpec = TargetSpec {
    triple: "x86_64-w64-windows-gnu",
    codegen: CodegenTarget::WindowsX86_64Gnu,
    arch: "x86_64",
    vendor: "w64",
    os: "windows",
    env: "gnu",
    object_format: "coff",
    hosted: true,
    cpus: X86_CPUS,
    features: X86_FEATURES,
    abis: &[],
};

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-x86"))]
const WINDOWS_PC_X86_64_GNU: TargetSpec = TargetSpec {
    triple: "x86_64-pc-windows-gnu",
    codegen: CodegenTarget::WindowsX86_64Gnu,
    arch: "x86_64",
    vendor: "pc",
    os: "windows",
    env: "gnu",
    object_format: "coff",
    hosted: true,
    cpus: X86_CPUS,
    features: X86_FEATURES,
    abis: &[],
};

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-x86"))]
const FREESTANDING_X86_64: TargetSpec = TargetSpec {
    triple: "x86_64-unknown-none-elf",
    codegen: CodegenTarget::FreestandingX86_64,
    arch: "x86_64",
    vendor: "unknown",
    os: "none",
    env: "none",
    object_format: "elf",
    hosted: false,
    cpus: X86_CPUS,
    features: X86_FEATURES,
    abis: &[],
};

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-aarch64"))]
const LINUX_AARCH64: TargetSpec = TargetSpec {
    triple: "aarch64-unknown-linux-gnu",
    codegen: CodegenTarget::LinuxArm64,
    arch: "aarch64",
    vendor: "unknown",
    os: "linux",
    env: "gnu",
    object_format: "elf",
    hosted: true,
    cpus: AARCH64_CPUS,
    features: AARCH64_FEATURES,
    abis: &[],
};

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-aarch64"))]
const DARWIN_AARCH64: TargetSpec = TargetSpec {
    triple: "aarch64-apple-darwin",
    codegen: CodegenTarget::DarwinArm64,
    arch: "aarch64",
    vendor: "apple",
    os: "macos",
    env: "",
    object_format: "macho",
    hosted: true,
    cpus: DARWIN_AARCH64_CPUS,
    features: AARCH64_FEATURES,
    abis: &[],
};

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-aarch64"))]
const FREESTANDING_AARCH64: TargetSpec = TargetSpec {
    triple: "aarch64-unknown-none-elf",
    codegen: CodegenTarget::FreestandingArm64,
    arch: "aarch64",
    vendor: "unknown",
    os: "none",
    env: "none",
    object_format: "elf",
    hosted: false,
    cpus: AARCH64_CPUS,
    features: AARCH64_FEATURES,
    abis: &[],
};

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-riscv"))]
const FREESTANDING_RISCV64: TargetSpec = TargetSpec {
    triple: "riscv64-unknown-none-elf",
    codegen: CodegenTarget::FreestandingRISCV64,
    arch: "riscv64",
    vendor: "unknown",
    os: "none",
    env: "none",
    object_format: "elf",
    hosted: false,
    cpus: RISCV64_CPUS,
    features: RISCV64_FEATURES,
    abis: RISCV64_ABIS,
};

pub fn supported_target_specs() -> Vec<&'static TargetSpec> {
    let mut specs: Vec<&'static TargetSpec> = Vec::new();

    #[cfg(any(feature = "llvm-target-all", feature = "llvm-target-x86"))]
    specs.extend([
        &LINUX_X86_64,
        &DARWIN_X86_64,
        &WINDOWS_W64_X86_64_GNU,
        &WINDOWS_PC_X86_64_GNU,
        &FREESTANDING_X86_64,
    ]);

    #[cfg(any(feature = "llvm-target-all", feature = "llvm-target-aarch64"))]
    specs.extend([&LINUX_AARCH64, &DARWIN_AARCH64, &FREESTANDING_AARCH64]);

    #[cfg(any(feature = "llvm-target-all", feature = "llvm-target-riscv"))]
    specs.push(&FREESTANDING_RISCV64);

    specs.sort_unstable_by_key(|spec| spec.triple);
    specs
}

pub fn target_spec_for_triple(triple: &str) -> Option<&'static TargetSpec> {
    supported_target_specs()
        .into_iter()
        .find(|spec| spec.triple == triple)
}

impl CodegenTarget {
    pub fn from_triple_str(triple: &str) -> Option<Self> {
        target_spec_for_triple(triple).map(|spec| spec.codegen)
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
            Self::LinuxArm64 => "linux arm64",
            Self::DarwinX86_64 => "darwin x86_64",
            Self::DarwinArm64 => "darwin arm64",
            Self::WindowsX86_64Gnu => "windows x86_64 gnu",
            Self::FreestandingX86_64 => "freestanding x86_64",
            Self::FreestandingArm64 => "freestanding arm64",
            Self::FreestandingRISCV64 => "freestanding riscv64",
        }
    }
}

pub fn require_supported_target_from_triple(triple: &TargetTriple) -> CodegenTarget {
    if let Some(t) = CodegenTarget::from_target_triple(triple) {
        return t;
    }

    let raw = triple.as_str().to_string_lossy();
    let supported = supported_target_specs()
        .into_iter()
        .map(|spec| spec.triple)
        .collect::<Vec<_>>()
        .join(", ");
    panic!(
        "unsupported target triple '{}': Wave currently supports {}",
        raw, supported
    );
}

pub fn require_supported_target_from_module(module: &Module<'_>) -> CodegenTarget {
    if let Some(t) = CodegenTarget::from_module(module) {
        return t;
    }

    let triple = module.get_triple();
    let raw = triple.as_str().to_string_lossy();
    let supported = supported_target_specs()
        .into_iter()
        .map(|spec| spec.triple)
        .collect::<Vec<_>>()
        .join(", ");
    panic!(
        "unsupported target triple '{}': Wave currently supports {}",
        raw, supported
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_targets_round_trip_through_exact_lookup() {
        let specs = supported_target_specs();
        assert!(!specs.is_empty());

        for (index, spec) in specs.iter().enumerate() {
            assert_eq!(target_spec_for_triple(spec.triple), Some(*spec));
            assert_eq!(
                CodegenTarget::from_triple_str(spec.triple),
                Some(spec.codegen)
            );
            assert!(!spec.arch.is_empty());
            assert!(!spec.os.is_empty());
            assert!(!spec.object_format.is_empty());

            for other in specs.iter().skip(index + 1) {
                assert_ne!(spec.triple, other.triple, "duplicate target triple");
            }
        }
    }

    #[test]
    fn malformed_or_unregistered_triples_do_not_match_by_substring() {
        for triple in [
            "x86_64-garbage-linux-gnu",
            "prefix-x86_64-unknown-linux-gnu-suffix",
            "riscv64-unknown-linux-gnu",
            "x86_64-unknown-none-elf-waveabi",
            "",
        ] {
            assert_eq!(target_spec_for_triple(triple), None, "{triple}");
            assert_eq!(CodegenTarget::from_triple_str(triple), None, "{triple}");
        }
    }
}
