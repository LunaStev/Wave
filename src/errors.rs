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

//! Errors produced while parsing or executing the `wavec` command line.
//!
//! CLI failures are separate from source diagnostics because they have no Wave
//! source span. Stable `kind` and exit-code mappings are shared by human and JSON
//! output.

use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub enum CliError {
    Usage(String),

    // std
    StdAlreadyInstalled { path: PathBuf },
    ExternalToolMissing(String),
    CommandFailed(String),
    HomeNotSet,

    // io
    Io(std::io::Error),
}

impl CliError {
    pub fn usage(msg: impl Into<String>) -> Self {
        CliError::Usage(msg.into())
    }

    pub fn kind(&self) -> &'static str {
        match self {
            CliError::Usage(_) => "usage",
            CliError::StdAlreadyInstalled { .. } => "std-already-installed",
            CliError::ExternalToolMissing(_) => "external-tool-missing",
            CliError::CommandFailed(_) => "command-failed",
            CliError::HomeNotSet => "home-not-set",
            CliError::Io(_) => "io",
        }
    }

    pub fn message(&self) -> String {
        match self {
            CliError::Usage(msg) => msg.clone(),
            CliError::StdAlreadyInstalled { path } => {
                format!("std already installed at '{}'", path.display())
            }
            CliError::ExternalToolMissing(t) => format!("required tool not found: {}", t),
            CliError::CommandFailed(msg) => format!("command failed: {}", msg),
            CliError::HomeNotSet => "HOME environment variable not set".to_string(),
            CliError::Io(e) => e.to_string(),
        }
    }

    pub fn to_json(&self) -> String {
        format!(
            "{{\"error\":{{\"kind\":{},\"message\":{},\"exit_code\":{}}}}}",
            json_string(self.kind()),
            json_string(&self.message()),
            self.exit_code()
        )
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::Usage(_) => 2,
            CliError::ExternalToolMissing(_) | CliError::HomeNotSet | CliError::Io(_) => 3,
            CliError::StdAlreadyInstalled { .. } | CliError::CommandFailed(_) => 1,
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CliError::Usage(msg) => write!(f, "Error: {}", msg),
            CliError::StdAlreadyInstalled { path } => {
                write!(f, "Error: std already installed at '{}'", path.display())
            }
            CliError::ExternalToolMissing(t) => write!(f, "Error: required tool not found: {}", t),
            CliError::CommandFailed(msg) => write!(f, "Error: command failed: {}", msg),
            CliError::HomeNotSet => write!(f, "Error: HOME environment variable not set"),
            CliError::Io(e) => write!(f, "IO Error: {}", e),
        }
    }
}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        CliError::Io(e)
    }
}

fn json_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
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
