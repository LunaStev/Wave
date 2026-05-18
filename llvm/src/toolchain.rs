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

use std::env;
use std::path::PathBuf;

pub fn find_bundled_linux_crt1(target: &str) -> Option<PathBuf> {
    bundled_linux_crt1_candidates(target)
        .into_iter()
        .find(|path| path.is_file())
}

pub fn expected_bundled_linux_crt1(target: &str) -> PathBuf {
    bundled_linux_crt1_candidates(target)
        .into_iter()
        .next()
        .unwrap_or_else(|| PathBuf::from(format!("crt/{}/crt1.o", target)))
}

fn bundled_linux_crt1_candidates(target: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(path) = env::var("WAVE_LINUX_CRT1_OBJECT") {
        if !path.trim().is_empty() {
            paths.push(PathBuf::from(path));
        }
    }

    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join("crt").join(target).join("crt1.o"));
            if let Some(root) = dir.parent() {
                paths.push(
                    root.join("lib")
                        .join("wave")
                        .join("crt")
                        .join(target)
                        .join("crt1.o"),
                );
            }
        }
    }

    paths
}
