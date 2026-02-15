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
use crate::codegen::generate_address_ir;
use crate::codegen::types::{wave_type_to_llvm_type, TypeFlavor};
use crate::statement::variable::{coerce_basic_value, CoercionMode};
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValue, BasicValueEnum};
use parser::ast::{Expression, WaveType};

fn push_deref_into_base(expr: &Expression) -> Expression {
    match expr {
        Expression::Grouped(inner) => Expression::Grouped(Box::new(push_deref_into_base(inner))),
        Expression::IndexAccess { target, index } => Expression::IndexAccess {
            target: Box::new(push_deref_into_base(target)),
            index: index.clone(),
        },
        Expression::FieldAccess { object, field } => Expression::FieldAccess {
            object: Box::new(push_deref_into_base(object)),
            field: field.clone(),
        },
        other => Expression::Deref(Box::new(other.clone())),
    }
}

fn infer_deref_load_ty<'ctx, 'a>(
    env: &ExprGenEnv<'ctx, 'a>,
    inner_expr: &Expression,
    expected_type: Option<BasicTypeEnum<'ctx>>,
) -> BasicTypeEnum<'ctx> {
    if let Some(t) = expected_type {
        return t;
    }
    match inner_expr {
        Expression::Grouped(inner) => infer_deref_load_ty(env, inner, None),

        Expression::Variable(name) => {
            let vi = env
                .variables
                .get(name)
                .unwrap_or_else(|| panic!("deref: variable '{}' not found", name));

            match &vi.ty {
                WaveType::Pointer(inner) => {
                    wave_type_to_llvm_type(env.context, inner, env.struct_types, TypeFlavor::Value)
                }
                WaveType::String => env.context.i8_type().as_basic_type_enum(), // *string -> byte
                other => panic!("deref: '{}' is not a pointer type: {:?}", name, other),
            }
        }

        _ => panic!(
            "deref needs expected_type in opaque-pointer mode (cannot infer load type from {:?})",
            inner_expr
        ),
    }
}

pub(crate) fn gen_deref<'ctx, 'a>(
    env: &mut ExprGenEnv<'ctx, 'a>,
    inner_expr: &Expression,
    expected_type: Option<BasicTypeEnum<'ctx>>,
) -> BasicValueEnum<'ctx> {
    let load_ty = infer_deref_load_ty(env, inner_expr, expected_type);

    match inner_expr {
        Expression::Grouped(inner) => return gen_deref(env, inner, Some(load_ty)),

        // lvalue (x[i], x.field) -> address -> typed load
        Expression::IndexAccess { .. } | Expression::FieldAccess { .. } => {
            let addr = generate_address_ir(
                env.context,
                env.builder,
                inner_expr,
                env.variables,
                env.module,
                env.struct_types,
                env.struct_field_indices,
            );

            return env
                .builder
                .build_load(load_ty, addr, "deref_load")
                .unwrap()
                .as_basic_value_enum();
        }

        _ => {}
    }

    // pointer value -> typed load
    let v = env.gen(inner_expr, None);
    if let BasicValueEnum::PointerValue(p) = v {
        return env
            .builder
            .build_load(load_ty, p, "deref_load")
            .unwrap()
            .as_basic_value_enum();
    }

    panic!(
        "deref expects pointer or lvalue (x[i], x.field), got: {:?}",
        inner_expr
    );
}

pub(crate) fn gen_addressof<'ctx, 'a>(
    env: &mut ExprGenEnv<'ctx, 'a>,
    inner_expr: &Expression,
    expected_type: Option<BasicTypeEnum<'ctx>>,
) -> BasicValueEnum<'ctx> {
    // &[ ... ] : array literal address-of
    if let Expression::ArrayLiteral(elements) = inner_expr {
        let ptr_ty = match expected_type {
            Some(BasicTypeEnum::PointerType(p)) => p,
            _ => panic!("&[ ... ] needs an expected pointer type (e.g. ptr<i32>)"),
        };

        if elements.is_empty() {
            panic!("&[] cannot infer element type in opaque-pointer mode (empty array literal)");
        }

        let first_val0 = env.gen(&elements[0], None);
        let elem_ty = first_val0.get_type();

        let array_ty = elem_ty.array_type(elements.len() as u32);
        let arr_alloca = env.builder.build_alloca(array_ty, "tmp_array").unwrap();

        let zero = env.context.i32_type().const_zero();

        for (i, expr) in elements.iter().enumerate() {
            let mut val = if i == 0 {
                first_val0
            } else {
                env.gen(expr, Some(elem_ty))
            };

            if val.get_type() != elem_ty {
                val = coerce_basic_value(
                    env.context,
                    env.builder,
                    val,
                    elem_ty,
                    &format!("addrof_arr{}_cast", i),
                    CoercionMode::Implicit,
                );
            }

            let idx = env.context.i32_type().const_int(i as u64, false);
            let gep = unsafe {
                env.builder
                    .build_in_bounds_gep(
                        array_ty,
                        arr_alloca,
                        &[zero, idx],
                        &format!("array_idx_{}", i),
                    )
                    .unwrap()
            };

            env.builder.build_store(gep, val).unwrap();
        }

        // return pointer to first element (array decays)
        let first = unsafe {
            env.builder
                .build_in_bounds_gep(array_ty, arr_alloca, &[zero, zero], "array_first_ptr")
                .unwrap()
        };

        if first.get_type() != ptr_ty {
            return env
                .builder
                .build_bit_cast(
                    first.as_basic_value_enum(),
                    ptr_ty.as_basic_type_enum(),
                    "addrof_array_cast",
                )
                .unwrap()
                .as_basic_value_enum();
        }

        return first.as_basic_value_enum();
    }

    // normal &lvalue : address
    let addr = generate_address_ir(
        env.context,
        env.builder,
        inner_expr,
        env.variables,
        env.module,
        env.struct_types,
        env.struct_field_indices,
    );

    if let Some(BasicTypeEnum::PointerType(ptr_ty)) = expected_type {
        if addr.get_type() != ptr_ty {
            return env
                .builder
                .build_bit_cast(
                    addr.as_basic_value_enum(),
                    ptr_ty.as_basic_type_enum(),
                    "addrof_cast",
                )
                .unwrap()
                .as_basic_value_enum();
        }
    }

    addr.as_basic_value_enum()
}
