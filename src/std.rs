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

//! Installation and update commands for the separately licensed Wave standard library.
//!
//! Only the repository's `std` subtree is fetched. Its manifest is validated
//! before files are copied into the per-user Wave library directory so an
//! unexpected repository layout is not installed as the standard library.

use crate::errors::CliError;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs};

const STD_REPOSITORY: &str = "https://github.com/wavefnd/Wave.git";
const STD_REFERENCE: &str = "master";

pub fn std_install() -> Result<(), CliError> {
    install_or_update_std(false)
}

pub fn std_update() -> Result<(), CliError> {
    install_or_update_std(true)
}

fn install_or_update_std(is_update: bool) -> Result<(), CliError> {
    let install_dir = resolve_std_install_dir()?;

    if install_dir.exists() && !is_update {
        return Err(CliError::StdAlreadyInstalled { path: install_dir });
    }

    let install_parent = install_dir.parent().ok_or_else(|| {
        CliError::CommandFailed(format!(
            "std installation path '{}' has no parent",
            install_dir.display()
        ))
    })?;
    fs::create_dir_all(install_parent)?;

    // Keep staging beside the final directory so the final rename stays on one
    // filesystem. The installed tree is untouched until every check passes.
    let checkout = make_tmp_dir("wave-std-checkout")?;
    let stage_home = make_tmp_dir_in(install_parent, ".std-stage")?;
    let stage_std = stage_home.join(".wave/lib/wave/std");

    let result = (|| {
        let (src_std, source_revision) = fetch_std_from_wave_repo_sparse(&checkout)?;
        validate_std_manifest(&src_std)?;

        copy_dir_all(&src_std, &stage_std)?;
        fs::write(
            stage_std.join("INSTALL_META"),
            format!(
                "repo={}\nref={}\nrevision={}\ncompatibility_revision={}\n",
                STD_REPOSITORY,
                STD_REFERENCE,
                source_revision,
                parser::import::STD_COMPATIBILITY_REVISION
            ),
        )?;

        validate_staged_std(&stage_home, &stage_std)?;
        replace_std_tree(&stage_std, &install_dir)
    })();

    let _ = fs::remove_dir_all(&checkout);
    let _ = fs::remove_dir_all(&stage_home);
    result?;

    if is_update {
        println!("✅ std updated: {}", install_dir.display());
    } else {
        println!("✅ std installed: {}", install_dir.display());
    }

    Ok(())
}

fn fetch_std_from_wave_repo_sparse(checkout: &Path) -> Result<(PathBuf, String), CliError> {
    if !tool_exists("git") {
        return Err(CliError::ExternalToolMissing("git".to_string()));
    }

    run_cmd(
        Command::new("git")
            .arg("clone")
            .arg("--depth")
            .arg("1")
            .arg("--filter=blob:none")
            .arg("--sparse")
            .arg("--branch")
            .arg(STD_REFERENCE)
            .arg(STD_REPOSITORY)
            .arg(checkout),
        "git clone",
    )?;

    run_cmd(
        Command::new("git")
            .arg("-C")
            .arg(checkout)
            .arg("sparse-checkout")
            .arg("set")
            .arg("std"),
        "git sparse-checkout set std",
    )?;

    let source_revision = run_cmd_stdout(
        Command::new("git")
            .arg("-C")
            .arg(checkout)
            .arg("rev-parse")
            .arg("HEAD"),
        "git rev-parse HEAD",
    )?;

    Ok((checkout.join("std"), source_revision))
}

fn validate_std_manifest(std_root: &Path) -> Result<(), CliError> {
    let revision =
        parser::import::std_compatibility_revision(std_root).map_err(CliError::CommandFailed)?;
    let required = parser::import::STD_COMPATIBILITY_REVISION;
    if revision != required {
        return Err(CliError::CommandFailed(format!(
            "downloaded std compatibility revision {}, but this compiler requires {}",
            revision, required
        )));
    }
    Ok(())
}

fn validate_staged_std(stage_home: &Path, stage_std: &Path) -> Result<(), CliError> {
    validate_std_manifest(stage_std)?;

    let mut sources = Vec::new();
    collect_wave_sources(stage_std, &mut sources)?;
    sources.sort();
    if sources.is_empty() {
        return Err(CliError::CommandFailed(
            "downloaded std contains no Wave source files".to_string(),
        ));
    }

    let compiler = env::current_exe()?;
    for source in sources {
        let output = Command::new(&compiler)
            .env("HOME", stage_home)
            .arg("check")
            .arg(&source)
            .output()?;
        if !output.status.success() {
            return Err(CliError::CommandFailed(format!(
                "staged std validation failed for '{}'\nstdout: {}\nstderr: {}",
                source.display(),
                String::from_utf8_lossy(&output.stdout).trim(),
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
    }
    Ok(())
}

fn collect_wave_sources(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), CliError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let ty = entry.file_type()?;
        if ty.is_dir() {
            collect_wave_sources(&path, out)?;
        } else if ty.is_file() && path.extension().is_some_and(|ext| ext == "wave") {
            out.push(path);
        }
    }
    Ok(())
}

fn replace_std_tree(staged: &Path, install_dir: &Path) -> Result<(), CliError> {
    if !install_dir.exists() {
        fs::rename(staged, install_dir)?;
        return Ok(());
    }

    let parent = install_dir.parent().ok_or_else(|| {
        CliError::CommandFailed(format!(
            "std installation path '{}' has no parent",
            install_dir.display()
        ))
    })?;
    let backup = unique_path_in(parent, ".std-backup");
    fs::rename(install_dir, &backup)?;

    if let Err(install_error) = fs::rename(staged, install_dir) {
        return match fs::rename(&backup, install_dir) {
            Ok(()) => Err(CliError::CommandFailed(format!(
                "failed to activate staged std; previous installation was restored: {}",
                install_error
            ))),
            Err(restore_error) => Err(CliError::CommandFailed(format!(
                "failed to activate staged std ({}) and failed to restore '{}' ({})",
                install_error,
                backup.display(),
                restore_error
            ))),
        };
    }

    let _ = fs::remove_dir_all(backup);
    Ok(())
}

fn resolve_std_install_dir() -> Result<PathBuf, CliError> {
    let home = env::var("HOME").map_err(|_| CliError::HomeNotSet)?;
    Ok(PathBuf::from(home).join(".wave/lib/wave/std"))
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), CliError> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_all(&from, &to)?;
        } else if ty.is_file() {
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn tool_exists(name: &str) -> bool {
    Command::new(name).arg("--version").output().is_ok()
}

fn run_cmd(cmd: &mut Command, label: &str) -> Result<(), CliError> {
    let out = cmd.output()?;
    if out.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        Err(CliError::CommandFailed(format!(
            "{} (status={})\nstdout: {}\nstderr: {}",
            label, out.status, stdout, stderr
        )))
    }
}

fn run_cmd_stdout(cmd: &mut Command, label: &str) -> Result<String, CliError> {
    let out = cmd.output()?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        return Err(CliError::CommandFailed(format!(
            "{} (status={})\nstdout: {}\nstderr: {}",
            label, out.status, stdout, stderr
        )));
    }

    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if stdout.is_empty() {
        return Err(CliError::CommandFailed(format!(
            "{} returned empty output",
            label
        )));
    }
    Ok(stdout)
}

fn make_tmp_dir(prefix: &str) -> Result<PathBuf, CliError> {
    make_tmp_dir_in(&env::temp_dir(), prefix)
}

fn unique_path_in(parent: &Path, prefix: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    parent.join(format!("{}-{}-{}", prefix, std::process::id(), timestamp))
}

fn make_tmp_dir_in(parent: &Path, prefix: &str) -> Result<PathBuf, CliError> {
    let path = unique_path_in(parent, prefix);
    fs::create_dir_all(&path)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successful_replacement_exposes_only_the_staged_tree() {
        let root = make_tmp_dir("wave-std-replace-success").unwrap();
        let installed = root.join("std");
        let staged = root.join("staged");
        fs::create_dir_all(&installed).unwrap();
        fs::create_dir_all(&staged).unwrap();
        fs::write(installed.join("old.wave"), "old").unwrap();
        fs::write(staged.join("new.wave"), "new").unwrap();

        replace_std_tree(&staged, &installed).unwrap();

        assert!(!installed.join("old.wave").exists());
        assert_eq!(
            fs::read_to_string(installed.join("new.wave")).unwrap(),
            "new"
        );
        assert!(!staged.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn failed_activation_restores_the_previous_installation() {
        let root = make_tmp_dir("wave-std-replace-rollback").unwrap();
        let installed = root.join("std");
        let missing_stage = root.join("missing-stage");
        fs::create_dir_all(&installed).unwrap();
        fs::write(installed.join("old.wave"), "old").unwrap();

        let error = replace_std_tree(&missing_stage, &installed).unwrap_err();

        assert!(error
            .message()
            .contains("previous installation was restored"));
        assert_eq!(
            fs::read_to_string(installed.join("old.wave")).unwrap(),
            "old"
        );
        let _ = fs::remove_dir_all(root);
    }
}
