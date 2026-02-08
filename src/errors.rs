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

use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub enum CliError {
    Usage(String),

    // std
    StdAlreadyInstalled { path: PathBuf },
    ExternalToolMissing(&'static str),
    CommandFailed(String),
    HomeNotSet,

    // io
    Io(std::io::Error),
}

impl CliError {
    pub fn usage(msg: impl Into<String>) -> Self {
        CliError::Usage(msg.into())
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