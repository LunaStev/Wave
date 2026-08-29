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

//! Canonical operating-system names used by target attributes.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatingSystem {
    Linux,
    MacOs,
    Windows,
    FreeBsd,
    None,
}

impl OperatingSystem {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Linux => "linux",
            Self::MacOs => "macos",
            Self::Windows => "windows",
            Self::FreeBsd => "freebsd",
            Self::None => "none",
        }
    }

    pub fn from_name(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "linux" => Some(Self::Linux),
            "macos" | "darwin" | "apple" => Some(Self::MacOs),
            "windows" | "win32" | "win64" => Some(Self::Windows),
            "freebsd" => Some(Self::FreeBsd),
            "none" | "freestanding" => Some(Self::None),
            _ => None,
        }
    }
}

pub fn canonical_name(value: &str) -> String {
    OperatingSystem::from_name(value)
        .map(|os| os.name().to_string())
        .unwrap_or_else(|| value.trim().to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operating_system_aliases_have_stable_canonical_names() {
        for (input, expected) in [
            ("Linux", "linux"),
            ("darwin", "macos"),
            ("unknown-os", "unknown-os"),
            ("win64", "windows"),
            ("FreeBSD", "freebsd"),
            ("freestanding", "none"),
        ] {
            assert_eq!(canonical_name(input), expected);
        }
    }
}
