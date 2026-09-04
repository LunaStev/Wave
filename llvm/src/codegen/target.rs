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

//! Supported-target registry and target-option contract validation.
//!
//! A [`TargetSpec`] is the single source of truth shared by CLI validation,
//! LLVM target-machine creation, inline assembly, and C ABI classification.
//! Keep exact triples here rather than accepting an architecture prefix: OS,
//! environment, and object format are ABI-relevant parts of a target.

use inkwell::module::Module;
use inkwell::targets::TargetTriple;
use std::collections::{BTreeMap, BTreeSet};

use super::arch::{self, Architecture};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodegenTarget {
    LinuxX86_64,
    LinuxArm64,
    LinuxRISCV64,
    DarwinX86_64,
    DarwinArm64,
    WindowsX86_64Gnu,
    WindowsArm64Gnu,
    FreeBsdX86_64,
    FreestandingX86_64,
    FreestandingArm64,
    FreestandingRISCV64,
    Wasm32Unknown,
    Wasm32WasiP1,
    Wasm64Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetSpec {
    pub triple: &'static str,
    pub codegen: CodegenTarget,
    pub architecture: Architecture,
    pub vendor: &'static str,
    pub os: &'static str,
    pub env: &'static str,
    pub object_format: &'static str,
    pub hosted: bool,
    pub cpus: &'static [&'static str],
    pub features: &'static [&'static str],
    pub abis: &'static [&'static str],
    pub default_cpu: &'static str,
    pub default_features: &'static [&'static str],
    pub default_abi: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveTargetOptions {
    pub cpu: String,
    pub features: String,
    pub abi: Option<String>,
    pub isa: Option<String>,
}

/// Resolves user overrides into the complete option set passed to LLVM.
///
/// RISC-V ABI, floating-point extensions, and ISA spelling are coupled. This
/// function validates that contract once so later codegen does not need to
/// reinterpret partially specified options.
pub fn resolve_target_options(
    spec: &TargetSpec,
    cpu: Option<&str>,
    features: Option<&str>,
    abi: Option<&str>,
) -> Result<EffectiveTargetOptions, String> {
    let cpu = cpu.unwrap_or(spec.default_cpu);
    if !spec.cpus.contains(&cpu) {
        return Err(format!(
            "unsupported CPU '{}' for target '{}'; supported CPUs: {}",
            cpu,
            spec.triple,
            spec.cpus.join(", ")
        ));
    }

    if let Some(abi) = abi {
        if !spec.abis.contains(&abi) {
            let supported = if spec.abis.is_empty() {
                "no ABI overrides".to_string()
            } else {
                spec.abis.join(", ")
            };
            return Err(format!(
                "unsupported ABI '{}' for target '{}'; supported ABIs: {}",
                abi, spec.triple, supported
            ));
        }
    }

    let mut enabled = spec
        .features
        .iter()
        .copied()
        .map(|name| (name, spec.default_features.contains(&name)))
        .collect::<BTreeMap<_, _>>();

    if spec.architecture == Architecture::Riscv64 {
        // The ABI establishes the initial F/D state. Explicit feature settings
        // are applied afterward and must still describe the same ABI.
        match abi.or(spec.default_abi) {
            Some("lp64") => {
                enabled.insert("f", false);
                enabled.insert("d", false);
            }
            Some("lp64f") => {
                enabled.insert("f", true);
                enabled.insert("d", false);
            }
            Some("lp64d") => {
                enabled.insert("f", true);
                enabled.insert("d", true);
            }
            _ => {}
        }
    }

    let mut explicitly_set = BTreeSet::new();
    if let Some(features) = features {
        for raw in features.split(',') {
            let setting = raw.trim();
            if setting.is_empty() {
                return Err(format!(
                    "invalid empty target feature in '{}' for target '{}'",
                    features, spec.triple
                ));
            }
            let (value, name) = if let Some(name) = setting.strip_prefix('+') {
                (true, name)
            } else if let Some(name) = setting.strip_prefix('-') {
                (false, name)
            } else {
                return Err(format!(
                    "invalid target feature '{}'; use '+feature' to enable or '-feature' to disable it",
                    setting
                ));
            };
            if name.is_empty() || !spec.features.contains(&name) {
                return Err(format!(
                    "unsupported feature '{}' for target '{}'; supported features: {}",
                    name,
                    spec.triple,
                    spec.features.join(", ")
                ));
            }
            if !explicitly_set.insert(name) {
                return Err(format!(
                    "target feature '{}' is specified more than once for target '{}'",
                    name, spec.triple
                ));
            }
            enabled.insert(name, value);
        }
    }

    let mut effective_abi = abi.or(spec.default_abi).map(str::to_string);
    let mut isa = None;
    if spec.architecture == Architecture::Riscv64 {
        // LLVM requires the CSR extension for floating-point instructions. Add
        // it implicitly unless the user explicitly disabled it, in which case
        // the consistency check below produces a useful error.
        if enabled.get("f").copied().unwrap_or(false) && !explicitly_set.contains("zicsr") {
            enabled.insert("zicsr", true);
        }
        let feature = |name| enabled.get(name).copied().unwrap_or(false);
        if feature("d") && !feature("f") {
            return Err(format!(
                "invalid feature combination for target '{}': feature 'd' requires feature 'f'",
                spec.triple
            ));
        }
        if feature("f") && !feature("zicsr") {
            return Err(format!(
                "invalid feature combination for target '{}': feature 'f' requires feature 'zicsr'",
                spec.triple
            ));
        }

        let derived_abi = if feature("d") {
            "lp64d"
        } else if feature("f") {
            "lp64f"
        } else {
            "lp64"
        };
        if let Some(requested) = abi {
            if requested != derived_abi {
                let requirement = match requested {
                    "lp64" => "features 'f' and 'd' to be disabled",
                    "lp64f" => "feature 'f' enabled and feature 'd' disabled",
                    "lp64d" => "features 'f' and 'd' enabled",
                    _ => unreachable!(),
                };
                return Err(format!(
                    "ABI '{}' for target '{}' requires {}",
                    requested, spec.triple, requirement
                ));
            }
        } else {
            effective_abi = Some(derived_abi.to_string());
        }

        isa = Some(arch::riscv64::isa_name(
            feature("m"),
            feature("a"),
            feature("f"),
            feature("d"),
            feature("c"),
            feature("zicsr"),
            feature("zifencei"),
        ));
    }

    // RISC-V passes every supported feature with an explicit sign. Omitting a
    // disabled F/D feature can let LLVM's CPU defaults silently contradict the
    // effective ABI.
    let render_all_features = spec.architecture == Architecture::Riscv64;
    let features = spec
        .features
        .iter()
        .filter(|name| {
            render_all_features
                || explicitly_set.contains(**name)
                || spec.default_features.contains(name)
        })
        .map(|name| {
            let sign = if enabled.get(name).copied().unwrap_or(false) {
                '+'
            } else {
                '-'
            };
            format!("{}{}", sign, name)
        })
        .collect::<Vec<_>>()
        .join(",");

    Ok(EffectiveTargetOptions {
        cpu: cpu.to_string(),
        features,
        abi: effective_abi,
        isa,
    })
}

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-x86"))]
const LINUX_X86_64: TargetSpec = TargetSpec {
    triple: "x86_64-unknown-linux-gnu",
    codegen: CodegenTarget::LinuxX86_64,
    architecture: Architecture::X86_64,
    vendor: "unknown",
    os: "linux",
    env: "gnu",
    object_format: "elf",
    hosted: true,
    cpus: arch::x86_64::CPUS,
    features: arch::x86_64::FEATURES,
    abis: &[],
    default_cpu: arch::x86_64::DEFAULT_CPU,
    default_features: arch::x86_64::DEFAULT_FEATURES,
    default_abi: None,
};

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-x86"))]
const DARWIN_X86_64: TargetSpec = TargetSpec {
    triple: "x86_64-apple-darwin",
    codegen: CodegenTarget::DarwinX86_64,
    architecture: Architecture::X86_64,
    vendor: "apple",
    os: "macos",
    env: "",
    object_format: "macho",
    hosted: true,
    cpus: arch::x86_64::CPUS,
    features: arch::x86_64::FEATURES,
    abis: &[],
    default_cpu: arch::x86_64::DEFAULT_CPU,
    default_features: arch::x86_64::DEFAULT_FEATURES,
    default_abi: None,
};

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-x86"))]
const WINDOWS_W64_X86_64_GNU: TargetSpec = TargetSpec {
    triple: "x86_64-w64-windows-gnu",
    codegen: CodegenTarget::WindowsX86_64Gnu,
    architecture: Architecture::X86_64,
    vendor: "w64",
    os: "windows",
    env: "gnu",
    object_format: "coff",
    hosted: true,
    cpus: arch::x86_64::CPUS,
    features: arch::x86_64::FEATURES,
    abis: &[],
    default_cpu: arch::x86_64::DEFAULT_CPU,
    default_features: arch::x86_64::DEFAULT_FEATURES,
    default_abi: None,
};

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-x86"))]
const WINDOWS_PC_X86_64_GNU: TargetSpec = TargetSpec {
    triple: "x86_64-pc-windows-gnu",
    codegen: CodegenTarget::WindowsX86_64Gnu,
    architecture: Architecture::X86_64,
    vendor: "pc",
    os: "windows",
    env: "gnu",
    object_format: "coff",
    hosted: true,
    cpus: arch::x86_64::CPUS,
    features: arch::x86_64::FEATURES,
    abis: &[],
    default_cpu: arch::x86_64::DEFAULT_CPU,
    default_features: arch::x86_64::DEFAULT_FEATURES,
    default_abi: None,
};

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-x86"))]
const FREEBSD_X86_64: TargetSpec = TargetSpec {
    triple: "x86_64-unknown-freebsd",
    codegen: CodegenTarget::FreeBsdX86_64,
    architecture: Architecture::X86_64,
    vendor: "unknown",
    os: "freebsd",
    env: "",
    object_format: "elf",
    hosted: true,
    cpus: arch::x86_64::CPUS,
    features: arch::x86_64::FEATURES,
    abis: &[],
    default_cpu: arch::x86_64::DEFAULT_CPU,
    default_features: arch::x86_64::DEFAULT_FEATURES,
    default_abi: None,
};

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-x86"))]
const FREESTANDING_X86_64: TargetSpec = TargetSpec {
    triple: "x86_64-unknown-none-elf",
    codegen: CodegenTarget::FreestandingX86_64,
    architecture: Architecture::X86_64,
    vendor: "unknown",
    os: "none",
    env: "none",
    object_format: "elf",
    hosted: false,
    cpus: arch::x86_64::CPUS,
    features: arch::x86_64::FEATURES,
    abis: &[],
    default_cpu: arch::x86_64::DEFAULT_CPU,
    default_features: arch::x86_64::DEFAULT_FEATURES,
    default_abi: None,
};

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-aarch64"))]
const LINUX_AARCH64: TargetSpec = TargetSpec {
    triple: "aarch64-unknown-linux-gnu",
    codegen: CodegenTarget::LinuxArm64,
    architecture: Architecture::Aarch64,
    vendor: "unknown",
    os: "linux",
    env: "gnu",
    object_format: "elf",
    hosted: true,
    cpus: arch::aarch64::CPUS,
    features: arch::aarch64::FEATURES,
    abis: &[],
    default_cpu: arch::aarch64::DEFAULT_CPU,
    default_features: arch::aarch64::DEFAULT_FEATURES,
    default_abi: None,
};

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-aarch64"))]
const DARWIN_AARCH64: TargetSpec = TargetSpec {
    triple: "aarch64-apple-darwin",
    codegen: CodegenTarget::DarwinArm64,
    architecture: Architecture::Aarch64,
    vendor: "apple",
    os: "macos",
    env: "",
    object_format: "macho",
    hosted: true,
    cpus: arch::aarch64::DARWIN_CPUS,
    features: arch::aarch64::FEATURES,
    abis: &[],
    default_cpu: arch::aarch64::DEFAULT_CPU,
    default_features: arch::aarch64::DEFAULT_FEATURES,
    default_abi: None,
};

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-aarch64"))]
const WINDOWS_AARCH64_GNU: TargetSpec = TargetSpec {
    triple: "aarch64-w64-windows-gnu",
    codegen: CodegenTarget::WindowsArm64Gnu,
    architecture: Architecture::Aarch64,
    vendor: "w64",
    os: "windows",
    env: "gnu",
    object_format: "coff",
    hosted: true,
    cpus: arch::aarch64::CPUS,
    features: arch::aarch64::FEATURES,
    abis: &[],
    default_cpu: arch::aarch64::DEFAULT_CPU,
    default_features: arch::aarch64::DEFAULT_FEATURES,
    default_abi: None,
};

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-aarch64"))]
const FREESTANDING_AARCH64: TargetSpec = TargetSpec {
    triple: "aarch64-unknown-none-elf",
    codegen: CodegenTarget::FreestandingArm64,
    architecture: Architecture::Aarch64,
    vendor: "unknown",
    os: "none",
    env: "none",
    object_format: "elf",
    hosted: false,
    cpus: arch::aarch64::CPUS,
    features: arch::aarch64::FEATURES,
    abis: &[],
    default_cpu: arch::aarch64::DEFAULT_CPU,
    default_features: arch::aarch64::DEFAULT_FEATURES,
    default_abi: None,
};

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-riscv"))]
const LINUX_RISCV64: TargetSpec = TargetSpec {
    triple: "riscv64-unknown-linux-gnu",
    codegen: CodegenTarget::LinuxRISCV64,
    architecture: Architecture::Riscv64,
    vendor: "unknown",
    os: "linux",
    env: "gnu",
    object_format: "elf",
    hosted: true,
    cpus: arch::riscv64::CPUS,
    features: arch::riscv64::FEATURES,
    abis: arch::riscv64::ABIS,
    default_cpu: arch::riscv64::DEFAULT_CPU,
    default_features: arch::riscv64::LINUX_DEFAULT_FEATURES,
    default_abi: Some(arch::riscv64::LINUX_DEFAULT_ABI),
};

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-riscv"))]
const FREESTANDING_RISCV64: TargetSpec = TargetSpec {
    triple: "riscv64-unknown-none-elf",
    codegen: CodegenTarget::FreestandingRISCV64,
    architecture: Architecture::Riscv64,
    vendor: "unknown",
    os: "none",
    env: "none",
    object_format: "elf",
    hosted: false,
    cpus: arch::riscv64::CPUS,
    features: arch::riscv64::FEATURES,
    abis: arch::riscv64::ABIS,
    default_cpu: arch::riscv64::DEFAULT_CPU,
    default_features: arch::riscv64::FREESTANDING_DEFAULT_FEATURES,
    default_abi: Some(arch::riscv64::FREESTANDING_DEFAULT_ABI),
};

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-wasm"))]
const WASM32_UNKNOWN_UNKNOWN: TargetSpec = TargetSpec {
    triple: "wasm32-unknown-unknown",
    codegen: CodegenTarget::Wasm32Unknown,
    architecture: Architecture::Wasm32,
    vendor: "unknown",
    os: "unknown",
    env: "",
    object_format: "wasm",
    hosted: false,
    cpus: arch::wasm::CPUS,
    features: arch::wasm::FEATURES,
    abis: &[],
    default_cpu: arch::wasm::DEFAULT_CPU,
    default_features: arch::wasm::DEFAULT_FEATURES,
    default_abi: None,
};

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-wasm"))]
const WASM32_WASIP1: TargetSpec = TargetSpec {
    triple: "wasm32-wasip1",
    codegen: CodegenTarget::Wasm32WasiP1,
    architecture: Architecture::Wasm32,
    vendor: "unknown",
    os: "wasi",
    env: "p1",
    object_format: "wasm",
    hosted: true,
    cpus: arch::wasm::CPUS,
    features: arch::wasm::FEATURES,
    abis: &[],
    default_cpu: arch::wasm::DEFAULT_CPU,
    default_features: arch::wasm::DEFAULT_FEATURES,
    default_abi: None,
};

#[cfg(any(feature = "llvm-target-all", feature = "llvm-target-wasm"))]
const WASM64_UNKNOWN_UNKNOWN: TargetSpec = TargetSpec {
    triple: "wasm64-unknown-unknown",
    codegen: CodegenTarget::Wasm64Unknown,
    architecture: Architecture::Wasm64,
    vendor: "unknown",
    os: "unknown",
    env: "",
    object_format: "wasm",
    hosted: false,
    cpus: arch::wasm::CPUS,
    features: arch::wasm::FEATURES,
    abis: &[],
    default_cpu: arch::wasm::DEFAULT_CPU,
    default_features: arch::wasm::DEFAULT_FEATURES,
    default_abi: None,
};

/// Returns the targets compiled into this backend, in deterministic order.
pub fn supported_target_specs() -> Vec<&'static TargetSpec> {
    let mut specs: Vec<&'static TargetSpec> = Vec::new();

    #[cfg(any(feature = "llvm-target-all", feature = "llvm-target-x86"))]
    specs.extend([
        &LINUX_X86_64,
        &DARWIN_X86_64,
        &WINDOWS_W64_X86_64_GNU,
        &WINDOWS_PC_X86_64_GNU,
        &FREEBSD_X86_64,
        &FREESTANDING_X86_64,
    ]);

    #[cfg(any(feature = "llvm-target-all", feature = "llvm-target-aarch64"))]
    specs.extend([
        &LINUX_AARCH64,
        &DARWIN_AARCH64,
        &WINDOWS_AARCH64_GNU,
        &FREESTANDING_AARCH64,
    ]);

    #[cfg(any(feature = "llvm-target-all", feature = "llvm-target-riscv"))]
    specs.extend([&LINUX_RISCV64, &FREESTANDING_RISCV64]);

    #[cfg(any(feature = "llvm-target-all", feature = "llvm-target-wasm"))]
    specs.extend([
        &WASM32_UNKNOWN_UNKNOWN,
        &WASM32_WASIP1,
        &WASM64_UNKNOWN_UNKNOWN,
    ]);

    specs.sort_unstable_by_key(|spec| spec.triple);
    specs
}

/// Performs an exact lookup in the compiled target registry.
pub fn target_spec_for_triple(triple: &str) -> Option<&'static TargetSpec> {
    supported_target_specs()
        .into_iter()
        .find(|spec| spec.triple == triple)
}

impl CodegenTarget {
    pub const fn architecture(self) -> Architecture {
        match self {
            Self::LinuxX86_64
            | Self::DarwinX86_64
            | Self::WindowsX86_64Gnu
            | Self::FreeBsdX86_64
            | Self::FreestandingX86_64 => Architecture::X86_64,
            Self::LinuxArm64
            | Self::DarwinArm64
            | Self::WindowsArm64Gnu
            | Self::FreestandingArm64 => Architecture::Aarch64,
            Self::LinuxRISCV64 | Self::FreestandingRISCV64 => Architecture::Riscv64,
            Self::Wasm32Unknown | Self::Wasm32WasiP1 => Architecture::Wasm32,
            Self::Wasm64Unknown => Architecture::Wasm64,
        }
    }

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
            Self::WindowsArm64Gnu => "windows arm64 gnu",
            Self::FreeBsdX86_64 => "freebsd x86_64",
            Self::FreestandingX86_64 => "freestanding x86_64",
            Self::FreestandingArm64 => "freestanding arm64",
            Self::LinuxRISCV64 => "linux riscv64",
            Self::FreestandingRISCV64 => "freestanding riscv64",
            Self::Wasm32Unknown => "webassembly wasm32 unknown",
            Self::Wasm32WasiP1 => "webassembly wasm32 WASI Preview 1",
            Self::Wasm64Unknown => "webassembly wasm64 unknown",
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
            assert!(!spec.architecture.name().is_empty());
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
            "riscv64-unknown-linux-musl",
            "x86_64-unknown-none-elf-waveabi",
            "",
        ] {
            assert_eq!(target_spec_for_triple(triple), None, "{triple}");
            assert_eq!(CodegenTarget::from_triple_str(triple), None, "{triple}");
        }
    }

    #[cfg(any(feature = "llvm-target-all", feature = "llvm-target-riscv"))]
    #[test]
    fn riscv64_defaults_define_distinct_hosted_and_freestanding_contracts() {
        let linux = resolve_target_options(&LINUX_RISCV64, None, None, None).unwrap();
        assert_eq!(linux.cpu, "generic-rv64");
        assert_eq!(linux.features, "+m,+a,+f,+d,+c,+zicsr,+zifencei");
        assert_eq!(linux.abi.as_deref(), Some("lp64d"));
        assert_eq!(linux.isa.as_deref(), Some("rv64gc"));

        let freestanding = resolve_target_options(&FREESTANDING_RISCV64, None, None, None).unwrap();
        assert_eq!(freestanding.cpu, "generic-rv64");
        assert_eq!(freestanding.features, "+m,+a,-f,-d,+c,-zicsr,-zifencei");
        assert_eq!(freestanding.abi.as_deref(), Some("lp64"));
        assert_eq!(freestanding.isa.as_deref(), Some("rv64imac"));
    }

    #[cfg(any(feature = "llvm-target-all", feature = "llvm-target-riscv"))]
    #[test]
    fn riscv64_feature_overrides_derive_or_validate_the_float_abi() {
        let derived =
            resolve_target_options(&FREESTANDING_RISCV64, None, Some("+f,-d"), None).unwrap();
        assert_eq!(derived.abi.as_deref(), Some("lp64f"));
        assert_eq!(derived.isa.as_deref(), Some("rv64imafc_zicsr"));

        let error =
            resolve_target_options(&FREESTANDING_RISCV64, None, Some("+f,-d"), Some("lp64d"))
                .unwrap_err();
        assert!(error.contains("requires features 'f' and 'd' enabled"));

        let error = resolve_target_options(&LINUX_RISCV64, None, Some("-f"), None).unwrap_err();
        assert!(error.contains("feature 'd' requires feature 'f'"));
    }
}
