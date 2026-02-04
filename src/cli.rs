use crate::errors::CliError;
use crate::flags::{validate_opt_flag, DebugFlags, LinkFlags};
use crate::{runner, std as wave_std, version};

use std::{env, path::PathBuf};
use llvm_temporary::backend;
use utils::colorex::*;
use crate::version::get_os_pretty_name;

#[derive(Debug)]
enum Command {
    Run { file: PathBuf },
    Img { file: PathBuf },
    BuildExe { file: PathBuf },
    BuildObj { file: PathBuf },

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
                runner::run_wave_file(&file, &global.opt, &global.debug, &global.link);
            }
            Ok(())
        }

        Command::Img { file } => {
            unsafe {
                runner::img_wave_file(&file);
            }
            Ok(())
        }

        Command::BuildExe { file } => {
            unsafe {
                runner::build_wave_file(&file, &global.opt, &global.debug, &global.link);
            }
            Ok(())
        }

        Command::BuildObj { file } => {
            let obj = unsafe {
                runner::object_build_wave_file(&file, &global.opt, &global.debug)
            };
            println!("{}", obj);
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

        rest.push(a.clone());
        i += 1;
    }

    Ok((g, rest))
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
        "img" => parse_img(args),
        "build" => parse_build(args),

        "install" => parse_install(args),
        "update" => parse_update(args),

        other => Err(CliError::usage(format!("unknown command: {}", other))),
    }
}

fn parse_run(args: &[String]) -> Result<Command, CliError> {
    // legacy: run --img file
    let mut img = false;
    let mut file: Option<PathBuf> = None;

    for a in args {
        if a == "--img" {
            img = true;
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

    if img {
        Ok(Command::Img { file })
    } else {
        Ok(Command::Run { file })
    }
}

fn parse_img(args: &[String]) -> Result<Command, CliError> {
    let file = args.get(0).ok_or_else(|| CliError::usage("usage: wavec img <file>"))?;
    if args.len() > 1 {
        return Err(CliError::usage(format!("unexpected extra argument: {}", args[1])));
    }
    Ok(Command::Img { file: PathBuf::from(file) })
}

fn parse_build(args: &[String]) -> Result<Command, CliError> {
    // build <file>
    // build -o <file>   (object only)
    let mut emit_obj = false;
    let mut file: Option<PathBuf> = None;

    for a in args {
        match a.as_str() {
            "-o" | "--obj" => emit_obj = true,
            _ if a.starts_with('-') => {
                return Err(CliError::usage(format!("unknown option for build: {}", a)));
            }
            _ => {
                if file.is_none() {
                    file = Some(PathBuf::from(a));
                } else {
                    return Err(CliError::usage(format!("unexpected extra argument: {}", a)));
                }
            }
        }
    }

    let file = file.ok_or_else(|| CliError::usage("usage: wavec build <file>"))?;

    if emit_obj {
        Ok(Command::BuildObj { file })
    } else {
        Ok(Command::BuildExe { file })
    }
}

fn parse_install(args: &[String]) -> Result<Command, CliError> {
    let target = args.get(0).ok_or_else(|| CliError::usage("usage: wavec install <target>"))?;
    if args.len() > 1 {
        return Err(CliError::usage(format!("unexpected extra argument: {}", args[1])));
    }

    match target.as_str() {
        "std" => Ok(Command::StdInstall),
        _ => Err(CliError::usage(format!("unknown install target: {}", target))),
    }
}

fn parse_update(args: &[String]) -> Result<Command, CliError> {
    let target = args.get(0).ok_or_else(|| CliError::usage("usage: wavec update <target>"))?;
    if args.len() > 1 {
        return Err(CliError::usage(format!("unexpected extra argument: {}", args[1])));
    }

    match target.as_str() {
        "std" => Ok(Command::StdUpdate),
        _ => Err(CliError::usage(format!("unknown update target: {}", target))),
    }
}

pub fn print_usage() {
    eprintln!(
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
    println!("  {:<18} {}", "run <file>".color("38,139,235"), "Execute Wave file");
    println!("  {:<18} {}", "img <file>".color("38,139,235"), "Build & run image via QEMU (legacy: run --img <file>)");
    println!("  {:<18} {}", "build <file>".color("38,139,235"), "Compile Wave file (exe)");
    println!("  {:<18} {}", "build -o <file>".color("38,139,235"), "Compile Wave file (object; prints path)");
    println!("  {:<18} {}", "install std".color("38,139,235"), "Install Wave standard library");
    println!("  {:<18} {}", "update std".color("38,139,235"), "Update Wave standard library");
    println!("  {:<18} {}", "--version".color("38,139,235"), "Show version");
    println!("  {:<18} {}", "--help".color("38,139,235"), "Show help");

    println!("\nGlobal options (anywhere):");
    println!("  {:<22} {}", "-O0..-O3/-Oz/-Ofast".color("38,139,235"), "Optimization level");
    println!("  {:<22} {}", "--debug-wave=...".color("38,139,235"), "tokens,ast,ir,mc,hex,all (comma ok)");
    println!("  {:<22} {}", "--link=<lib>".color("38,139,235"), "Link library");
    println!("  {:<22} {}", "-L<path> / -L <path>".color("38,139,235"), "Library search path");
}
