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

use crate::ast::{ASTNode};
use crate::{parse_syntax_only, ParseError};
use error::error::{WaveError, WaveErrorKind};
use lexer::Lexer;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub struct ImportedUnit {
    pub abs_path: PathBuf,
    pub ast: Vec<ASTNode>,
}

#[derive(Debug, Clone, Default)]
pub struct ImportConfig {
    pub dep_roots: Vec<PathBuf>,
    pub dep_packages: HashMap<String, PathBuf>,
}

pub fn local_import_unit(
    path: &str,
    already_imported: &mut HashSet<String>,
    base_dir: &Path,
) -> Result<ImportedUnit, WaveError> {
    local_import_unit_with_config(path, already_imported, base_dir, &ImportConfig::default())
}

pub fn local_import_unit_with_config(
    path: &str,
    already_imported: &mut HashSet<String>,
    base_dir: &Path,
    config: &ImportConfig,
) -> Result<ImportedUnit, WaveError> {
    if path.trim().is_empty() {
        return Err(WaveError::new(
            WaveErrorKind::SyntaxError("Empty import path".to_string()),
            "import path cannot be empty",
            "<main>",
            0,
            0,
        ));
    }

    if path.starts_with("std::") {
        return std_import_unit(path, already_imported);
    }

    if path.contains("::") {
        return external_import_unit(path, already_imported, config);
    }

    let target_file_name = if path.ends_with(".wave") {
        path.to_string()
    } else {
        format!("{}.wave", path)
    };

    let found_path = base_dir.join(&target_file_name);
    if !found_path.exists() || !found_path.is_file() {
        return Err(WaveError::new(
            WaveErrorKind::SyntaxError("File not found".to_string()),
            format!("Could not find import target '{}'", target_file_name),
            target_file_name.clone(),
            0,
            0,
        ));
    }

    parse_wave_file(&found_path, &target_file_name, already_imported)
}

pub fn local_import(
    path: &str,
    already_imported: &mut HashSet<String>,
    base_dir: &Path,
) -> Result<Vec<ASTNode>, WaveError> {
    Ok(local_import_unit_with_config(path, already_imported, base_dir, &ImportConfig::default())?.ast)
}

pub fn local_import_with_config(
    path: &str,
    already_imported: &mut HashSet<String>,
    base_dir: &Path,
    config: &ImportConfig,
) -> Result<Vec<ASTNode>, WaveError> {
    Ok(local_import_unit_with_config(path, already_imported, base_dir, config)?.ast)
}

fn resolve_external_package_root(package: &str, config: &ImportConfig) -> Result<Option<PathBuf>, Vec<PathBuf>> {
    if let Some(path) = config.dep_packages.get(package) {
        return Ok(Some(path.clone()));
    }

    let mut matches = Vec::new();
    for root in &config.dep_roots {
        let candidate = root.join(package);
        if candidate.is_dir() {
            matches.push(candidate);
        }
    }

    match matches.len() {
        0 => Ok(None),
        1 => Ok(matches.into_iter().next()),
        _ => Err(matches),
    }
}

fn external_import_unit(
    path: &str,
    already_imported: &mut HashSet<String>,
    config: &ImportConfig,
) -> Result<ImportedUnit, WaveError> {
    let mut parts = path.split("::");
    let package = parts.next().unwrap_or("").trim();
    let module_parts: Vec<&str> = parts.collect();

    if package.is_empty()
        || module_parts.is_empty()
        || module_parts.iter().any(|s| s.trim().is_empty())
    {
        return Err(
            WaveError::new(
                WaveErrorKind::SyntaxError("Invalid external import path".to_string()),
                format!(
                    "invalid external import '{}': expected `package::module::path`",
                    path
                ),
                path,
                0,
                0,
            )
            .with_help("use at least two segments, for example: import(\"math::vector::ops\")"),
        );
    }

    let package_root = match resolve_external_package_root(package, config) {
        Ok(Some(root)) => root,
        Ok(None) => {
            let mut err = WaveError::new(
                WaveErrorKind::SyntaxError("External dependency not found".to_string()),
                format!(
                    "could not resolve external package '{}' for import '{}'",
                    package, path
                ),
                path,
                0,
                0,
            )
            .with_help("provide dependency paths with `--dep-root <dir>` or `--dep <name>=<path>`")
            .with_suggestion("example: wavec run main.wave --dep-root .vex/dep")
            .with_suggestion(format!(
                "example: wavec run main.wave --dep {}=/abs/path/to/{}",
                package, package
            ));

            if !config.dep_roots.is_empty() {
                let roots = config
                    .dep_roots
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                err = err.with_note(format!("currently configured dependency roots: {}", roots));
            }
            return Err(err);
        }
        Err(candidates) => {
            let roots = candidates
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");

            return Err(
                WaveError::new(
                    WaveErrorKind::SyntaxError("Ambiguous external package root".to_string()),
                    format!(
                        "package '{}' is found in multiple dependency roots; resolution is ambiguous",
                        package
                    ),
                    path,
                    0,
                    0,
                )
                .with_note(format!("candidates: {}", roots))
                .with_help("pin the package path explicitly with `--dep <name>=<path>`"),
            );
        }
    };

    if !package_root.is_dir() {
        return Err(
            WaveError::new(
                WaveErrorKind::SyntaxError("Dependency path is not a directory".to_string()),
                format!(
                    "configured dependency path for package '{}' is invalid: {}",
                    package,
                    package_root.display()
                ),
                path,
                0,
                0,
            )
            .with_help("pass a valid directory path via `--dep <name>=<path>`"),
        );
    }

    let module_rel = module_parts.join("/");
    let module_file = if module_rel.ends_with(".wave") {
        module_rel
    } else {
        format!("{}.wave", module_rel)
    };

    let candidates = [
        package_root.join(&module_file),
        package_root.join("src").join(&module_file),
    ];

    for candidate in &candidates {
        if candidate.exists() && candidate.is_file() {
            return parse_wave_file(candidate, path, already_imported);
        }
    }

    let searched = candidates
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    Err(
        WaveError::new(
            WaveErrorKind::SyntaxError("File not found".to_string()),
            format!(
                "could not find external import target '{}' in package '{}'",
                path, package
            ),
            path,
            0,
            0,
        )
        .with_note(format!("searched paths: {}", searched))
        .with_help("check package/module names or pass an explicit path with `--dep <name>=<path>`"),
    )
}

fn std_import_unit(path: &str, already_imported: &mut HashSet<String>) -> Result<ImportedUnit, WaveError> {
    let rel = path.strip_prefix("std::").unwrap();
    if rel.trim().is_empty() {
        return Err(WaveError::new(
            WaveErrorKind::SyntaxError("Empty std import".to_string()),
            "std import path cannot be empty (example: import(\"std::io::format\"))",
            path,
            0,
            0,
        ));
    }

    let std_root = std_root_dir(path)?;

    // std::io::format -> ~/.wave/lib/wave/std/io/format.wave
    let rel_path = rel.replace("::", "/");
    let found_path = std_root.join(format!("{}.wave", rel_path));

    if !found_path.exists() || !found_path.is_file() {
        return Err(WaveError::new(
            WaveErrorKind::SyntaxError("File not found".to_string()),
            format!("Could not find std import target '{}'", found_path.display()),
            path,
            0,
            0,
        ));
    }

    parse_wave_file(&found_path, path, already_imported)
}

fn std_root_dir(import_path: &str) -> Result<PathBuf, WaveError> {
    let home = std::env::var("HOME").map_err(|_| {
        WaveError::new(
            WaveErrorKind::SyntaxError("std not installed".to_string()),
            "HOME env not set; cannot locate std at ~/.wave/lib/wave/std",
            import_path,
            0,
            0,
        )
    })?;

    Ok(PathBuf::from(home).join(".wave/lib/wave/std"))
}

fn parse_wave_file(
    found_path: &Path,
    display_name: &str,
    already_imported: &mut HashSet<String>,
) -> Result<ImportedUnit, WaveError> {
    let abs_path = found_path.canonicalize().map_err(|e| {
        WaveError::new(
            WaveErrorKind::SyntaxError("Canonicalization failed".to_string()),
            format!("Failed to canonicalize path: {}", e),
            display_name,
            0,
            0,
        )
    })?;

    let abs_path_str = abs_path
        .to_str()
        .ok_or_else(|| {
            WaveError::new(
                WaveErrorKind::UnexpectedChar('?'),
                "Invalid path encoding",
                display_name,
                0,
                0,
            )
        })?
        .to_string();

    if already_imported.contains(&abs_path_str) {
        return Ok(ImportedUnit { abs_path, ast: vec![] });
    }
    already_imported.insert(abs_path_str);

    let content = std::fs::read_to_string(&abs_path).map_err(|e| {
        WaveError::new(
            WaveErrorKind::SyntaxError("Read error".to_string()),
            format!("Failed to read '{}': {}", abs_path.display(), e),
            display_name,
            0,
            0,
        )
    })?;

    let mut lexer = Lexer::new_with_file(&content, abs_path.display().to_string());
    let tokens = lexer.tokenize()?;

    let ast = parse_syntax_only(&tokens).map_err(|e| {
        let (kind, phase, code) = match &e {
            ParseError::Syntax(_) => (WaveErrorKind::SyntaxError(e.message().to_string()), "syntax", "E2001"),
            ParseError::Semantic(_) => (WaveErrorKind::InvalidStatement(e.message().to_string()), "semantic", "E3001"),
        };

        let mut we = WaveError::new(
            kind,
            format!("{} validation failed for '{}': {}", phase, abs_path.display(), e.message()),
            display_name,
            e.line().max(1),
            e.column().max(1),
        )
        .with_code(code)
        .with_source_code(content.clone());

        if let Some(ctx) = e.context() {
            we = we.with_context(ctx.to_string());
        }
        if !e.expected().is_empty() {
            we = we.with_expected_many(e.expected().iter().cloned());
        }
        if let Some(found) = e.found() {
            we = we.with_found(found.to_string());
        }
        if let Some(note) = e.note() {
            we = we.with_note(note.to_string());
        }
        if let Some(help) = e.help() {
            we = we.with_help(help.to_string());
        }

        we
    })?;

    Ok(ImportedUnit { abs_path, ast })
}
