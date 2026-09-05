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

//! Target-specific validation performed before invoking the native linker.
//!
//! Metadata inspection is kept separate from linker command construction so the
//! checks can move to Whale without coupling them to the current external linker.

mod elf;
mod loongarch;
mod riscv;

pub use elf::LinkInputInspectionError;
pub use loongarch::{
    validate_loongarch64_link_inputs, LoongArchAbiValidationError, LoongArchFloatAbi,
};
pub use riscv::{validate_riscv_link_inputs, AbiValidationError, RiscvFloatAbi};
