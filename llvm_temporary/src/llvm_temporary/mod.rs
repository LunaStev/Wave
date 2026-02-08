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

#![allow(
    dead_code,
    unused_variables,
    clippy::module_inception,
    clippy::type_complexity,
    clippy::unnecessary_map_or,
    clippy::question_mark,
    clippy::result_large_err,
    clippy::ptr_arg,
    clippy::while_let_on_iterator,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::manual_strip,
    clippy::explicit_auto_deref,
    clippy::collapsible_if,
    clippy::collapsible_else_if,
    clippy::new_without_default
)]

pub mod expression;
pub mod llvm_backend;
pub mod llvm_codegen;
mod statement;
mod importgen;
