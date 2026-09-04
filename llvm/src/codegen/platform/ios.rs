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

//! Apple iOS SDK target-planning contract.

use super::{LinkerFamily, PlatformPlan};

pub const CANDIDATE_TRIPLES: &[&str] = &["x86_64-apple-ios", "aarch64-apple-ios"];

pub const PLAN: PlatformPlan = PlatformPlan {
    os: "ios",
    // riscv64 reserves syntax coverage only; no vendor ABI/triple is claimed.
    architectures: &["x86_64", "aarch64", "riscv64"],
    object_format: "macho",
    sdk: "Apple iOS SDK",
    linker: LinkerFamily::DarwinLld,
};
