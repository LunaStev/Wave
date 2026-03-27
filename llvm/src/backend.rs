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

use std::process::Command;

#[derive(Debug, Default, Clone)]
pub struct BackendOptions {
    pub target: Option<String>,
    pub cpu: Option<String>,
    pub features: Option<String>,
    pub abi: Option<String>,
    pub code_model: Option<String>,
    pub relocation_model: Option<String>,
    pub sysroot: Option<String>,
    pub linker: Option<String>,
    pub link_args: Vec<String>,
    pub no_default_libs: bool,
}

fn normalize_clang_opt_flag(opt_flag: &str) -> &str {
    match opt_flag {
        // LLVM pass pipeline currently has no dedicated Ofast preset, so keep
        // frontend/back-end optimization behavior aligned at O3.
        "-Ofast" => "-O3",
        other => other,
    }
}

pub fn compile_ir_to_object(
    ir: &str,
    file_stem: &str,
    opt_flag: &str,
    backend: &BackendOptions,
) -> String {
    let object_path = format!("{}.o", file_stem);

    let normalized_opt = normalize_clang_opt_flag(opt_flag);
    let mut cmd = Command::new("clang");

    if let Some(target) = &backend.target {
        cmd.arg(format!("--target={}", target));
    }

    if let Some(sysroot) = &backend.sysroot {
        cmd.arg(format!("--sysroot={}", sysroot));
    }

    if let Some(abi) = &backend.abi {
        cmd.arg("-target-abi").arg(abi);
    }

    if !normalized_opt.is_empty() {
        cmd.arg(normalized_opt);
    }

    let mut child = cmd
        .arg("-c")
        .arg("-x")
        .arg("ir")
        .arg("-")
        .arg("-o")
        .arg(&object_path)
        .arg("-Wno-override-module")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to execute clang");

    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(ir.as_bytes())
        .unwrap();

    let output = child.wait_with_output().unwrap();
    if !output.status.success() {
        eprintln!("clang failed: {}", String::from_utf8_lossy(&output.stderr));
        return String::new();
    }

    object_path
}

pub fn link_objects(
    objects: &[String],
    output: &str,
    libs: &[String],
    lib_paths: &[String],
    backend: &BackendOptions,
) {
    let linker_bin = backend.linker.as_deref().unwrap_or("clang");
    let mut cmd = Command::new(linker_bin);

    if backend.linker.is_none() {
        if let Some(target) = &backend.target {
            cmd.arg(format!("--target={}", target));
        }
        if let Some(sysroot) = &backend.sysroot {
            cmd.arg(format!("--sysroot={}", sysroot));
        }
        if let Some(abi) = &backend.abi {
            cmd.arg("-target-abi").arg(abi);
        }
    }

    for obj in objects {
        cmd.arg(obj);
    }

    for path in lib_paths {
        cmd.arg(format!("-L{}", path));
    }

    for lib in libs {
        cmd.arg(format!("-l{}", lib));
    }

    for arg in &backend.link_args {
        cmd.arg(arg);
    }

    cmd.arg("-o").arg(output);

    if !backend.no_default_libs {
        cmd.arg("-lc").arg("-lm");
    }

    let output = cmd.output().expect("Failed to link");
    if !output.status.success() {
        eprintln!("link failed: {}", String::from_utf8_lossy(&output.stderr));
    }
}
