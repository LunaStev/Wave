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

pub mod ir;
pub mod consts;
pub mod format;
pub mod types;
pub mod address;
pub mod legacy;
pub mod plan;
pub mod abi_c;

pub use address::{generate_address_and_type_ir, generate_address_ir};
pub use format::{wave_format_to_c, wave_format_to_scanf};
pub use ir::generate_ir;
pub use types::{wave_type_to_llvm_type, VariableInfo};

pub use legacy::{create_alloc, get_llvm_type};
