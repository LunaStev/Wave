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
use std::{env, fs};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn std_install() -> Result<(), CliError> {
    install_or_update_std(false)
}

pub fn std_update() -> Result<(), CliError> {
    install_or_update_std(true)
}

fn install_or_update_std(is_update: bool) -> Result<(), CliError> {
    let install_dir = resolve_std_install_dir()?;

    if install_dir.exists() {
        if !is_update {
            return Err(CliError::StdAlreadyInstalled { path: install_dir });
        }
        fs::remove_dir_all(&install_dir)?;
    }

    fs::create_dir_all(&install_dir)?;
    install_std_from_wave_repo_sparse(&install_dir)?;

    if is_update {
        println!("✅ std updated: {}", install_dir.display());
    } else {
        println!("✅ std installed: {}", install_dir.display());
    }

    Ok(())
}

fn install_std_from_wave_repo_sparse(stage_dir: &Path) -> Result<(), CliError> {
    if !tool_exists("git") {
        return Err(CliError::ExternalToolMissing("git"));
    }

    let repo = "https://github.com/wavefnd/Wave.git";
    let reference = "master";

    let tmp = make_tmp_dir("wave-std")?;

    run_cmd(
        Command::new("git")
            .arg("clone")
            .arg("--depth").arg("1")
            .arg("--filter=blob:none")
            .arg("--sparse")
            .arg("--branch").arg(reference)
            .arg(repo)
            .arg(&tmp),
        "git clone",
    )?;

    run_cmd(
        Command::new("git")
            .arg("-C").arg(&tmp)
            .arg("sparse-checkout")
            .arg("set")
            .arg("std"),
        "git sparse-checkout set std",
    )?;

    let src_std = tmp.join("std");

    let manifest_path = src_std.join("manifest.json");
    if !manifest_path.exists() {
        return Err(CliError::CommandFailed(
            "manifest.json not found in repo/std (add std/manifest.json)".to_string(),
        ));
    }

    let text = fs::read_to_string(&manifest_path)?;
    let manifest = utils::json::parse(&text)
        .map_err(|e| CliError::CommandFailed(format!("invalid manifest.json: {}", e)))?;

    if manifest.get_str("name") != Some("std") {
        return Err(CliError::CommandFailed("manifest.json name != 'std'".to_string()));
    }

    copy_dir_all(&src_std, stage_dir)?;

    fs::write(
        stage_dir.join("INSTALL_META"),
        format!("repo={}\nref={}\n", repo, reference),
    )?;

    let _ = fs::remove_dir_all(&tmp);
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

fn make_tmp_dir(prefix: &str) -> Result<PathBuf, CliError> {
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let p = env::temp_dir().join(format!("{}-{}", prefix, t));
    fs::create_dir_all(&p)?;
    Ok(p)
}