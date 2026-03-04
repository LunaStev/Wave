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

use std::fs;
use std::path::Path;
use std::process::Command;

fn normalize_clang_opt_flag(opt_flag: &str) -> &str {
    match opt_flag {
        // LLVM pass pipeline currently has no dedicated Ofast preset, so keep
        // frontend/back-end optimization behavior aligned at O3.
        "-Ofast" => "-O3",
        other => other,
    }
}

pub fn compile_ir_to_object(ir: &str, file_stem: &str, opt_flag: &str) -> String {
    let object_path = format!("{}.o", file_stem);

    let normalized_opt = normalize_clang_opt_flag(opt_flag);
    let mut cmd = Command::new("clang");

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

pub fn link_objects(objects: &[String], output: &str, libs: &[String], lib_paths: &[String]) {
    let mut cmd = Command::new("clang");

    for obj in objects {
        cmd.arg(obj);
    }

    for path in lib_paths {
        cmd.arg(format!("-L{}", path));
    }

    for lib in libs {
        cmd.arg(format!("-l{}", lib));
    }

    cmd.arg("-o").arg(output).arg("-lc").arg("-lm");

    let output = cmd.output().expect("Failed to link");
    if !output.status.success() {
        eprintln!("link failed: {}", String::from_utf8_lossy(&output.stderr));
    }
}
