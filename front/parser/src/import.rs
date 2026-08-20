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

//! Target-aware source preprocessing and recursive import expansion.
//!
//! Target attributes are resolved before lexing while preserving line structure
//! for diagnostics. Import expansion tracks canonical paths to detect cycles,
//! retains each source unit for later error mapping, and resolves local,
//! dependency, and standard-library roots through explicit configuration.

use crate::arch;
use crate::ast::ASTNode;
use crate::{parse_syntax_only, ParseError};
use error::error::{WaveError, WaveErrorKind};
use lexer::Lexer;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct TargetConditionContext {
    pub arch: Option<String>,
    pub os: Option<String>,
    pub env: Option<String>,
    pub abi: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct TargetAttrCondition<'a> {
    arch: Option<&'a str>,
    os: Option<&'a str>,
    env: Option<&'a str>,
    abi: Option<&'a str>,
}

impl TargetConditionContext {
    fn actual_value(&self, key: &str) -> String {
        match key {
            "arch" => self
                .arch
                .clone()
                .unwrap_or_else(|| std::env::consts::ARCH.to_string()),
            "os" => self
                .os
                .clone()
                .unwrap_or_else(|| std::env::consts::OS.to_string()),
            "env" => self.env.clone().unwrap_or_default(),
            "abi" => self.abi.clone().unwrap_or_default(),
            _ => String::new(),
        }
    }
}

impl<'a> TargetAttrCondition<'a> {
    fn matches(&self, target: &TargetConditionContext) -> bool {
        for (key, expected) in [
            ("arch", self.arch),
            ("os", self.os),
            ("env", self.env),
            ("abi", self.abi),
        ] {
            if let Some(expected) = expected {
                let actual = target.actual_value(key);
                if normalize_target_value(key, &actual) != normalize_target_value(key, expected) {
                    return false;
                }
            }
        }

        true
    }
}

fn normalize_target_value(key: &str, value: &str) -> String {
    let lower = value.trim().to_ascii_lowercase();
    match key {
        "arch" => arch::canonical_name(&lower),
        "os" => match lower.as_str() {
            "darwin" | "apple" => "macos".to_string(),
            "win32" | "win64" => "windows".to_string(),
            other => other.to_string(),
        },
        "env" => match lower.as_str() {
            "mingw" => "gnu".to_string(),
            other => other.to_string(),
        },
        _ => lower,
    }
}

fn parse_target_attr(line: &str) -> Option<TargetAttrCondition<'_>> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix("#[target(")?.strip_suffix(")]")?;
    let inner = inner.trim();
    if inner.is_empty() {
        return None;
    }

    let mut condition = TargetAttrCondition::default();
    for raw in inner.split(',') {
        let (key, value) = raw.split_once('=')?;
        let key = key.trim();
        let value = parse_attr_string(value.trim())?;

        match key {
            "arch" => condition.arch = Some(value),
            "os" => condition.os = Some(value),
            "env" => condition.env = Some(value),
            "abi" => condition.abi = Some(value),
            _ => return None,
        }
    }

    Some(condition)
}

fn parse_attr_string(value: &str) -> Option<&str> {
    value.strip_prefix('"')?.strip_suffix('"')
}

fn is_supported_target_item_start(line: &str) -> bool {
    fn has_ident_boundary(rest: &str) -> bool {
        match rest.chars().next() {
            None => true,
            Some(c) => !(c.is_ascii_alphanumeric() || c == '_'),
        }
    }

    let trimmed = line.trim_start();
    for kw in [
        "import", "extern", "export", "fun", "struct", "enum", "const", "static", "type", "proto",
    ] {
        if let Some(rest) = trimmed.strip_prefix(kw) {
            if has_ident_boundary(rest) {
                return true;
            }
        }
    }

    false
}

fn scan_target_item_line(
    line: &str,
    in_block_comment: &mut bool,
    depth: &mut i32,
    seen_open: &mut bool,
    saw_semicolon: &mut bool,
) {
    let mut chars = line.chars().peekable();
    let mut in_string = false;
    let mut in_char = false;
    let mut escape = false;

    while let Some(ch) = chars.next() {
        if *in_block_comment {
            if ch == '*' && chars.peek() == Some(&'/') {
                chars.next();
                *in_block_comment = false;
            }
            continue;
        }

        if in_string {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                escape = true;
                continue;
            }
            if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if in_char {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                escape = true;
                continue;
            }
            if ch == '\'' {
                in_char = false;
            }
            continue;
        }

        if ch == '/' {
            if chars.peek() == Some(&'/') {
                break;
            }
            if chars.peek() == Some(&'*') {
                chars.next();
                *in_block_comment = true;
                continue;
            }
        }

        if ch == '"' {
            in_string = true;
            continue;
        }
        if ch == '\'' {
            in_char = true;
            continue;
        }

        if ch == '{' {
            *depth += 1;
            *seen_open = true;
            continue;
        }
        if ch == '}' {
            if *depth > 0 {
                *depth -= 1;
            }
            continue;
        }
        if ch == ';' {
            *saw_semicolon = true;
        }
    }
}

fn consume_target_item(lines: &[&str], mut idx: usize, keep: bool, out: &mut Vec<String>) -> usize {
    let mut depth: i32 = 0;
    let mut seen_open = false;
    let mut in_block_comment = false;

    while idx < lines.len() {
        let line = lines[idx];
        if keep {
            out.push(line.to_string());
        } else {
            out.push(String::new());
        }

        let mut saw_semicolon = false;
        scan_target_item_line(
            line,
            &mut in_block_comment,
            &mut depth,
            &mut seen_open,
            &mut saw_semicolon,
        );

        idx += 1;

        if seen_open {
            if depth == 0 {
                break;
            }
        } else if saw_semicolon {
            break;
        }
    }

    idx
}

pub fn preprocess_target_attrs(source: &str, target: &TargetConditionContext) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut idx: usize = 0;

    while idx < lines.len() {
        let line = lines[idx];
        if let Some(target_attr) = parse_target_attr(line) {
            // Attribute line is removed for parser compatibility,
            // but we keep its line slot to preserve diagnostics.
            out.push(String::new());
            idx += 1;

            let keep_item = target_attr.matches(target);

            // Attribute applies to the next top-level item.
            // Preserve line count for any leading blanks/comments.
            while idx < lines.len() {
                let item_line = lines[idx];
                let trimmed = item_line.trim_start();

                let is_leading_comment = trimmed.starts_with("//")
                    || trimmed.starts_with("/*")
                    || trimmed.starts_with('*')
                    || trimmed.starts_with("*/");

                if trimmed.is_empty() || is_leading_comment {
                    if keep_item {
                        out.push(item_line.to_string());
                    } else {
                        out.push(String::new());
                    }
                    idx += 1;
                    continue;
                }

                if is_supported_target_item_start(trimmed) {
                    idx = consume_target_item(&lines, idx, keep_item, &mut out);
                } else if keep_item {
                    out.push(item_line.to_string());
                    idx += 1;
                } else {
                    out.push(String::new());
                    idx += 1;
                }
                break;
            }
            continue;
        }

        out.push(line.to_string());
        idx += 1;
    }

    let mut processed = out.join("\n");
    if source.ends_with('\n') {
        processed.push('\n');
    }
    processed
}

pub struct ImportedUnit {
    pub abs_path: PathBuf,
    pub ast: Vec<ASTNode>,
    pub source: String,
}

#[derive(Debug, Clone, Default)]
pub struct ImportConfig {
    pub dep_roots: Vec<PathBuf>,
    pub dep_packages: HashMap<String, PathBuf>,
    pub target: TargetConditionContext,
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
        return std_import_unit(path, already_imported, config);
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

    parse_wave_file(&found_path, &target_file_name, already_imported, config)
}

pub fn local_import(
    path: &str,
    already_imported: &mut HashSet<String>,
    base_dir: &Path,
) -> Result<Vec<ASTNode>, WaveError> {
    Ok(
        local_import_unit_with_config(path, already_imported, base_dir, &ImportConfig::default())?
            .ast,
    )
}

pub fn local_import_with_config(
    path: &str,
    already_imported: &mut HashSet<String>,
    base_dir: &Path,
    config: &ImportConfig,
) -> Result<Vec<ASTNode>, WaveError> {
    Ok(local_import_unit_with_config(path, already_imported, base_dir, config)?.ast)
}

fn resolve_external_package_root(
    package: &str,
    config: &ImportConfig,
) -> Result<Option<PathBuf>, Vec<PathBuf>> {
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
        return Err(WaveError::new(
            WaveErrorKind::SyntaxError("Invalid external import path".to_string()),
            format!(
                "invalid external import '{}': expected `package::module::path`",
                path
            ),
            path,
            0,
            0,
        )
        .with_help("use at least two segments, for example: import(\"math::vector::ops\")"));
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

            return Err(WaveError::new(
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
            .with_help("pin the package path explicitly with `--dep <name>=<path>`"));
        }
    };

    if !package_root.is_dir() {
        return Err(WaveError::new(
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
        .with_help("pass a valid directory path via `--dep <name>=<path>`"));
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
            return parse_wave_file(candidate, path, already_imported, config);
        }
    }

    let searched = candidates
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    Err(WaveError::new(
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
    .with_help("check package/module names or pass an explicit path with `--dep <name>=<path>`"))
}

fn std_import_unit(
    path: &str,
    already_imported: &mut HashSet<String>,
    config: &ImportConfig,
) -> Result<ImportedUnit, WaveError> {
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
            format!(
                "Could not find std import target '{}'",
                found_path.display()
            ),
            path,
            0,
            0,
        ));
    }

    parse_wave_file(&found_path, path, already_imported, config)
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
    config: &ImportConfig,
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
        return Ok(ImportedUnit {
            abs_path,
            ast: vec![],
            source: String::new(),
        });
    }
    already_imported.insert(abs_path_str);

    let raw_content = std::fs::read_to_string(&abs_path).map_err(|e| {
        WaveError::new(
            WaveErrorKind::SyntaxError("Read error".to_string()),
            format!("Failed to read '{}': {}", abs_path.display(), e),
            display_name,
            0,
            0,
        )
    })?;
    let content = preprocess_target_attrs(&raw_content, &config.target);

    let mut lexer = Lexer::new_with_file(&content, abs_path.display().to_string());
    let tokens = lexer.tokenize()?;

    let ast = parse_syntax_only(&tokens).map_err(|e| {
        let (kind, phase, code) = match &e {
            ParseError::Syntax(_) => (
                WaveErrorKind::SyntaxError(e.message().to_string()),
                "syntax",
                "E2001",
            ),
            ParseError::Semantic(_) => (
                WaveErrorKind::InvalidStatement(e.message().to_string()),
                "semantic",
                "E3001",
            ),
        };

        let mut we = WaveError::new(
            kind,
            format!(
                "{} validation failed for '{}': {}",
                phase,
                abs_path.display(),
                e.message()
            ),
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

    Ok(ImportedUnit {
        abs_path,
        ast,
        source: content,
    })
}
