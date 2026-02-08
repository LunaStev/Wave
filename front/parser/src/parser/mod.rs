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

mod parse;
pub mod asm;
pub mod control;
pub mod decl;
pub mod expr;
pub mod functions;
pub mod io;
pub mod items;
pub mod stmt;
pub mod types;

pub use parse::parse;