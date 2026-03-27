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

use crate::errors::CliError;
use crate::flags::{
    validate_opt_flag, DebugFlags, DepFlags, DepPackage, LinkFlags, LlvmFlags, WhaleFlags,
};
use crate::{runner, std as wave_std, version};

use crate::version::get_os_pretty_name;
use std::collections::BTreeSet;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{self, Command as ProcessCommand, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs};
use utils::colorex::*;

#[derive(Debug)]
enum CliCommand {
    Build(BuildRequest),
    Print {
        item: String,
        target: Option<String>,
    },
    StdInstall,
    StdUpdate,
    Help,
    Version,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ErrorFormat {
    Human,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum EmitKind {
    Ast,
    Ir,
    Bc,
    Asm,
    Obj,
    Bin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EmitSpec {
    Check,
    Set(BTreeSet<EmitKind>),
}

impl EmitSpec {
    fn default_bin() -> Self {
        let mut set = BTreeSet::new();
        set.insert(EmitKind::Bin);
        EmitSpec::Set(set)
    }

    fn is_check(&self) -> bool {
        matches!(self, EmitSpec::Check)
    }

    fn as_set(&self) -> Option<&BTreeSet<EmitKind>> {
        match self {
            EmitSpec::Set(set) => Some(set),
            EmitSpec::Check => None,
        }
    }

    fn contains(&self, kind: EmitKind) -> bool {
        match self {
            EmitSpec::Check => false,
            EmitSpec::Set(set) => set.contains(&kind),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputKind {
    Wave,
    Ir,
    Bc,
    Asm,
    Obj,
}

impl InputKind {
    fn as_str(self) -> &'static str {
        match self {
            InputKind::Wave => "wave",
            InputKind::Ir => "ir",
            InputKind::Bc => "bc",
            InputKind::Asm => "asm",
            InputKind::Obj => "obj",
        }
    }
}

#[derive(Debug, Clone)]
struct BuildRequest {
    inputs: Vec<PathBuf>,
    output: Option<PathBuf>,
    out_dir: Option<PathBuf>,
    target_dir: Option<PathBuf>,
    emit: EmitSpec,
    input_type: Option<InputKind>,
    link_only: bool,
    run: bool,
    dry_run: bool,
    run_args: Vec<String>,
    freestanding: bool,
    entry: Option<String>,
    linker_script: Option<PathBuf>,
    no_start_files: bool,
    shared: bool,
    static_link: bool,
    pie: Option<bool>,
    error_format: ErrorFormat,
}

impl Default for BuildRequest {
    fn default() -> Self {
        Self {
            inputs: Vec::new(),
            output: None,
            out_dir: None,
            target_dir: None,
            emit: EmitSpec::default_bin(),
            input_type: None,
            link_only: false,
            run: false,
            dry_run: false,
            run_args: Vec::new(),
            freestanding: false,
            entry: None,
            linker_script: None,
            no_start_files: false,
            shared: false,
            static_link: false,
            pie: None,
            error_format: ErrorFormat::Human,
        }
    }
}

#[derive(Default, Clone)]
struct Global {
    opt: String,
    debug: DebugFlags,
    link: LinkFlags,
    dep: DepFlags,
    llvm: LlvmFlags,
    whale: WhaleFlags,
}

#[derive(Debug, Clone)]
struct ClassifiedInput {
    path: PathBuf,
    kind: InputKind,
}

#[derive(Debug, Clone)]
struct CompileJob {
    input: PathBuf,
    kind: InputKind,
    output: PathBuf,
}

#[derive(Debug, Clone, Default)]
struct BuildPlan {
    compile_jobs: Vec<CompileJob>,
    link_inputs: Vec<String>,
    link_output: Option<PathBuf>,
}

pub fn run() -> Result<(), CliError> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        return Err(CliError::usage("not enough arguments"));
    }

    let (global, rest) = parse_global(args)?;
    let cmd = parse_command(rest)?;

    dispatch(global, cmd)
}

fn dispatch(global: Global, cmd: CliCommand) -> Result<(), CliError> {
    if global.whale.enabled {
        return Err(CliError::usage(
            "TODO: --whale backend is reserved but not implemented yet",
        ));
    }

    match cmd {
        CliCommand::Version => {
            print_version();
            Ok(())
        }
        CliCommand::Help => {
            print_help();
            Ok(())
        }
        CliCommand::Build(build) => dispatch_build(&global, &build),
        CliCommand::Print { item, target } => dispatch_print(&global, &item, target.as_deref()),
        CliCommand::StdInstall => wave_std::std_install(),
        CliCommand::StdUpdate => wave_std::std_update(),
    }
}

fn dispatch_build(global: &Global, build: &BuildRequest) -> Result<(), CliError> {
    let effective_global = effective_global_for_build(global, build);
    let classified = classify_inputs(build)?;
    validate_build_request(&effective_global, build, &classified)?;

    let plan = create_build_plan(build, &classified)?;

    if build.dry_run {
        print_dry_run(&effective_global, build, &classified, &plan);
        return Ok(());
    }

    if build.emit.is_check() {
        for input in &classified {
            unsafe {
                runner::check_wave_file(&input.path, &effective_global.debug, &effective_global.dep);
            }
        }
        return Ok(());
    }

    let Some(emit_set) = build.emit.as_set() else {
        return Err(CliError::usage("invalid emit mode"));
    };

    execute_explicit_emit_artifacts(&effective_global, build, &classified, emit_set)?;

    for job in &plan.compile_jobs {
        match job.kind {
            InputKind::Wave => unsafe {
                runner::object_build_wave_file(
                    &job.input,
                    &effective_global.opt,
                    &effective_global.debug,
                    &effective_global.dep,
                    &effective_global.llvm,
                    Some(job.output.as_path()),
                );
            },
            InputKind::Ir | InputKind::Bc | InputKind::Asm => {
                compile_non_wave_to_object(&effective_global, job)?;
            }
            InputKind::Obj => {}
        }
    }

    if let Some(link_output) = &plan.link_output {
        if plan.link_inputs.is_empty() {
            return Err(CliError::CommandFailed(
                "no object inputs available for link stage".to_string(),
            ));
        }

        link_objects(&effective_global, build, &plan.link_inputs, link_output)?;

        if build.run {
            let status = ProcessCommand::new(link_output)
                .args(&build.run_args)
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .map_err(|e| {
                    CliError::CommandFailed(format!(
                        "failed to run `{}`: {}",
                        link_output.display(),
                        e
                    ))
                })?;

            if !status.success() {
                process::exit(status.code().unwrap_or(1));
            }
        }
    }

    Ok(())
}

fn effective_global_for_build(global: &Global, build: &BuildRequest) -> Global {
    let mut out = global.clone();

    if build.freestanding {
        out.llvm.no_default_libs = true;
    }
    if build.no_start_files {
        out.llvm.link_args.push("-nostartfiles".to_string());
    }
    if let Some(entry) = &build.entry {
        out.llvm.link_args.push(format!("-Wl,-e,{}", entry));
    }
    if let Some(script) = &build.linker_script {
        out.llvm
            .link_args
            .push(format!("-Wl,-T,{}", script.to_string_lossy()));
    }

    out
}

fn dispatch_print(global: &Global, item: &str, target_arg: Option<&str>) -> Result<(), CliError> {
    let target = target_arg
        .map(|s| s.to_string())
        .or_else(|| global.llvm.target.clone())
        .unwrap_or_else(host_target_triple);

    match item {
        "host-target" => {
            println!("{}", host_target_triple());
            Ok(())
        }
        "default-target" => {
            println!("{}", host_target_triple());
            Ok(())
        }
        "target-list" => {
            for t in supported_targets() {
                println!("{}", t);
            }
            Ok(())
        }
        "sysroot" => {
            if let Some(s) = detect_clang_sysroot() {
                println!("{}", s);
            } else {
                println!();
            }
            Ok(())
        }
        "dep-search-paths" => {
            let home = env::var("HOME").unwrap_or_default();
            if !home.is_empty() {
                println!("{}/.wave/lib/wave/std", home);
            }
            Ok(())
        }
        "default-linker" => {
            println!("{}", global.llvm.linker.as_deref().unwrap_or("clang"));
            Ok(())
        }
        "supported-input-types" => {
            for t in ["wave", "ir", "bc", "asm", "obj"] {
                println!("{}", t);
            }
            Ok(())
        }
        "supported-emit-kinds" => {
            println!("check (control-mode)");
            for e in ["ast", "ir", "bc", "asm", "obj", "bin"] {
                println!("{}", e);
            }
            Ok(())
        }
        "cpu-list" => {
            ensure_supported_target(&target)?;
            for cpu in cpu_list_for_target(&target) {
                println!("{}", cpu);
            }
            Ok(())
        }
        "target-features" => {
            ensure_supported_target(&target)?;
            for feat in target_features_for_target(&target) {
                println!("{}", feat);
            }
            Ok(())
        }
        _ => Err(CliError::usage(format!("unknown print item: {}", item))),
    }
}

fn parse_global(args: Vec<String>) -> Result<(Global, Vec<String>), CliError> {
    let mut g = Global {
        opt: "-O0".to_string(),
        debug: DebugFlags::default(),
        link: LinkFlags::default(),
        dep: DepFlags::default(),
        llvm: LlvmFlags::default(),
        whale: WhaleFlags::default(),
    };

    let mut rest: Vec<String> = Vec::new();
    let mut i = 0usize;

    while i < args.len() {
        let a = &args[i];

        if a == "--" {
            rest.push("--".to_string());
            rest.extend_from_slice(&args[i + 1..]);
            break;
        }

        if a == "--whale" {
            g.whale.enabled = true;
            i += 1;
            continue;
        }

        if a == "--llvm" {
            i += 1;
            continue;
        }

        if parse_llvm_backend_option(&args, &mut i, &mut g.llvm)? {
            continue;
        }

        if a.starts_with("-O") {
            if !validate_opt_flag(a) {
                return Err(CliError::usage(format!("invalid optimization flag: {}", a)));
            }
            g.opt = a.clone();
            i += 1;
            continue;
        }

        if let Some(mode) = a.strip_prefix("--debug-wave=") {
            g.debug.apply(mode);
            i += 1;
            continue;
        }

        if a == "--debug-wave" {
            let mode = args.get(i + 1).ok_or_else(|| {
                CliError::usage("missing value: --debug-wave <tokens,ast,ir,mc,hex,all,...>")
            })?;
            g.debug.apply(mode);
            i += 2;
            continue;
        }

        if let Some(lib) = a.strip_prefix("--link=") {
            g.link.libs.push(lib.to_string());
            i += 1;
            continue;
        }

        if a == "--link" {
            let lib = args
                .get(i + 1)
                .ok_or_else(|| CliError::usage("missing value: --link <lib>"))?;
            g.link.libs.push(lib.to_string());
            i += 2;
            continue;
        }

        if let Some(p) = a.strip_prefix("-L") {
            if p.is_empty() {
                let path = args
                    .get(i + 1)
                    .ok_or_else(|| CliError::usage("missing value: -L <path>"))?;
                g.link.paths.push(path.to_string());
                i += 2;
            } else if let Some(native) = p.strip_prefix("native=") {
                g.link.paths.push(native.to_string());
                i += 1;
            } else {
                g.link.paths.push(p.to_string());
                i += 1;
            }
            continue;
        }

        if let Some(path) = a.strip_prefix("--dep-root=") {
            if path.trim().is_empty() {
                return Err(CliError::usage("missing value: --dep-root <path>"));
            }
            g.dep.roots.push(path.to_string());
            i += 1;
            continue;
        }

        if a == "--dep-root" {
            let path = args
                .get(i + 1)
                .ok_or_else(|| CliError::usage("missing value: --dep-root <path>"))?;
            g.dep.roots.push(path.to_string());
            i += 2;
            continue;
        }

        if let Some(spec) = a.strip_prefix("--dep=") {
            let dep = parse_dep_spec(spec)?;
            if g.dep.packages.iter().any(|p| p.name == dep.name) {
                return Err(CliError::usage(format!(
                    "duplicate dependency mapping for '{}': pass --dep once per package",
                    dep.name
                )));
            }
            g.dep.packages.push(dep);
            i += 1;
            continue;
        }

        if a == "--dep" {
            let spec = args
                .get(i + 1)
                .ok_or_else(|| CliError::usage("missing value: --dep <name>=<path>"))?;
            let dep = parse_dep_spec(spec)?;
            if g.dep.packages.iter().any(|p| p.name == dep.name) {
                return Err(CliError::usage(format!(
                    "duplicate dependency mapping for '{}': pass --dep once per package",
                    dep.name
                )));
            }
            g.dep.packages.push(dep);
            i += 2;
            continue;
        }

        rest.push(a.clone());
        i += 1;
    }

    Ok((g, rest))
}

fn parse_dep_spec(spec: &str) -> Result<DepPackage, CliError> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return Err(CliError::usage(
            "invalid --dep value: expected <name>=<path>",
        ));
    }

    let Some((name, path)) = trimmed.split_once('=') else {
        return Err(CliError::usage(
            "invalid --dep value: expected <name>=<path>",
        ));
    };

    let name = name.trim();
    let path = path.trim();

    if name.is_empty() || path.is_empty() {
        return Err(CliError::usage(
            "invalid --dep value: expected <name>=<path>",
        ));
    }

    let mut chars = name.chars();
    let valid = if let Some(first) = chars.next() {
        (first.is_ascii_alphabetic() || first == '_')
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    } else {
        false
    };

    if !valid {
        return Err(CliError::usage(
            "invalid --dep package name: use [A-Za-z_][A-Za-z0-9_]*",
        ));
    }

    Ok(DepPackage {
        name: name.to_string(),
        path: path.to_string(),
    })
}

fn parse_llvm_backend_option(
    args: &[String],
    i: &mut usize,
    llvm: &mut LlvmFlags,
) -> Result<bool, CliError> {
    let a = &args[*i];

    if let Some(v) = a.strip_prefix("--target=") {
        if v.trim().is_empty() {
            return Err(CliError::usage("missing value: --target=<triple>"));
        }
        llvm.target = Some(v.to_string());
        *i += 1;
        return Ok(true);
    }
    if a == "--target" {
        let v = args
            .get(*i + 1)
            .ok_or_else(|| CliError::usage("missing value: --target <triple>"))?;
        if v.trim().is_empty() {
            return Err(CliError::usage("missing value: --target <triple>"));
        }
        llvm.target = Some(v.to_string());
        *i += 2;
        return Ok(true);
    }

    if let Some(v) = a.strip_prefix("--cpu=") {
        if v.trim().is_empty() {
            return Err(CliError::usage("missing value: --cpu=<name>"));
        }
        llvm.cpu = Some(v.to_string());
        *i += 1;
        return Ok(true);
    }
    if a == "--cpu" {
        let v = args
            .get(*i + 1)
            .ok_or_else(|| CliError::usage("missing value: --cpu <name>"))?;
        if v.trim().is_empty() {
            return Err(CliError::usage("missing value: --cpu <name>"));
        }
        llvm.cpu = Some(v.to_string());
        *i += 2;
        return Ok(true);
    }

    if let Some(v) = a.strip_prefix("--features=") {
        if v.trim().is_empty() {
            return Err(CliError::usage("missing value: --features=<csv>"));
        }
        llvm.features = Some(v.to_string());
        *i += 1;
        return Ok(true);
    }
    if a == "--features" {
        let v = args
            .get(*i + 1)
            .ok_or_else(|| CliError::usage("missing value: --features <csv>"))?;
        if v.trim().is_empty() {
            return Err(CliError::usage("missing value: --features <csv>"));
        }
        llvm.features = Some(v.to_string());
        *i += 2;
        return Ok(true);
    }

    if let Some(v) = a.strip_prefix("--abi=") {
        if v.trim().is_empty() {
            return Err(CliError::usage("missing value: --abi=<name>"));
        }
        llvm.abi = Some(v.to_string());
        *i += 1;
        return Ok(true);
    }
    if a == "--abi" {
        let v = args
            .get(*i + 1)
            .ok_or_else(|| CliError::usage("missing value: --abi <name>"))?;
        if v.trim().is_empty() {
            return Err(CliError::usage("missing value: --abi <name>"));
        }
        llvm.abi = Some(v.to_string());
        *i += 2;
        return Ok(true);
    }

    if let Some(v) = a.strip_prefix("--sysroot=") {
        if v.trim().is_empty() {
            return Err(CliError::usage("missing value: --sysroot=<path>"));
        }
        llvm.sysroot = Some(v.to_string());
        *i += 1;
        return Ok(true);
    }
    if a == "--sysroot" {
        let v = args
            .get(*i + 1)
            .ok_or_else(|| CliError::usage("missing value: --sysroot <path>"))?;
        if v.trim().is_empty() {
            return Err(CliError::usage("missing value: --sysroot <path>"));
        }
        llvm.sysroot = Some(v.to_string());
        *i += 2;
        return Ok(true);
    }

    if a == "-C" {
        let spec = args
            .get(*i + 1)
            .ok_or_else(|| CliError::usage("missing value: -C <key>[=<value>]"))?;
        parse_llvm_codegen_spec(spec, llvm)?;
        *i += 2;
        return Ok(true);
    }

    if let Some(spec) = a.strip_prefix("-C") {
        if spec.is_empty() {
            return Err(CliError::usage("missing value: -C <key>[=<value>]"));
        }
        parse_llvm_codegen_spec(spec, llvm)?;
        *i += 1;
        return Ok(true);
    }

    Ok(false)
}

fn parse_llvm_codegen_spec(spec: &str, llvm: &mut LlvmFlags) -> Result<(), CliError> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Err(CliError::usage("missing value: -C <key>[=<value>]"));
    }

    if spec == "no-default-libs" {
        llvm.no_default_libs = true;
        return Ok(());
    }

    let Some((key, value)) = spec.split_once('=') else {
        return Err(CliError::usage(format!(
            "invalid -C option '{}': expected key=value or no-default-libs",
            spec
        )));
    };

    let key = key.trim();
    let value = value.trim();
    if value.is_empty() {
        return Err(CliError::usage(format!("missing value for -C {}", key)));
    }

    match key {
        "linker" => llvm.linker = Some(value.to_string()),
        "link-arg" => llvm.link_args.push(value.to_string()),
        "code-model" => llvm.code_model = Some(value.to_string()),
        "relocation-model" => llvm.relocation_model = Some(value.to_string()),
        _ => {
            return Err(CliError::usage(format!(
                "unsupported -C option '{}': supported keys are linker, link-arg, no-default-libs, code-model, relocation-model",
                key
            )));
        }
    }

    Ok(())
}

fn parse_command(rest: Vec<String>) -> Result<CliCommand, CliError> {
    if rest.is_empty() {
        return Err(CliError::usage("not enough arguments"));
    }

    let cmd = rest[0].as_str();
    let args = &rest[1..];

    match cmd {
        "--help" | "-h" | "help" => Ok(CliCommand::Help),
        "--version" | "-V" | "version" => Ok(CliCommand::Version),

        "build" => parse_build(args),
        "run" => parse_run_alias(args),
        "check" => parse_check_alias(args),
        "print" => parse_print(args),

        "install" => parse_install(args),
        "update" => parse_update(args),

        other => Err(CliError::usage(format!("unknown command: {}", other))),
    }
}

fn parse_run_alias(args: &[String]) -> Result<CliCommand, CliError> {
    let mut file: Option<PathBuf> = None;
    let mut run_args: Vec<String> = Vec::new();
    let mut after_double_dash = false;

    for a in args {
        if after_double_dash {
            run_args.push(a.clone());
            continue;
        }

        if a == "--" {
            after_double_dash = true;
            continue;
        }

        if a.starts_with('-') {
            return Err(CliError::usage(format!("unknown option for run: {}", a)));
        }

        if file.is_none() {
            file = Some(PathBuf::from(a));
        } else {
            return Err(CliError::usage(format!("unexpected extra argument: {}", a)));
        }
    }

    let file = file.ok_or_else(|| CliError::usage("usage: wavec run <file>"))?;

    let mut build = BuildRequest::default();
    build.inputs.push(file);
    build.run = true;
    build.run_args = run_args;

    Ok(CliCommand::Build(build))
}

fn parse_check_alias(args: &[String]) -> Result<CliCommand, CliError> {
    let mut file: Option<PathBuf> = None;

    for a in args {
        if a.starts_with('-') {
            return Err(CliError::usage(format!("unknown option for check: {}", a)));
        }

        if file.is_none() {
            file = Some(PathBuf::from(a));
        } else {
            return Err(CliError::usage(format!("unexpected extra argument: {}", a)));
        }
    }

    let file = file.ok_or_else(|| CliError::usage("usage: wavec check <file>"))?;

    let mut build = BuildRequest::default();
    build.inputs.push(file);
    build.emit = EmitSpec::Check;

    Ok(CliCommand::Build(build))
}

fn parse_build(args: &[String]) -> Result<CliCommand, CliError> {
    let mut build = BuildRequest::default();
    let mut emit_explicit = false;
    let mut compile_only = false;
    let mut after_double_dash = false;
    let mut i = 0usize;

    while i < args.len() {
        let a = &args[i];

        if after_double_dash {
            build.run_args.push(a.clone());
            i += 1;
            continue;
        }

        match a.as_str() {
            "--" => {
                after_double_dash = true;
                i += 1;
            }
            "-c" => {
                compile_only = true;
                i += 1;
            }
            "-o" | "--output" => {
                let Some(v) = args.get(i + 1) else {
                    return Err(CliError::usage(format!("missing value: {} <file>", a)));
                };
                if v.trim().is_empty() {
                    return Err(CliError::usage(format!("invalid output file: {}", v)));
                }
                build.output = Some(PathBuf::from(v));
                i += 2;
            }
            _ if a.starts_with("--output=") => {
                let v = a.trim_start_matches("--output=");
                if v.trim().is_empty() {
                    return Err(CliError::usage("missing value: --output=<file>"));
                }
                build.output = Some(PathBuf::from(v));
                i += 1;
            }
            "--out-dir" => {
                let Some(v) = args.get(i + 1) else {
                    return Err(CliError::usage("missing value: --out-dir <dir>"));
                };
                if v.trim().is_empty() {
                    return Err(CliError::usage("missing value: --out-dir <dir>"));
                }
                build.out_dir = Some(PathBuf::from(v));
                i += 2;
            }
            _ if a.starts_with("--out-dir=") => {
                let v = a.trim_start_matches("--out-dir=");
                if v.trim().is_empty() {
                    return Err(CliError::usage("missing value: --out-dir=<dir>"));
                }
                build.out_dir = Some(PathBuf::from(v));
                i += 1;
            }
            "--target-dir" => {
                let Some(v) = args.get(i + 1) else {
                    return Err(CliError::usage("missing value: --target-dir <dir>"));
                };
                if v.trim().is_empty() {
                    return Err(CliError::usage("missing value: --target-dir <dir>"));
                }
                build.target_dir = Some(PathBuf::from(v));
                i += 2;
            }
            _ if a.starts_with("--target-dir=") => {
                let v = a.trim_start_matches("--target-dir=");
                if v.trim().is_empty() {
                    return Err(CliError::usage("missing value: --target-dir=<dir>"));
                }
                build.target_dir = Some(PathBuf::from(v));
                i += 1;
            }
            "--emit" => {
                let Some(v) = args.get(i + 1) else {
                    return Err(CliError::usage("missing value: --emit <kinds>"));
                };
                apply_emit_spec(&mut build, &mut emit_explicit, v)?;
                i += 2;
            }
            _ if a.starts_with("--emit=") => {
                let v = a.trim_start_matches("--emit=");
                apply_emit_spec(&mut build, &mut emit_explicit, v)?;
                i += 1;
            }
            "--input-type" => {
                let Some(v) = args.get(i + 1) else {
                    return Err(CliError::usage("missing value: --input-type <kind>"));
                };
                build.input_type = Some(parse_input_kind(v)?);
                i += 2;
            }
            _ if a.starts_with("--input-type=") => {
                let v = a.trim_start_matches("--input-type=");
                build.input_type = Some(parse_input_kind(v)?);
                i += 1;
            }
            "--link-only" => {
                build.link_only = true;
                i += 1;
            }
            "--run" => {
                build.run = true;
                i += 1;
            }
            "--dry-run" => {
                build.dry_run = true;
                i += 1;
            }
            "--freestanding" => {
                build.freestanding = true;
                i += 1;
            }
            "--entry" => {
                let Some(v) = args.get(i + 1) else {
                    return Err(CliError::usage("missing value: --entry <symbol>"));
                };
                if v.trim().is_empty() {
                    return Err(CliError::usage("missing value: --entry <symbol>"));
                }
                build.entry = Some(v.clone());
                i += 2;
            }
            _ if a.starts_with("--entry=") => {
                let v = a.trim_start_matches("--entry=");
                if v.trim().is_empty() {
                    return Err(CliError::usage("missing value: --entry=<symbol>"));
                }
                build.entry = Some(v.to_string());
                i += 1;
            }
            "--linker-script" => {
                let Some(v) = args.get(i + 1) else {
                    return Err(CliError::usage("missing value: --linker-script <path>"));
                };
                if v.trim().is_empty() {
                    return Err(CliError::usage("missing value: --linker-script <path>"));
                }
                build.linker_script = Some(PathBuf::from(v));
                i += 2;
            }
            _ if a.starts_with("--linker-script=") => {
                let v = a.trim_start_matches("--linker-script=");
                if v.trim().is_empty() {
                    return Err(CliError::usage("missing value: --linker-script=<path>"));
                }
                build.linker_script = Some(PathBuf::from(v));
                i += 1;
            }
            "--no-start-files" => {
                build.no_start_files = true;
                i += 1;
            }
            "--shared" => {
                build.shared = true;
                i += 1;
            }
            "--static" => {
                build.static_link = true;
                i += 1;
            }
            "--pie" => {
                if build.pie == Some(false) {
                    return Err(CliError::usage("cannot combine --pie and --no-pie"));
                }
                build.pie = Some(true);
                i += 1;
            }
            "--no-pie" => {
                if build.pie == Some(true) {
                    return Err(CliError::usage("cannot combine --pie and --no-pie"));
                }
                build.pie = Some(false);
                i += 1;
            }
            "--error-format" => {
                let Some(v) = args.get(i + 1) else {
                    return Err(CliError::usage("missing value: --error-format <human,json>"));
                };
                build.error_format = parse_error_format(v)?;
                i += 2;
            }
            _ if a.starts_with("--error-format=") => {
                let v = a.trim_start_matches("--error-format=");
                build.error_format = parse_error_format(v)?;
                i += 1;
            }
            _ if a.starts_with('-') => {
                return Err(CliError::usage(format!("unknown option for build: {}", a)));
            }
            _ => {
                build.inputs.push(PathBuf::from(a));
                i += 1;
            }
        }
    }

    if build.inputs.is_empty() {
        return Err(CliError::usage("usage: wavec build <input...> [options]"));
    }

    if !build.run_args.is_empty() && !build.run {
        return Err(CliError::usage(
            "run arguments after `--` require --run (or use `wavec run <file> -- <args...>`)",
        ));
    }

    if compile_only {
        match &build.emit {
            EmitSpec::Check => {
                return Err(CliError::usage("-c cannot be combined with --emit=check"));
            }
            EmitSpec::Set(set) => {
                if emit_explicit {
                    if !(set.len() == 1 && set.contains(&EmitKind::Obj)) {
                        return Err(CliError::usage(
                            "-c is equivalent to --emit=obj and cannot be combined with other emit kinds",
                        ));
                    }
                } else {
                    let mut obj_only = BTreeSet::new();
                    obj_only.insert(EmitKind::Obj);
                    build.emit = EmitSpec::Set(obj_only);
                }
            }
        }
    }

    Ok(CliCommand::Build(build))
}

fn parse_print(args: &[String]) -> Result<CliCommand, CliError> {
    let item = args
        .first()
        .ok_or_else(|| CliError::usage("usage: wavec print <item> [--target <triple>]"))?
        .clone();

    let mut target: Option<String> = None;
    let mut i = 1usize;
    while i < args.len() {
        let a = &args[i];
        if a == "--target" {
            let v = args
                .get(i + 1)
                .ok_or_else(|| CliError::usage("missing value: --target <triple>"))?;
            if v.trim().is_empty() {
                return Err(CliError::usage("missing value: --target <triple>"));
            }
            target = Some(v.clone());
            i += 2;
            continue;
        }
        if let Some(v) = a.strip_prefix("--target=") {
            if v.trim().is_empty() {
                return Err(CliError::usage("missing value: --target=<triple>"));
            }
            target = Some(v.to_string());
            i += 1;
            continue;
        }

        return Err(CliError::usage(format!(
            "unknown option for print: {}",
            a
        )));
    }

    Ok(CliCommand::Print { item, target })
}

fn parse_install(args: &[String]) -> Result<CliCommand, CliError> {
    let target = args
        .first()
        .ok_or_else(|| CliError::usage("usage: wavec install <target>"))?;
    if args.len() > 1 {
        return Err(CliError::usage(format!(
            "unexpected extra argument: {}",
            args[1]
        )));
    }

    match target.as_str() {
        "std" => Ok(CliCommand::StdInstall),
        _ => Err(CliError::usage(format!(
            "unknown install target: {}",
            target
        ))),
    }
}

fn parse_update(args: &[String]) -> Result<CliCommand, CliError> {
    let target = args
        .first()
        .ok_or_else(|| CliError::usage("usage: wavec update <target>"))?;
    if args.len() > 1 {
        return Err(CliError::usage(format!(
            "unexpected extra argument: {}",
            args[1]
        )));
    }

    match target.as_str() {
        "std" => Ok(CliCommand::StdUpdate),
        _ => Err(CliError::usage(format!(
            "unknown update target: {}",
            target
        ))),
    }
}

fn parse_input_kind(v: &str) -> Result<InputKind, CliError> {
    match v.trim().to_ascii_lowercase().as_str() {
        "wave" => Ok(InputKind::Wave),
        "ir" => Ok(InputKind::Ir),
        "bc" => Ok(InputKind::Bc),
        "asm" => Ok(InputKind::Asm),
        "obj" => Ok(InputKind::Obj),
        _ => Err(CliError::usage(format!(
            "invalid --input-type '{}': expected wave, ir, bc, asm, obj",
            v
        ))),
    }
}

fn parse_error_format(v: &str) -> Result<ErrorFormat, CliError> {
    match v.trim() {
        "human" => Ok(ErrorFormat::Human),
        "json" => Ok(ErrorFormat::Json),
        _ => Err(CliError::usage(format!(
            "invalid --error-format '{}': expected human, json",
            v
        ))),
    }
}

fn parse_emit_kind(item: &str) -> Result<EmitKind, CliError> {
    match item.trim() {
        "ast" => Ok(EmitKind::Ast),
        "ir" => Ok(EmitKind::Ir),
        "bc" => Ok(EmitKind::Bc),
        "asm" => Ok(EmitKind::Asm),
        "obj" => Ok(EmitKind::Obj),
        "bin" => Ok(EmitKind::Bin),
        _ => Err(CliError::usage(format!(
            "unknown --emit kind '{}': expected check, ast, ir, bc, asm, obj, bin",
            item
        ))),
    }
}

fn apply_emit_spec(
    build: &mut BuildRequest,
    emit_explicit: &mut bool,
    spec: &str,
) -> Result<(), CliError> {
    if spec.trim().is_empty() {
        return Err(CliError::usage("missing value: --emit=<kinds>"));
    }

    if !*emit_explicit {
        build.emit = EmitSpec::Set(BTreeSet::new());
        *emit_explicit = true;
    }

    let mut saw_check = false;
    let mut set = BTreeSet::new();

    for raw in spec.split(',') {
        let item = raw.trim();
        if item.is_empty() {
            continue;
        }

        if item == "check" {
            saw_check = true;
        } else {
            set.insert(parse_emit_kind(item)?);
        }
    }

    if saw_check && !set.is_empty() {
        return Err(CliError::usage(
            "--emit=check must be used alone (check is a control mode)",
        ));
    }

    if saw_check {
        match build.emit {
            EmitSpec::Check => return Ok(()),
            EmitSpec::Set(ref existing) if existing.is_empty() => {
                build.emit = EmitSpec::Check;
                return Ok(());
            }
            EmitSpec::Set(_) => {
                return Err(CliError::usage(
                    "--emit=check cannot be combined with other emit kinds",
                ));
            }
        }
    }

    if set.is_empty() {
        return Err(CliError::usage("--emit requires at least one emit kind"));
    }

    match &mut build.emit {
        EmitSpec::Check => Err(CliError::usage(
            "--emit=check cannot be combined with other emit kinds",
        )),
        EmitSpec::Set(existing) => {
            existing.extend(set);
            Ok(())
        }
    }
}

fn classify_inputs(build: &BuildRequest) -> Result<Vec<ClassifiedInput>, CliError> {
    let mut out = Vec::with_capacity(build.inputs.len());
    for input in &build.inputs {
        let kind = resolve_input_kind(input, build.input_type)?;
        out.push(ClassifiedInput {
            path: input.clone(),
            kind,
        });
    }
    Ok(out)
}

fn resolve_input_kind(path: &Path, forced: Option<InputKind>) -> Result<InputKind, CliError> {
    let inferred = infer_input_kind(path);

    if let Some(forced_kind) = forced {
        if let Some(inferred_kind) = inferred {
            if inferred_kind != forced_kind {
                return Err(CliError::usage(format!(
                    "--input-type={} conflicts with input '{}'(inferred {})",
                    forced_kind.as_str(),
                    path.display(),
                    inferred_kind.as_str()
                )));
            }
        }
        return Ok(forced_kind);
    }

    inferred.ok_or_else(|| {
        CliError::usage(format!(
            "cannot infer input type for '{}': use --input-type=<wave,ir,bc,asm,obj>",
            path.display()
        ))
    })
}

fn infer_input_kind(path: &Path) -> Option<InputKind> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "wave" => Some(InputKind::Wave),
        "ll" | "ir" => Some(InputKind::Ir),
        "bc" => Some(InputKind::Bc),
        "s" | "asm" => Some(InputKind::Asm),
        "o" | "obj" => Some(InputKind::Obj),
        _ => None,
    }
}

fn validate_build_request(
    global: &Global,
    build: &BuildRequest,
    classified: &[ClassifiedInput],
) -> Result<(), CliError> {
    if build.shared && build.static_link {
        return Err(CliError::usage("cannot combine --shared and --static"));
    }
    if build.shared && build.pie.is_some() {
        return Err(CliError::usage(
            "cannot combine --shared with --pie/--no-pie in v1",
        ));
    }

    if let Some(reloc) = global.llvm.relocation_model.as_deref() {
        if build.pie == Some(true) && reloc != "pie" {
            return Err(CliError::usage(
                "--pie requires -C relocation-model=pie when relocation-model is set",
            ));
        }
        if build.pie == Some(false) && reloc == "pie" {
            return Err(CliError::usage(
                "--no-pie cannot be combined with -C relocation-model=pie",
            ));
        }
        if build.shared && reloc != "pic" && reloc != "dynamic-no-pic" {
            return Err(CliError::usage(
                "--shared requires -C relocation-model=pic or dynamic-no-pic",
            ));
        }
    }

    if build.emit.is_check() {
        if build.link_only {
            return Err(CliError::usage(
                "--emit=check cannot be combined with --link-only",
            ));
        }
        if build.run {
            return Err(CliError::usage("--emit=check cannot be combined with --run"));
        }
        if build.output.is_some() || build.out_dir.is_some() {
            return Err(CliError::usage(
                "--emit=check does not produce artifacts; remove -o/--out-dir",
            ));
        }
        if classified.iter().any(|i| i.kind != InputKind::Wave) {
            return Err(CliError::usage(
                "--emit=check currently supports only Wave source inputs",
            ));
        }
        return Ok(());
    }

    let emit_set = build.emit.as_set().expect("non-check emit set expected");

    for kind in [EmitKind::Ast, EmitKind::Ir, EmitKind::Bc, EmitKind::Asm] {
        if emit_set.contains(&kind)
            && !classified
                .iter()
                .any(|input| supports_emit_for_input(kind, input.kind))
        {
            return Err(CliError::usage(format!(
                "--emit={} has no compatible inputs in this build request",
                emit_kind_name(kind)
            )));
        }
    }

    if build.link_only {
        if !(emit_set.len() == 1 && emit_set.contains(&EmitKind::Bin)) {
            return Err(CliError::usage(
                "--link-only supports only --emit=bin in v1",
            ));
        }
        if classified.iter().any(|i| i.kind != InputKind::Obj) {
            return Err(CliError::usage(
                "--link-only requires object inputs only (.o/.obj)",
            ));
        }
    }

    if build.run {
        if !emit_set.contains(&EmitKind::Bin) {
            return Err(CliError::usage(
                "--run requires a binary output (emit includes bin)",
            ));
        }
        if build.shared {
            return Err(CliError::usage(
                "--run is not allowed when --shared is specified",
            ));
        }
    }

    let need_link = emit_set.contains(&EmitKind::Bin) || build.run;
    if (build.entry.is_some() || build.linker_script.is_some() || build.no_start_files) && !need_link
    {
        return Err(CliError::usage(
            "--entry/--linker-script/--no-start-files require a link stage (emit includes bin)",
        ));
    }

    if build.output.is_some() {
        let compile_count = classified.iter().filter(|i| i.kind != InputKind::Obj).count();
        let has_bin = emit_set.contains(&EmitKind::Bin) || build.run;

        if !has_bin {
            let obj_only = emit_set.len() == 1 && emit_set.contains(&EmitKind::Obj);
            if !(obj_only && compile_count == 1) {
                return Err(CliError::usage(
                    "-o is only allowed for final binary output, or single-input --emit=obj",
                ));
            }
        }
    }

    Ok(())
}

fn create_build_plan(
    build: &BuildRequest,
    classified: &[ClassifiedInput],
) -> Result<BuildPlan, CliError> {
    if build.emit.is_check() {
        return Ok(BuildPlan::default());
    }

    let emit_set = build.emit.as_set().expect("non-check emit set expected");
    let need_objects = emit_set.contains(&EmitKind::Obj) || emit_set.contains(&EmitKind::Bin) || build.run;
    let need_link = emit_set.contains(&EmitKind::Bin) || build.run;

    if !need_objects && !need_link {
        return Ok(BuildPlan::default());
    }

    let compile_total = classified.iter().filter(|i| i.kind != InputKind::Obj).count();
    let mut compile_index = 0usize;

    let mut plan = BuildPlan::default();

    for input in classified {
        if input.kind == InputKind::Obj {
            plan.link_inputs
                .push(input.path.to_string_lossy().to_string());
            continue;
        }

        if !need_objects {
            continue;
        }

        let output = resolve_object_output_path(
            build,
            input,
            compile_index,
            compile_total,
            emit_set.contains(&EmitKind::Obj),
            need_link,
        );

        plan.link_inputs.push(output.to_string_lossy().to_string());
        plan.compile_jobs.push(CompileJob {
            input: input.path.clone(),
            kind: input.kind,
            output,
        });
        compile_index += 1;
    }

    if need_link {
        let primary = classified
            .first()
            .ok_or_else(|| CliError::usage("build requires at least one input"))?;
        plan.link_output = Some(resolve_binary_output_path(build, primary));
    }

    Ok(plan)
}

fn resolve_object_output_path(
    build: &BuildRequest,
    input: &ClassifiedInput,
    compile_index: usize,
    compile_total: usize,
    emit_obj: bool,
    need_link: bool,
) -> PathBuf {
    if emit_obj && !need_link && compile_total == 1 {
        if let Some(path) = &build.output {
            return path.clone();
        }
    }

    let file_name = object_file_name(&input.path, compile_index, compile_total);

    if emit_obj {
        if let Some(out_dir) = &build.out_dir {
            return out_dir.join(&file_name);
        }
        if let Some(target_dir) = &build.target_dir {
            return target_dir.join(&file_name);
        }
        return PathBuf::from(file_name);
    }

    if let Some(target_dir) = &build.target_dir {
        return target_dir.join(file_name);
    }

    PathBuf::from("target").join(file_name)
}

fn resolve_binary_output_path(build: &BuildRequest, primary: &ClassifiedInput) -> PathBuf {
    if let Some(path) = &build.output {
        return path.clone();
    }

    let stem = primary
        .path
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("a.out")
        .to_string();

    if let Some(out_dir) = &build.out_dir {
        return out_dir.join(&stem);
    }

    if let Some(target_dir) = &build.target_dir {
        return target_dir.join(&stem);
    }

    PathBuf::from("target").join(&stem)
}

fn object_file_name(path: &Path, compile_index: usize, compile_total: usize) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("input");

    if compile_total > 1 {
        format!("{}_{}.o", stem, compile_index + 1)
    } else {
        format!("{}.o", stem)
    }
}

fn emit_kind_name(kind: EmitKind) -> &'static str {
    match kind {
        EmitKind::Ast => "ast",
        EmitKind::Ir => "ir",
        EmitKind::Bc => "bc",
        EmitKind::Asm => "asm",
        EmitKind::Obj => "obj",
        EmitKind::Bin => "bin",
    }
}

fn supports_emit_for_input(kind: EmitKind, input: InputKind) -> bool {
    match kind {
        EmitKind::Ast => input == InputKind::Wave,
        EmitKind::Ir => input == InputKind::Wave || input == InputKind::Ir,
        EmitKind::Bc => matches!(input, InputKind::Wave | InputKind::Ir | InputKind::Bc),
        EmitKind::Asm => matches!(
            input,
            InputKind::Wave | InputKind::Ir | InputKind::Bc | InputKind::Asm
        ),
        EmitKind::Obj => matches!(
            input,
            InputKind::Wave | InputKind::Ir | InputKind::Bc | InputKind::Asm | InputKind::Obj
        ),
        EmitKind::Bin => matches!(
            input,
            InputKind::Wave | InputKind::Ir | InputKind::Bc | InputKind::Asm | InputKind::Obj
        ),
    }
}

fn emit_artifact_extension(kind: EmitKind) -> &'static str {
    match kind {
        EmitKind::Ast => "ast",
        EmitKind::Ir => "ll",
        EmitKind::Bc => "bc",
        EmitKind::Asm => "s",
        EmitKind::Obj => "o",
        EmitKind::Bin => "",
    }
}

fn emit_artifact_file_name(
    path: &Path,
    input_index: usize,
    input_total: usize,
    kind: EmitKind,
) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("input");

    let base = if input_total > 1 {
        format!("{}_{}", stem, input_index + 1)
    } else {
        stem.to_string()
    };

    let ext = emit_artifact_extension(kind);
    if ext.is_empty() {
        base
    } else {
        format!("{}.{}", base, ext)
    }
}

fn resolve_extra_emit_output_path(
    build: &BuildRequest,
    input: &ClassifiedInput,
    kind: EmitKind,
    input_index: usize,
    input_total: usize,
) -> PathBuf {
    let file_name = emit_artifact_file_name(&input.path, input_index, input_total, kind);
    if let Some(out_dir) = &build.out_dir {
        return out_dir.join(&file_name);
    }
    if let Some(target_dir) = &build.target_dir {
        return target_dir.join(&file_name);
    }
    PathBuf::from(file_name)
}

fn copy_if_different(src: &Path, dst: &Path) -> Result<(), CliError> {
    if src == dst {
        return Ok(());
    }
    ensure_parent_dir(dst)?;
    fs::copy(src, dst)?;
    Ok(())
}

fn execute_explicit_emit_artifacts(
    global: &Global,
    build: &BuildRequest,
    classified: &[ClassifiedInput],
    emit_set: &BTreeSet<EmitKind>,
) -> Result<(), CliError> {
    let kinds = [EmitKind::Ast, EmitKind::Ir, EmitKind::Bc, EmitKind::Asm];
    let total_inputs = classified.len();

    for (input_index, input) in classified.iter().enumerate() {
        for kind in kinds {
            if !emit_set.contains(&kind) || !supports_emit_for_input(kind, input.kind) {
                continue;
            }

            let output = resolve_extra_emit_output_path(build, input, kind, input_index, total_inputs);
            ensure_parent_dir(&output)?;

            match kind {
                EmitKind::Ast => {
                    let text = unsafe { runner::emit_wave_ast_text(&input.path, &global.debug, &global.dep) };
                    fs::write(output, text)?;
                }
                EmitKind::Ir => match input.kind {
                    InputKind::Wave => {
                        let text = unsafe {
                            runner::emit_wave_ir_text(
                                &input.path,
                                &global.opt,
                                &global.debug,
                                &global.dep,
                                &global.llvm,
                            )
                        };
                        fs::write(output, text)?;
                    }
                    InputKind::Ir => copy_if_different(&input.path, &output)?,
                    _ => {}
                },
                EmitKind::Bc => match input.kind {
                    InputKind::Wave => {
                        let text = unsafe {
                            runner::emit_wave_ir_text(
                                &input.path,
                                &global.opt,
                                &global.debug,
                                &global.dep,
                                &global.llvm,
                            )
                        };
                        emit_ir_text_via_clang(global, &text, &output, EmitKind::Bc)?;
                    }
                    InputKind::Ir => {
                        compile_lowering_with_clang(
                            global,
                            &input.path,
                            InputKind::Ir,
                            &output,
                            EmitKind::Bc,
                        )?;
                    }
                    InputKind::Bc => copy_if_different(&input.path, &output)?,
                    _ => {}
                },
                EmitKind::Asm => match input.kind {
                    InputKind::Wave => {
                        let text = unsafe {
                            runner::emit_wave_ir_text(
                                &input.path,
                                &global.opt,
                                &global.debug,
                                &global.dep,
                                &global.llvm,
                            )
                        };
                        emit_ir_text_via_clang(global, &text, &output, EmitKind::Asm)?;
                    }
                    InputKind::Ir | InputKind::Bc => {
                        compile_lowering_with_clang(global, &input.path, input.kind, &output, EmitKind::Asm)?;
                    }
                    InputKind::Asm => copy_if_different(&input.path, &output)?,
                    _ => {}
                },
                _ => {}
            }
        }
    }

    Ok(())
}

fn compile_non_wave_to_object(global: &Global, job: &CompileJob) -> Result<(), CliError> {
    ensure_parent_dir(&job.output)?;
    compile_lowering_with_clang(global, &job.input, job.kind, &job.output, EmitKind::Obj)
}

fn compile_lowering_with_clang(
    global: &Global,
    input: &Path,
    input_kind: InputKind,
    output: &Path,
    emit_kind: EmitKind,
) -> Result<(), CliError> {
    let (bin, args) = build_clang_lowering_args(global, input, input_kind, output, emit_kind);
    let output = ProcessCommand::new(&bin).args(&args).output().map_err(|e| {
        if e.kind() == ErrorKind::NotFound && bin == "clang" {
            CliError::ExternalToolMissing("clang")
        } else {
            CliError::Io(e)
        }
    })?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

    Err(CliError::CommandFailed(format!(
        "{} failed (status={})\nstdout: {}\nstderr: {}",
        emit_kind_name(emit_kind),
        output.status,
        stdout,
        stderr
    )))
}

fn build_clang_lowering_args(
    global: &Global,
    input: &Path,
    input_kind: InputKind,
    output: &Path,
    emit_kind: EmitKind,
) -> (String, Vec<String>) {
    let mut args = Vec::new();

    if let Some(target) = &global.llvm.target {
        args.push(format!("--target={}", target));
    }
    if let Some(sysroot) = &global.llvm.sysroot {
        args.push(format!("--sysroot={}", sysroot));
    }
    if let Some(abi) = &global.llvm.abi {
        args.push("-target-abi".to_string());
        args.push(abi.to_string());
    }

    if !global.opt.is_empty() {
        args.push(normalize_opt_for_clang(&global.opt).to_string());
    }

    if input_kind == InputKind::Ir {
        args.push("-x".to_string());
        args.push("ir".to_string());
    }

    match emit_kind {
        EmitKind::Obj => {
            args.push("-c".to_string());
        }
        EmitKind::Bc => {
            args.push("-c".to_string());
            args.push("-emit-llvm".to_string());
        }
        EmitKind::Asm => {
            args.push("-S".to_string());
        }
        _ => {}
    }

    args.push(input.to_string_lossy().to_string());
    args.push("-o".to_string());
    args.push(output.to_string_lossy().to_string());

    ("clang".to_string(), args)
}

fn emit_ir_text_via_clang(
    global: &Global,
    ir_text: &str,
    output: &Path,
    emit_kind: EmitKind,
) -> Result<(), CliError> {
    let temp_input = temp_ir_input_path();
    fs::write(&temp_input, ir_text)?;
    let result = compile_lowering_with_clang(global, &temp_input, InputKind::Ir, output, emit_kind);
    let _ = fs::remove_file(&temp_input);
    result
}

fn temp_ir_input_path() -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    env::temp_dir().join(format!("wavec-{}-{}.ll", process::id(), now))
}

fn build_clang_compile_args(global: &Global, job: &CompileJob) -> (String, Vec<String>) {
    let mut args = Vec::new();

    if let Some(target) = &global.llvm.target {
        args.push(format!("--target={}", target));
    }
    if let Some(sysroot) = &global.llvm.sysroot {
        args.push(format!("--sysroot={}", sysroot));
    }
    if let Some(abi) = &global.llvm.abi {
        args.push("-target-abi".to_string());
        args.push(abi.to_string());
    }

    if !global.opt.is_empty() {
        args.push(normalize_opt_for_clang(&global.opt).to_string());
    }

    if job.kind == InputKind::Ir {
        args.push("-x".to_string());
        args.push("ir".to_string());
    }

    args.push("-c".to_string());
    args.push(job.input.to_string_lossy().to_string());
    args.push("-o".to_string());
    args.push(job.output.to_string_lossy().to_string());

    ("clang".to_string(), args)
}

fn link_objects(
    global: &Global,
    build: &BuildRequest,
    objects: &[String],
    output: &Path,
) -> Result<(), CliError> {
    ensure_parent_dir(output)?;

    let (bin, args) = build_linker_args(global, build, objects, output);
    let out = ProcessCommand::new(&bin)
        .args(&args)
        .output()
        .map_err(|e| {
            if e.kind() == ErrorKind::NotFound && bin == "clang" {
                CliError::ExternalToolMissing("clang")
            } else {
                CliError::Io(e)
            }
        })?;

    if out.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();

    Err(CliError::CommandFailed(format!(
        "link failed (status={})\nstdout: {}\nstderr: {}",
        out.status, stdout, stderr
    )))
}

fn build_linker_args(
    global: &Global,
    build: &BuildRequest,
    objects: &[String],
    output: &Path,
) -> (String, Vec<String>) {
    let linker_bin = global
        .llvm
        .linker
        .clone()
        .unwrap_or_else(|| "clang".to_string());

    let mut args = Vec::new();

    if global.llvm.linker.is_none() {
        if let Some(target) = &global.llvm.target {
            args.push(format!("--target={}", target));
        }
        if let Some(sysroot) = &global.llvm.sysroot {
            args.push(format!("--sysroot={}", sysroot));
        }
        if let Some(abi) = &global.llvm.abi {
            args.push("-target-abi".to_string());
            args.push(abi.to_string());
        }
    }

    for obj in objects {
        args.push(obj.clone());
    }

    for path in &global.link.paths {
        args.push(format!("-L{}", path));
    }

    for lib in &global.link.libs {
        args.push(format!("-l{}", lib));
    }

    for arg in &global.llvm.link_args {
        args.push(arg.clone());
    }

    if build.shared {
        args.push("-shared".to_string());
    }
    if build.static_link {
        args.push("-static".to_string());
    }
    if build.pie == Some(true) {
        args.push("-pie".to_string());
    }
    if build.pie == Some(false) {
        args.push("-no-pie".to_string());
    }

    args.push("-o".to_string());
    args.push(output.to_string_lossy().to_string());

    if !global.llvm.no_default_libs {
        args.push("-lc".to_string());
        args.push("-lm".to_string());
    }

    (linker_bin, args)
}

fn normalize_opt_for_clang(flag: &str) -> &str {
    match flag {
        "-Ofast" => "-O3",
        other => other,
    }
}

fn ensure_parent_dir(path: &Path) -> Result<(), CliError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}

fn dry_run_explicit_emit_steps(
    global: &Global,
    build: &BuildRequest,
    classified: &[ClassifiedInput],
) -> Vec<String> {
    let Some(emit_set) = build.emit.as_set() else {
        return Vec::new();
    };

    let total_inputs = classified.len();
    let mut steps = Vec::new();
    let kinds = [EmitKind::Ast, EmitKind::Ir, EmitKind::Bc, EmitKind::Asm];

    for (input_index, input) in classified.iter().enumerate() {
        for kind in kinds {
            if !emit_set.contains(&kind) || !supports_emit_for_input(kind, input.kind) {
                continue;
            }

            let output = resolve_extra_emit_output_path(build, input, kind, input_index, total_inputs);
            let step = match (kind, input.kind) {
                (EmitKind::Ast, InputKind::Wave) => {
                    format!(
                        "[wave frontend] {} -> {} (ast)",
                        input.path.display(),
                        output.display()
                    )
                }
                (EmitKind::Ir, InputKind::Wave) => {
                    format!(
                        "[wave frontend] {} -> {} (ir)",
                        input.path.display(),
                        output.display()
                    )
                }
                (EmitKind::Ir, InputKind::Ir) => {
                    format!("cp {} {}", input.path.display(), output.display())
                }
                (EmitKind::Bc, InputKind::Wave) | (EmitKind::Asm, InputKind::Wave) => {
                    format!(
                        "[wave frontend + clang] {} -> {} ({})",
                        input.path.display(),
                        output.display(),
                        emit_kind_name(kind)
                    )
                }
                (EmitKind::Bc, InputKind::Ir)
                | (EmitKind::Asm, InputKind::Ir)
                | (EmitKind::Asm, InputKind::Bc) => {
                    let (bin, args) =
                        build_clang_lowering_args(global, &input.path, input.kind, &output, kind);
                    shell_join(&bin, &args)
                }
                (EmitKind::Bc, InputKind::Bc) | (EmitKind::Asm, InputKind::Asm) => {
                    format!("cp {} {}", input.path.display(), output.display())
                }
                _ => continue,
            };
            steps.push(step);
        }
    }

    steps
}

fn print_dry_run(
    global: &Global,
    build: &BuildRequest,
    classified: &[ClassifiedInput],
    plan: &BuildPlan,
) {
    match build.error_format {
        ErrorFormat::Human => print_dry_run_human(global, build, classified, plan),
        ErrorFormat::Json => print_dry_run_json(global, build, classified, plan),
    }
}

fn print_dry_run_human(
    global: &Global,
    build: &BuildRequest,
    classified: &[ClassifiedInput],
    plan: &BuildPlan,
) {
    println!("DRY-RUN PLAN");
    println!("  mode: {}", build_mode_label(build));
    println!("  emit: {}", render_emit_spec(&build.emit));
    println!("  link-only: {}", build.link_only);
    println!("  run: {}", build.run);
    println!("  freestanding: {}", build.freestanding);
    if let Some(entry) = &build.entry {
        println!("  entry: {}", entry);
    }
    if let Some(script) = &build.linker_script {
        println!("  linker-script: {}", script.display());
    }
    println!("  no-start-files: {}", build.no_start_files);
    if !build.run_args.is_empty() {
        println!("  run-args: {}", build.run_args.join(" "));
    }

    println!("  inputs:");
    for i in classified {
        println!("    - {} ({})", i.path.display(), i.kind.as_str());
    }

    if build.emit.is_check() {
        println!("  steps:");
        println!("    - frontend check only (parse/import/semantic)");
        return;
    }

    let emit_jobs = dry_run_explicit_emit_steps(global, build, classified);
    if !emit_jobs.is_empty() {
        println!("  emit jobs:");
        for step in emit_jobs {
            println!("    - {}", step);
        }
    }

    if !plan.compile_jobs.is_empty() {
        println!("  compile jobs:");
        for job in &plan.compile_jobs {
            if job.kind == InputKind::Wave {
                println!(
                    "    - [wave frontend] {} -> {}",
                    job.input.display(),
                    job.output.display()
                );
            } else {
                let (bin, args) = build_clang_compile_args(global, job);
                println!("    - {}", shell_join(&bin, &args));
            }
        }
    }

    if let Some(link_output) = &plan.link_output {
        let (bin, args) = build_linker_args(global, build, &plan.link_inputs, link_output);
        println!("  link:");
        println!("    - {}", shell_join(&bin, &args));
    }

    if build.run {
        if let Some(link_output) = &plan.link_output {
            println!("  run:");
            println!(
                "    - {}",
                shell_join(&link_output.to_string_lossy(), &build.run_args)
            );
        }
    }
}

fn print_dry_run_json(
    global: &Global,
    build: &BuildRequest,
    classified: &[ClassifiedInput],
    plan: &BuildPlan,
) {
    let mut text = String::new();
    text.push('{');

    append_json_field(&mut text, "mode", &json_string(build_mode_label(build)));
    text.push(',');
    append_json_field(&mut text, "emit", &json_string(&render_emit_spec(&build.emit)));
    text.push(',');
    append_json_field(&mut text, "link_only", if build.link_only { "true" } else { "false" });
    text.push(',');
    append_json_field(&mut text, "run", if build.run { "true" } else { "false" });
    text.push(',');
    append_json_field(
        &mut text,
        "freestanding",
        if build.freestanding { "true" } else { "false" },
    );
    text.push(',');
    append_json_field(
        &mut text,
        "no_start_files",
        if build.no_start_files { "true" } else { "false" },
    );
    text.push(',');
    text.push_str("\"entry\":");
    if let Some(entry) = &build.entry {
        text.push_str(&json_string(entry));
    } else {
        text.push_str("null");
    }
    text.push(',');
    text.push_str("\"linker_script\":");
    if let Some(script) = &build.linker_script {
        text.push_str(&json_string(&script.to_string_lossy()));
    } else {
        text.push_str("null");
    }
    text.push(',');
    text.push_str("\"run_args\":");
    text.push('[');
    for (idx, arg) in build.run_args.iter().enumerate() {
        if idx > 0 {
            text.push(',');
        }
        text.push_str(&json_string(arg));
    }
    text.push(']');
    text.push(',');

    text.push_str("\"inputs\":");
    text.push('[');
    for (idx, i) in classified.iter().enumerate() {
        if idx > 0 {
            text.push(',');
        }
        text.push('{');
        append_json_field(
            &mut text,
            "path",
            &json_string(&i.path.to_string_lossy()),
        );
        text.push(',');
        append_json_field(&mut text, "kind", &json_string(i.kind.as_str()));
        text.push('}');
    }
    text.push(']');
    text.push(',');

    let emit_jobs = dry_run_explicit_emit_steps(global, build, classified);
    text.push_str("\"emit_jobs\":");
    text.push('[');
    for (idx, job) in emit_jobs.iter().enumerate() {
        if idx > 0 {
            text.push(',');
        }
        text.push_str(&json_string(job));
    }
    text.push(']');
    text.push(',');

    text.push_str("\"compile\":");
    text.push('[');
    for (idx, job) in plan.compile_jobs.iter().enumerate() {
        if idx > 0 {
            text.push(',');
        }
        text.push('{');
        append_json_field(
            &mut text,
            "input",
            &json_string(&job.input.to_string_lossy()),
        );
        text.push(',');
        append_json_field(&mut text, "kind", &json_string(job.kind.as_str()));
        text.push(',');
        append_json_field(
            &mut text,
            "output",
            &json_string(&job.output.to_string_lossy()),
        );
        text.push(',');

        let command = if job.kind == InputKind::Wave {
            format!(
                "wavec <internal-wave-compile> {} -o {}",
                job.input.display(),
                job.output.display()
            )
        } else {
            let (bin, args) = build_clang_compile_args(global, job);
            shell_join(&bin, &args)
        };

        append_json_field(&mut text, "command", &json_string(&command));
        text.push('}');
    }
    text.push(']');
    text.push(',');

    text.push_str("\"link\":");
    if let Some(link_output) = &plan.link_output {
        let (bin, args) = build_linker_args(global, build, &plan.link_inputs, link_output);
        text.push('{');
        append_json_field(&mut text, "output", &json_string(&link_output.to_string_lossy()));
        text.push(',');
        append_json_field(&mut text, "command", &json_string(&shell_join(&bin, &args)));
        text.push('}');
    } else {
        text.push_str("null");
    }

    text.push('}');
    println!("{}", text);
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn append_json_field(buf: &mut String, key: &str, raw_json_value: &str) {
    buf.push('"');
    buf.push_str(key);
    buf.push_str("\":");
    buf.push_str(raw_json_value);
}

fn shell_join(bin: &str, args: &[String]) -> String {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push(shell_quote(bin));
    for arg in args {
        parts.push(shell_quote(arg));
    }
    parts.join(" ")
}

fn shell_quote(s: &str) -> String {
    if s.is_empty() {
        return "''".to_string();
    }

    if s.chars().all(|c| {
        c.is_ascii_alphanumeric()
            || matches!(c, '_' | '-' | '.' | '/' | ':' | '=' | '+' | ',' )
    }) {
        return s.to_string();
    }

    let mut out = String::from("'");
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

fn build_mode_label(build: &BuildRequest) -> &'static str {
    if build.emit.is_check() {
        return "check";
    }
    if build.link_only {
        return "link-only";
    }
    if build.run {
        return "build+run";
    }
    if build.emit.contains(EmitKind::Bin) {
        return "build";
    }
    "compile-only"
}

fn render_emit_spec(spec: &EmitSpec) -> String {
    match spec {
        EmitSpec::Check => "check".to_string(),
        EmitSpec::Set(set) => set
            .iter()
            .map(|k| match k {
                EmitKind::Ast => "ast",
                EmitKind::Ir => "ir",
                EmitKind::Bc => "bc",
                EmitKind::Asm => "asm",
                EmitKind::Obj => "obj",
                EmitKind::Bin => "bin",
            })
            .collect::<Vec<_>>()
            .join(","),
    }
}

fn host_target_triple() -> String {
    let arch = env::consts::ARCH;
    let os_part = match env::consts::OS {
        "linux" => "unknown-linux-gnu".to_string(),
        "macos" => "apple-darwin".to_string(),
        "windows" => "pc-windows-msvc".to_string(),
        other => format!("unknown-{}", other),
    };
    format!("{}-{}", arch, os_part)
}

fn supported_targets() -> &'static [&'static str] {
    &[
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "x86_64-apple-darwin",
        "aarch64-apple-darwin",
    ]
}

fn ensure_supported_target(target: &str) -> Result<(), CliError> {
    if target == host_target_triple() || supported_targets().iter().any(|t| *t == target) {
        return Ok(());
    }

    Err(CliError::usage(format!(
        "unsupported target '{}': see `wavec print target-list`",
        target
    )))
}

fn cpu_list_for_target(target: &str) -> Vec<&'static str> {
    if target.starts_with("x86_64-") {
        vec!["generic", "x86-64", "x86-64-v2", "x86-64-v3"]
    } else if target.starts_with("aarch64-") {
        vec!["generic", "cortex-a53", "cortex-a72", "apple-m1"]
    } else {
        vec!["generic"]
    }
}

fn target_features_for_target(target: &str) -> Vec<&'static str> {
    if target.starts_with("x86_64-") {
        vec!["sse2", "sse4.1", "avx", "avx2"]
    } else if target.starts_with("aarch64-") {
        vec!["neon", "fp", "crypto"]
    } else {
        vec![]
    }
}

fn detect_clang_sysroot() -> Option<String> {
    let out = ProcessCommand::new("clang")
        .arg("--print-sysroot")
        .output()
        .ok()?;

    if !out.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Some(text)
}

pub fn print_usage() {
    println!(
        "\n{} {}",
        "Usage:".color("255,71,71"),
        "wavec [global-options] <command> [command-options]"
    );
}

pub fn print_version() {
    let os = format!("({})", get_os_pretty_name()).color("117,117,117");

    println!(
        "{} {} {}",
        "wavec".color("2,161,47"),
        version::version().color("2,161,47"),
        os
    );

    if let Some(backend) = llvm::backend() {
        println!("  backend: {}", backend.color("117,117,117"));
    } else {
        println!("{}", "  backend: unknown backend".color("117,117,117"));
    }
}

pub fn print_help() {
    println!("{}", "Wave Compiler".color("145,161,2"));
    print_usage();

    println!("\nCommands:");
    println!(
        "  {:<22} {}",
        "build <input...>".color("38,139,235"),
        "Build/check/link/run pipeline (flag-driven)"
    );
    println!(
        "  {:<22} {}",
        "check <file>".color("38,139,235"),
        "Alias: build <file> --emit=check"
    );
    println!(
        "  {:<22} {}",
        "run <file>".color("38,139,235"),
        "Alias: build <file> --run (supports `-- <args...>`)"
    );
    println!(
        "  {:<22} {}",
        "print <item>".color("38,139,235"),
        "Print compiler/toolchain capability item"
    );
    println!(
        "  {:<22} {}",
        "install std".color("38,139,235"),
        "Install Wave standard library"
    );
    println!(
        "  {:<22} {}",
        "update std".color("38,139,235"),
        "Update Wave standard library"
    );
    println!(
        "  {:<22} {}",
        "--version".color("38,139,235"),
        "Show version"
    );
    println!("  {:<22} {}", "--help".color("38,139,235"), "Show help");

    println!("\nBuild options:");
    println!(
        "  {:<24} {}",
        "--emit=<kinds>".color("38,139,235"),
        "check, ast, ir, bc, asm, obj, bin (check must be alone)"
    );
    println!(
        "  {:<24} {}",
        "--input-type=<kind>".color("38,139,235"),
        "wave, ir, bc, asm, obj (forced type for all inputs)"
    );
    println!(
        "  {:<24} {}",
        "--link-only".color("38,139,235"),
        "Link object inputs only (requires emit=bin)"
    );
    println!(
        "  {:<24} {}",
        "--run".color("38,139,235"),
        "Run linked binary (requires emit includes bin)"
    );
    println!(
        "  {:<24} {}",
        "-- <args...>".color("38,139,235"),
        "Forward run-time arguments to executable (with --run)"
    );
    println!(
        "  {:<24} {}",
        "--freestanding".color("38,139,235"),
        "Kernel/OS-style link defaults (no default libc/libm)"
    );
    println!(
        "  {:<24} {}",
        "--entry <symbol>".color("38,139,235"),
        "Set linker entry symbol (link stage only)"
    );
    println!(
        "  {:<24} {}",
        "--linker-script <path>".color("38,139,235"),
        "Pass linker script path via -Wl,-T,<path>"
    );
    println!(
        "  {:<24} {}",
        "--no-start-files".color("38,139,235"),
        "Pass -nostartfiles to linker (link stage only)"
    );
    println!("  {:<24} {}", "-o <file>".color("38,139,235"), "Output file");
    println!(
        "  {:<24} {}",
        "--out-dir <dir>".color("38,139,235"),
        "Output directory for emitted artifacts"
    );
    println!(
        "  {:<24} {}",
        "--target-dir <dir>".color("38,139,235"),
        "Intermediate/default artifact root"
    );
    println!(
        "  {:<24} {}",
        "--dry-run".color("38,139,235"),
        "Plan only, no compile/link/exec"
    );
    println!(
        "  {:<24} {}",
        "--error-format=...".color("38,139,235"),
        "human, json"
    );

    println!("\nLink mode options:");
    println!(
        "  {:<24} {}",
        "--shared".color("38,139,235"),
        "Build shared output (conflicts with --run)"
    );
    println!(
        "  {:<24} {}",
        "--static".color("38,139,235"),
        "Request static link mode"
    );
    println!("  {:<24} {}", "--pie".color("38,139,235"), "Enable PIE mode");
    println!(
        "  {:<24} {}",
        "--no-pie".color("38,139,235"),
        "Disable PIE mode"
    );

    println!("\nGlobal options:");
    println!(
        "  {:<24} {}",
        "-O0..-O3/-Os/-Oz/-Ofast".color("38,139,235"),
        "Optimization level"
    );
    println!(
        "  {:<24} {}",
        "--debug-wave=...".color("38,139,235"),
        "tokens,ast,ir,mc,hex,all"
    );
    println!(
        "  {:<24} {}",
        "--link=<lib>".color("38,139,235"),
        "Link library"
    );
    println!(
        "  {:<24} {}",
        "-L <path>".color("38,139,235"),
        "Library search path"
    );
    println!(
        "  {:<24} {}",
        "--dep-root=<path>".color("38,139,235"),
        "Dependency root directory"
    );
    println!(
        "  {:<24} {}",
        "--dep=<name>=<path>".color("38,139,235"),
        "Explicit dependency mapping"
    );

    println!("\nLLVM/backend options:");
    println!(
        "  {:<24} {}",
        "--target=<triple>".color("38,139,235"),
        "Target triple"
    );
    println!(
        "  {:<24} {}",
        "--cpu=<name>".color("38,139,235"),
        "Target CPU"
    );
    println!(
        "  {:<24} {}",
        "--features=<csv>".color("38,139,235"),
        "Target features"
    );
    println!(
        "  {:<24} {}",
        "--abi=<name>".color("38,139,235"),
        "Target ABI"
    );
    println!(
        "  {:<24} {}",
        "--sysroot=<path>".color("38,139,235"),
        "Sysroot path"
    );
    println!(
        "  {:<24} {}",
        "-C linker=<path>".color("38,139,235"),
        "Override linker executable"
    );
    println!(
        "  {:<24} {}",
        "-C link-arg=<arg>".color("38,139,235"),
        "Append raw linker argument"
    );
    println!(
        "  {:<24} {}",
        "-C relocation-model=<m>".color("38,139,235"),
        "relocation model for compatibility checks"
    );
    println!(
        "  {:<24} {}",
        "-C no-default-libs".color("38,139,235"),
        "Disable automatic -lc -lm"
    );
}
