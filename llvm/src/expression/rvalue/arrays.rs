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
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValue, BasicValueEnum};
use parser::ast::Expression;

use crate::statement::variable::{coerce_basic_value, CoercionMode};

pub(crate) fn gen_array_literal<'ctx, 'a>(
    env: &mut ExprGenEnv<'ctx, 'a>,
    elements: &[Expression],
    expected_type: Option<BasicTypeEnum<'ctx>>,
) -> BasicValueEnum<'ctx> {
    let arr_ty = match expected_type {
        Some(BasicTypeEnum::ArrayType(a)) => a,

        Some(BasicTypeEnum::PointerType(_)) => {
            panic!(
                "ArrayLiteral cannot use pointer expected_type on opaque-pointer LLVM. \
Use a temp variable: `var tmp: array<T,N> = [...]; foo(tmp);`"
            );
        }

        Some(t) => panic!("ArrayLiteral expected array type, got {:?}", t),
        None => panic!("ArrayLiteral requires expected_type (array type)"),
    };

    let elem_ty = arr_ty.get_element_type();

    let alloca = env.builder.build_alloca(arr_ty, "arr_lit").unwrap();
    let zero = env.context.i32_type().const_zero();

    for (i, e) in elements.iter().enumerate() {
        let mut v = env.gen(e, Some(elem_ty));

        if v.get_type() != elem_ty {
            v = coerce_basic_value(
                env.context,
                env.builder,
                v,
                elem_ty,
                &format!("arr{}_cast", i),
                CoercionMode::Implicit,
            );
        }

        let idx = env.context.i32_type().const_int(i as u64, false);

        let gep = unsafe {
            env.builder
                .build_in_bounds_gep(arr_ty, alloca, &[zero, idx], &format!("arr_gep_{}", i))
                .unwrap()
        };

        env.builder.build_store(gep, v).unwrap();
    }

    env.builder
        .build_load(arr_ty, alloca, "arr_lit_load")
        .unwrap()
        .as_basic_value_enum()
}
