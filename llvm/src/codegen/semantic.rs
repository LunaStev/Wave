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

//! Thread-local bridge from frontend expression types to backend lowering.
//!
//! Keys are addresses of expressions in the exact AST allocation analyzed before
//! codegen. The table must be installed after the last AST clone or rewrite and
//! consumed on the same thread; a future typed AST should replace this bridge.

use parser::ast::{Expression, WaveType};
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static EXPRESSION_TYPES: RefCell<HashMap<usize, WaveType>> = RefCell::new(HashMap::new());
}

pub(super) fn install_expression_types(types: HashMap<usize, WaveType>) {
    // Replace rather than extend so one compilation cannot observe entries from
    // an earlier module built on the same worker thread.
    EXPRESSION_TYPES.with(|current| *current.borrow_mut() = types);
}

pub(crate) fn expression_type(expression: &Expression) -> Option<WaveType> {
    let key = expression as *const Expression as usize;
    EXPRESSION_TYPES.with(|types| types.borrow().get(&key).cloned())
}
