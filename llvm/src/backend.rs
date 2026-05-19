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

use std::env;
use std::path::PathBuf;
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
    pub freestanding: bool,
}

fn is_windows_gnu_target(target: Option<&str>) -> bool {
    let Some(target) = target else {
        return false;
    };
    let t = target.to_ascii_lowercase();
    t.starts_with("x86_64-") && t.contains("windows") && !t.contains("msvc")
}

fn normalize_llvm_opt_flag(opt_flag: &str) -> &str {
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

    let normalized_opt = normalize_llvm_opt_flag(opt_flag);
    let llc = resolve_bundled_tool("llc");
    let mut cmd = Command::new(&llc);
    configure_bundled_llvm_tool_env(&mut cmd, &llc);

    if let Some(target) = &backend.target {
        cmd.arg(format!("--mtriple={}", target));
    }
    if let Some(cpu) = &backend.cpu {
        cmd.arg(format!("--mcpu={}", cpu));
    }
    if let Some(features) = &backend.features {
        cmd.arg(format!("--mattr={}", features));
    }
    if let Some(model) = &backend.code_model {
        cmd.arg(format!("--code-model={}", model));
    }
    if let Some(model) = &backend.relocation_model {
        cmd.arg(format!("--relocation-model={}", model));
    }
    if let Some(abi) = &backend.abi {
        cmd.arg(format!("--target-abi={}", abi));
    }

    if !normalized_opt.is_empty() {
        cmd.arg(normalized_opt);
    }

    let mut child = cmd
        .arg("--filetype=obj")
        .arg("-")
        .arg("-o")
        .arg(&object_path)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to execute llc");

    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(ir.as_bytes())
        .unwrap();

    let output = child.wait_with_output().unwrap();
    if !output.status.success() {
        eprintln!("llc failed: {}", String::from_utf8_lossy(&output.stderr));
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
    let target = backend.target.as_deref().unwrap_or("");
    let linker_bin = backend
        .linker
        .clone()
        .unwrap_or_else(|| default_lld_for_target(target));
    let mut cmd = Command::new(&linker_bin);
    configure_bundled_llvm_tool_env(&mut cmd, &linker_bin);

    if backend.linker.is_none() && !(is_windows_gnu_target(Some(target)) && linker_bin == "gcc") {
        append_lld_target_args(&mut cmd, target, backend);
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

    for arg in expand_lld_link_args(&backend.link_args) {
        cmd.arg(arg);
    }

    cmd.arg("-o").arg(output);

    if !backend.no_default_libs {
        if is_darwin_target(target) {
            cmd.arg("-lSystem");
        } else if !is_windows_gnu_target(backend.target.as_deref()) {
            cmd.arg("-lc").arg("-lm");
        }
    }

    let output = cmd.output().expect("Failed to link");
    if !output.status.success() {
        eprintln!("link failed: {}", String::from_utf8_lossy(&output.stderr));
    }
}

fn default_lld_for_target(target: &str) -> String {
    if is_darwin_target(target) {
        resolve_bundled_tool("ld64.lld")
    } else if is_windows_gnu_target(Some(target)) {
        resolve_bundled_tool_path("ld.lld")
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_else(|| "gcc".to_string())
    } else {
        resolve_bundled_tool("ld.lld")
    }
}

fn append_lld_target_args(cmd: &mut Command, target: &str, backend: &BackendOptions) {
    if is_darwin_target(target) {
        cmd.arg("-arch")
            .arg(if target.starts_with("x86_64-") {
                "x86_64"
            } else {
                "arm64"
            })
            .arg("-platform_version")
            .arg("macos")
            .arg("11.0")
            .arg("11.0");

        if let Some(sysroot) = &backend.sysroot {
            cmd.arg("-syslibroot").arg(sysroot);
        }
        return;
    }

    if is_windows_gnu_target(Some(target)) {
        cmd.arg("-m").arg("i386pep");
        return;
    }

    if let Some(emulation) = elf_lld_emulation(target) {
        cmd.arg("-m").arg(emulation);
    }
    if let Some(sysroot) = &backend.sysroot {
        cmd.arg(format!("--sysroot={}", sysroot));
    }
}

fn expand_lld_link_args(link_args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for arg in link_args {
        if arg == "-nostartfiles" {
            continue;
        }
        if let Some(rest) = arg.strip_prefix("-Wl,") {
            out.extend(
                rest.split(',')
                    .filter(|part| !part.is_empty())
                    .map(|part| part.to_string()),
            );
        } else {
            out.push(arg.clone());
        }
    }
    out
}

fn is_darwin_target(target: &str) -> bool {
    target.contains("apple-darwin")
}

fn elf_lld_emulation(target: &str) -> Option<&'static str> {
    match target.split('-').next().unwrap_or(target) {
        "x86_64" => Some("elf_x86_64"),
        "aarch64" => Some("aarch64elf"),
        "riscv64" => Some("elf64lriscv"),
        _ => None,
    }
}

fn resolve_bundled_tool(tool: &str) -> String {
    if let Some(path) = resolve_bundled_tool_path(tool) {
        return path.to_string_lossy().to_string();
    }
    executable_tool_name(tool)
}

fn resolve_bundled_tool_path(tool: &str) -> Option<PathBuf> {
    for dir in llvm_tool_search_dirs() {
        let candidate = dir.join(executable_tool_name(tool));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn configure_bundled_llvm_tool_env(cmd: &mut Command, bin: &str) {
    let Some(bin_dir) = bundled_llvm_bin_dir(bin) else {
        return;
    };

    if cfg!(target_os = "linux") {
        if let Some(lib_dir) = bin_dir.parent().map(|llvm_dir| llvm_dir.join("lib")) {
            if lib_dir.is_dir() {
                prepend_env_path(cmd, "LD_LIBRARY_PATH", lib_dir);
            }
        }
    } else if cfg!(windows) {
        if let Some(root_dir) = bin_dir.parent().and_then(|llvm_dir| llvm_dir.parent()) {
            prepend_env_path(cmd, "PATH", root_dir.to_path_buf());
        }
        prepend_env_path(cmd, "PATH", bin_dir);
    }
}

fn bundled_llvm_bin_dir(bin: &str) -> Option<PathBuf> {
    let bin_path = std::path::Path::new(bin);
    let bin_dir = bin_path.parent()?;
    if bin_dir.file_name().and_then(|name| name.to_str()) != Some("bin") {
        return None;
    }

    let llvm_dir = bin_dir.parent()?;
    if llvm_dir.file_name().and_then(|name| name.to_str()) != Some("llvm") {
        return None;
    }

    Some(bin_dir.to_path_buf())
}

fn prepend_env_path(cmd: &mut Command, name: &str, first: PathBuf) {
    let mut paths = vec![first];
    if let Some(current) = env::var_os(name) {
        paths.extend(env::split_paths(&current));
    }
    if let Ok(joined) = env::join_paths(paths) {
        cmd.env(name, joined);
    }
}

fn executable_tool_name(tool: &str) -> String {
    if cfg!(windows) && !tool.to_ascii_lowercase().ends_with(".exe") {
        format!("{}.exe", tool)
    } else {
        tool.to_string()
    }
}

fn llvm_tool_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(path) = env::var("WAVE_LLVM_BIN") {
        if !path.trim().is_empty() {
            dirs.push(PathBuf::from(path));
        }
    }
    for env_name in ["WAVE_LLVM_HOME", "LLVM_SYS_211_PREFIX"] {
        if let Ok(path) = env::var(env_name) {
            if !path.trim().is_empty() {
                dirs.push(PathBuf::from(path).join("bin"));
            }
        }
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            dirs.push(dir.to_path_buf());
            dirs.push(dir.join("llvm").join("bin"));
            if let Some(root) = dir.parent() {
                dirs.push(root.join("llvm").join("bin"));
                dirs.push(root.join("lib").join("wave").join("llvm").join("bin"));
            }
        }
    }

    dirs
}
