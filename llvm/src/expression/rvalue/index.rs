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

use super::ExprGenEnv;
use inkwell::types::BasicType;
use inkwell::values::{BasicValue, BasicValueEnum};
use parser::ast::Expression;
use crate::codegen::generate_address_and_type_ir;

pub(crate) fn gen<'ctx, 'a>(
    env: &mut ExprGenEnv<'ctx, 'a>,
    target: &Expression,
    index: &Expression,
) -> BasicValueEnum<'ctx> {
    let full = Expression::IndexAccess {
        target: Box::new(target.clone()),
        index: Box::new(index.clone()),
    };

    let (addr, elem_ty) = generate_address_and_type_ir(
        env.context,
        env.builder,
        &full,
        env.variables,
        env.module,
        env.struct_types,
        env.struct_field_indices,
    );

    if elem_ty.is_array_type() || elem_ty.is_struct_type() {
        return addr.as_basic_value_enum();
    }

    env.builder
        .build_load(elem_ty, addr, "load_index_elem")
        .unwrap()
        .as_basic_value_enum()
}
