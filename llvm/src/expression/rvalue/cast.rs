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
use crate::codegen::types::{wave_type_to_llvm_type, TypeFlavor};
use crate::statement::variable::{coerce_basic_value, CoercionMode};
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::BasicValueEnum;
use parser::ast::{Expression, Literal, WaveType};

pub(crate) fn gen<'ctx, 'a>(
    env: &mut ExprGenEnv<'ctx, 'a>,
    expr: &Expression,
    target_type: &WaveType,
) -> BasicValueEnum<'ctx> {
    let dst_ty = wave_type_to_llvm_type(
        env.context,
        target_type,
        env.struct_types,
        TypeFlavor::Value,
    );

    // Integer literals default to i32 with no context.
    // For explicit cast to pointer, prefer i64 source width.
    let src_hint = match (expr, dst_ty) {
        (Expression::Literal(Literal::Int(_)), BasicTypeEnum::PointerType(_)) => {
            Some(env.context.i64_type().as_basic_type_enum())
        }
        _ => None,
    };

    let src = env.gen(expr, src_hint);
    coerce_basic_value(
        env.context,
        env.builder,
        src,
        dst_ty,
        "as_cast",
        CoercionMode::Explicit,
    )
}
