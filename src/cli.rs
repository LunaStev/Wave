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

use crate::errors::CliError;
use crate::flags::{validate_opt_flag, DebugFlags, DepFlags, DepPackage, LinkFlags};
use crate::{runner, std as wave_std, version};

use crate::version::get_os_pretty_name;
use llvm::backend;
use std::{env, path::PathBuf};
use utils::colorex::*;

#[derive(Debug)]
enum Command {
    Run {
        file: PathBuf,
    },
    Build {
        file: PathBuf,
        output: Option<PathBuf>,
        compile_only: bool,
    },

    StdInstall,
    StdUpdate,

    Help,
    Version,
}

#[derive(Default)]
struct Global {
    opt: String,
    debug: DebugFlags,
    link: LinkFlags,
    dep: DepFlags,
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

fn dispatch(global: Global, cmd: Command) -> Result<(), CliError> {
    match cmd {
        Command::Version => {
            print_version();
            Ok(())
        }

        Command::Help => {
            print_help();
            Ok(())
        }

        Command::Run { file } => {
            unsafe {
                runner::run_wave_file(&file, &global.opt, &global.debug, &global.link, &global.dep);
            }
            Ok(())
        }

        Command::Build {
            file,
            output,
            compile_only,
        } => {
            unsafe {
                if compile_only {
                    let out = runner::object_build_wave_file(
                        &file,
                        &global.opt,
                        &global.debug,
                        &global.dep,
                        output.as_deref(),
                    );
                    println!("{}", out);
                } else {
                    runner::build_wave_file(
                        &file,
                        &global.opt,
                        &global.debug,
                        &global.link,
                        &global.dep,
                        output.as_deref(),
                    );
                }
            }
            Ok(())
        }

        Command::StdInstall => wave_std::std_install(),
        Command::StdUpdate => wave_std::std_update(),
    }
}

fn parse_global(args: Vec<String>) -> Result<(Global, Vec<String>), CliError> {
    let mut g = Global {
        opt: "-O0".to_string(),
        debug: DebugFlags::default(),
        link: LinkFlags::default(),
        dep: DepFlags::default(),
    };

    let mut rest: Vec<String> = Vec::new();
    let mut i = 0usize;

    while i < args.len() {
        let a = &args[i];

        if a == "--" {
            rest.extend_from_slice(&args[i + 1..]);
            break;
        }

        // -O*
        if a.starts_with("-O") {
            if !validate_opt_flag(a) {
                return Err(CliError::usage(format!("invalid optimization flag: {}", a)));
            }
            g.opt = a.clone();
            i += 1;
            continue;
        }

        // --debug-wave=...
        if let Some(mode) = a.strip_prefix("--debug-wave=") {
            g.debug.apply(mode);
            i += 1;
            continue;
        }

        // --debug-wave <mode>
        if a == "--debug-wave" {
            let mode = args.get(i + 1).ok_or_else(|| {
                CliError::usage("missing value: --debug-wave <tokens|ast|ir|mc|hex|all|...>")
            })?;
            g.debug.apply(mode);
            i += 2;
            continue;
        }

        // --link=lib
        if let Some(lib) = a.strip_prefix("--link=") {
            g.link.libs.push(lib.to_string());
            i += 1;
            continue;
        }

        // --link lib
        if a == "--link" {
            let lib = args
                .get(i + 1)
                .ok_or_else(|| CliError::usage("missing value: --link <lib>"))?;
            g.link.libs.push(lib.to_string());
            i += 2;
            continue;
        }

        // -L<path> or -L <path>
        if let Some(p) = a.strip_prefix("-L") {
            if p.is_empty() {
                let path = args
                    .get(i + 1)
                    .ok_or_else(|| CliError::usage("missing value: -L <path>"))?;
                g.link.paths.push(path.to_string());
                i += 2;
            } else {
                g.link.paths.push(p.to_string());
                i += 1;
            }
            continue;
        }

        // --dep-root=<path>
        if let Some(path) = a.strip_prefix("--dep-root=") {
            if path.trim().is_empty() {
                return Err(CliError::usage("missing value: --dep-root <path>"));
            }
            g.dep.roots.push(path.to_string());
            i += 1;
            continue;
        }

        // --dep-root <path>
        if a == "--dep-root" {
            let path = args
                .get(i + 1)
                .ok_or_else(|| CliError::usage("missing value: --dep-root <path>"))?;
            g.dep.roots.push(path.to_string());
            i += 2;
            continue;
        }

        // --dep=<name>=<path>
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

        // --dep <name>=<path>
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

fn parse_command(rest: Vec<String>) -> Result<Command, CliError> {
    if rest.is_empty() {
        return Err(CliError::usage("not enough arguments"));
    }

    let cmd = rest[0].as_str();
    let args = &rest[1..];

    match cmd {
        "--help" | "-h" | "help" => Ok(Command::Help),
        "--version" | "-V" | "version" => Ok(Command::Version),

        "run" => parse_run(args),
        "build" => parse_build(args),

        "install" => parse_install(args),
        "update" => parse_update(args),

        other => Err(CliError::usage(format!("unknown command: {}", other))),
    }
}

fn parse_run(args: &[String]) -> Result<Command, CliError> {
    let mut file: Option<PathBuf> = None;

    for a in args {
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

    Ok(Command::Run { file })
}

fn parse_build(args: &[String]) -> Result<Command, CliError> {
    // build <file> [-o <file>] [-c]
    let mut compile_only = false;
    let mut output: Option<PathBuf> = None;
    let mut file: Option<PathBuf> = None;
    let mut i = 0usize;

    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "-c" => {
                compile_only = true;
                i += 1;
            }
            "-o" => {
                let Some(v) = args.get(i + 1) else {
                    return Err(CliError::usage("missing value: -o <file>"));
                };
                if v.starts_with('-') {
                    return Err(CliError::usage(format!("invalid output file: {}", v)));
                }
                output = Some(PathBuf::from(v));
                i += 2;
            }
            "--output" => {
                let Some(v) = args.get(i + 1) else {
                    return Err(CliError::usage("missing value: --output <file>"));
                };
                if v.starts_with('-') {
                    return Err(CliError::usage(format!("invalid output file: {}", v)));
                }
                output = Some(PathBuf::from(v));
                i += 2;
            }
            _ if a.starts_with("--output=") => {
                let v = a.trim_start_matches("--output=");
                if v.trim().is_empty() {
                    return Err(CliError::usage("missing value: --output=<file>"));
                }
                output = Some(PathBuf::from(v));
                i += 1;
            }
            _ if a.starts_with('-') => {
                return Err(CliError::usage(format!("unknown option for build: {}", a)));
            }
            _ => {
                if file.is_none() {
                    file = Some(PathBuf::from(a));
                } else {
                    return Err(CliError::usage(format!("unexpected extra argument: {}", a)));
                }
                i += 1;
            }
        }
    }

    let file = file.ok_or_else(|| CliError::usage("usage: wavec build <file> [-o <file>] [-c]"))?;

    Ok(Command::Build {
        file,
        output,
        compile_only,
    })
}

fn parse_install(args: &[String]) -> Result<Command, CliError> {
    let target = args
        .get(0)
        .ok_or_else(|| CliError::usage("usage: wavec install <target>"))?;
    if args.len() > 1 {
        return Err(CliError::usage(format!(
            "unexpected extra argument: {}",
            args[1]
        )));
    }

    match target.as_str() {
        "std" => Ok(Command::StdInstall),
        _ => Err(CliError::usage(format!(
            "unknown install target: {}",
            target
        ))),
    }
}

fn parse_update(args: &[String]) -> Result<Command, CliError> {
    let target = args
        .get(0)
        .ok_or_else(|| CliError::usage("usage: wavec update <target>"))?;
    if args.len() > 1 {
        return Err(CliError::usage(format!(
            "unexpected extra argument: {}",
            args[1]
        )));
    }

    match target.as_str() {
        "std" => Ok(Command::StdUpdate),
        _ => Err(CliError::usage(format!(
            "unknown update target: {}",
            target
        ))),
    }
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

    if let Some(backend) = backend() {
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
        "  {:<20} {}",
        "run <file>".color("38,139,235"),
        "Compile & execute Wave file"
    );
    println!(
        "  {:<20} {}",
        "build <file>".color("38,139,235"),
        "Compile Wave file (executable)"
    );
    println!(
        "  {:<20} {}",
        "install std".color("38,139,235"),
        "Install Wave standard library"
    );
    println!(
        "  {:<20} {}",
        "update std".color("38,139,235"),
        "Update Wave standard library"
    );
    println!(
        "  {:<20} {}",
        "--version".color("38,139,235"),
        "Show version"
    );
    println!("  {:<20} {}", "--help".color("38,139,235"), "Show help");

    println!("\nBuild options:");
    println!(
        "  {:<22} {}",
        "-o <file>".color("38,139,235"),
        "Specify output file name"
    );
    println!(
        "  {:<22} {}",
        "-c".color("38,139,235"),
        "Compile only (emit object file)"
    );
    println!(
        "  {:<22} {}",
        "-O0..-O3/-Os/-Oz/-Ofast".color("38,139,235"),
        "Optimization level"
    );

    println!("\nDebug options:");
    println!(
        "  {:<22} {}",
        "--debug-wave=...".color("38,139,235"),
        "tokens,ast,ir,mc,hex,all (comma ok)"
    );

    println!("\nLink options:");
    println!(
        "  {:<22} {}",
        "--link=<lib>".color("38,139,235"),
        "Link library"
    );
    println!(
        "  {:<22} {}",
        "-L <path>".color("38,139,235"),
        "Library search path"
    );

    println!("\nDependency options:");
    println!(
        "  {:<22} {}",
        "--dep-root=<path>".color("38,139,235"),
        "Dependency root directory (e.g. .vex/dep)"
    );
    println!(
        "  {:<22} {}",
        "--dep=<name>=<path>".color("38,139,235"),
        "Explicit dependency package mapping"
    );
}
