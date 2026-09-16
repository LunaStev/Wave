// SPDX-License-Identifier: MPL-2.0
use super::coff;
use std::path::{Path, PathBuf};

fn value<'a>(arg: &'a str, prefix: &str) -> Option<&'a str> {
    arg.get(..prefix.len())
        .filter(|v| v.eq_ignore_ascii_case(prefix))
        .map(|_| &arg[prefix.len()..])
}
fn find_library(name: &str, paths: &[PathBuf]) -> Option<PathBuf> {
    let name = name.trim_matches('"');
    let file = if name.to_ascii_lowercase().ends_with(".lib") {
        name.to_string()
    } else {
        format!("{name}.lib")
    };
    let direct = PathBuf::from(&file);
    if direct.is_file() {
        return Some(direct);
    }
    paths.iter().map(|p| p.join(&file)).find(|p| p.is_file())
}

/// Inspect explicitly resolved inputs before either linker is launched. Object
/// directives are still interpreted by the linker; no claim of LTO validation.
pub fn validate_arguments(target: &str, args: &[String]) -> Result<(), String> {
    let mut paths: Vec<_> = args
        .iter()
        .filter_map(|a| value(a, "/LIBPATH:"))
        .map(PathBuf::from)
        .collect();
    if let Some(lib) = std::env::var_os("LIB") {
        paths.extend(
            lib.to_string_lossy()
                .split(';')
                .filter(|p| !p.is_empty())
                .map(PathBuf::from),
        );
    }
    let no_defaults = args.iter().any(|a| a.eq_ignore_ascii_case("/NODEFAULTLIB"));
    let mut files = Vec::new();
    let mut required = Vec::new();
    for arg in args {
        if let Some(lib) = value(arg, "/DEFAULTLIB:") {
            if !no_defaults {
                required.push(lib);
            }
        } else if let Some(lib) = value(arg, "/WHOLEARCHIVE:") {
            required.push(lib);
        } else if arg.starts_with('@') {
            return Err(
                "MSVC response-file input cannot be inspected; supply inputs directly".into(),
            );
        } else if arg.starts_with('/') && arg.contains(':') && !Path::new(arg).is_file() {
            continue;
        } else {
            match Path::new(arg)
                .extension()
                .and_then(|e| e.to_str())
                .map(str::to_ascii_lowercase)
                .as_deref()
            {
                Some("lib") => required.push(arg),
                Some("obj" | "o" | "a" | "bc") => files.push(PathBuf::from(arg)),
                _ => {}
            }
        }
    }
    for lib in required {
        let file = find_library(lib,&paths).ok_or_else(||format!(
            "MSVC library '{lib}' for {target} was not found; provide matching Windows SDK UM/UCRT and VC libraries with -L, LIB, or WindowsSdkDir/WindowsSDKVersion and VCToolsInstallDir. Searched: {}",
            paths.iter().map(|p|p.display().to_string()).collect::<Vec<_>>().join("; ")
        ))?;
        if !files.contains(&file) {
            files.push(file);
        }
    }
    for file in files {
        coff::validate_file(&file, target)?;
    }
    Ok(())
}

/// Object inputs are supplied separately so inspection never depends on their
/// filename extension. The argument pass also resolves named libraries.
pub fn validate_link_inputs(
    target: &str,
    objects: &[String],
    args: &[String],
) -> Result<(), String> {
    for object in objects {
        coff::validate_file(Path::new(object), target)?;
    }
    validate_arguments(target, args)
}
