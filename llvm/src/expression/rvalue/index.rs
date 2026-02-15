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
use inkwell::values::{BasicValue, BasicValueEnum};
use parser::ast::{Expression, WaveType};

use crate::codegen::types::{wave_type_to_llvm_type, TypeFlavor};

fn idx_to_i32<'ctx>(
    env: &mut ExprGenEnv<'ctx, '_>,
    idx: inkwell::values::IntValue<'ctx>,
) -> inkwell::values::IntValue<'ctx> {
    let i32t = env.context.i32_type();
    let w = idx.get_type().get_bit_width();
    if w == 32 {
        idx
    } else if w < 32 {
        env.builder.build_int_s_extend(idx, i32t, "idx_sext").unwrap()
    } else {
        env.builder.build_int_truncate(idx, i32t, "idx_trunc").unwrap()
    }
}

fn wave_type_of_expr<'ctx, 'a>(env: &ExprGenEnv<'ctx, 'a>, e: &Expression) -> Option<WaveType> {
    match e {
        Expression::Variable(name) => env.variables.get(name).map(|vi| vi.ty.clone()),
        Expression::Grouped(inner) => wave_type_of_expr(env, inner),
        Expression::AddressOf(inner) => wave_type_of_expr(env, inner).map(|t| WaveType::Pointer(Box::new(t))),
        Expression::Deref(inner) => {
            match &**inner {
                Expression::Variable(name) => {
                    let vi = env.variables.get(name)?;
                    match &vi.ty {
                        WaveType::Pointer(inner_ty) => Some((**inner_ty).clone()),
                        WaveType::String => Some(WaveType::Byte),
                        _ => None,
                    }
                }
                _ => None,
            }
        }
        _ => None,
    }
}

pub(crate) fn gen<'ctx, 'a>(
    env: &mut ExprGenEnv<'ctx, 'a>,
    target: &Expression,
    index: &Expression,
) -> BasicValueEnum<'ctx> {
    unsafe {
        let target_val = env.gen(target, None);
        let index_val = env.gen(index, None);

        let index_int = match index_val {
            BasicValueEnum::IntValue(i) => idx_to_i32(env, i),
            _ => panic!("Index must be an integer"),
        };

        let zero = env.context.i32_type().const_zero();

        match target_val {
            BasicValueEnum::PointerValue(ptr_val) => {
                let target_ty = wave_type_of_expr(env, target).unwrap_or_else(|| {
                    panic!(
                        "IndexAccess needs static type info under opaque pointers (LLVM15+). \
                         Cannot infer WaveType for target expr: {:?}",
                        target
                    )
                });

                match target_ty {
                    WaveType::Array(inner, size) => {
                        let arr_bt = wave_type_to_llvm_type(
                            env.context,
                            &WaveType::Array(inner.clone(), size),
                            env.struct_types,
                            TypeFlavor::Value,
                        );

                        let elem_ty = *inner;
                        let elem_bt = wave_type_to_llvm_type(
                            env.context,
                            &elem_ty,
                            env.struct_types,
                            TypeFlavor::Value,
                        );

                        let gep = env
                            .builder
                            .build_in_bounds_gep(arr_bt, ptr_val, &[zero, index_int], "array_index_gep")
                            .unwrap();

                        if matches!(elem_ty, WaveType::Array(_, _) | WaveType::Struct(_)) {
                            return gep.as_basic_value_enum();
                        }

                        return env
                            .builder
                            .build_load(elem_bt, gep, "load_array_elem")
                            .unwrap()
                            .as_basic_value_enum();
                    }

                    WaveType::Pointer(inner) => match *inner {
                        // case 1) ptr -> [array]
                        WaveType::Array(elem, size) => {
                            let arr_bt = wave_type_to_llvm_type(
                                env.context,
                                &WaveType::Array(elem.clone(), size),
                                env.struct_types,
                                TypeFlavor::Value,
                            );

                            let elem_ty = *elem;
                            let elem_bt = wave_type_to_llvm_type(
                                env.context,
                                &elem_ty,
                                env.struct_types,
                                TypeFlavor::Value,
                            );

                            let gep = env
                                .builder
                                .build_in_bounds_gep(arr_bt, ptr_val, &[zero, index_int], "array_index_gep_ptr")
                                .unwrap();

                            if matches!(elem_ty, WaveType::Array(_, _) | WaveType::Struct(_)) {
                                return gep.as_basic_value_enum();
                            }

                            return env
                                .builder
                                .build_load(elem_bt, gep, "load_array_elem_ptr")
                                .unwrap()
                                .as_basic_value_enum();
                        }

                        // case 2) ptr -> T
                        elem_ty => {
                            let elem_bt = wave_type_to_llvm_type(
                                env.context,
                                &elem_ty,
                                env.struct_types,
                                TypeFlavor::Value,
                            );

                            let gep = env
                                .builder
                                .build_in_bounds_gep(elem_bt, ptr_val, &[index_int], "ptr_index_gep")
                                .unwrap();

                            if matches!(elem_ty, WaveType::Array(_, _) | WaveType::Struct(_)) {
                                return gep.as_basic_value_enum();
                            }

                            return env
                                .builder
                                .build_load(elem_bt, gep, "load_ptr_elem")
                                .unwrap()
                                .as_basic_value_enum();
                        }
                    },

                    WaveType::String => {
                        let i8t = env.context.i8_type();
                        let gep = env
                            .builder
                            .build_in_bounds_gep(i8t, ptr_val, &[index_int], "str_index_gep")
                            .unwrap();

                        return env
                            .builder
                            .build_load(i8t, gep, "load_str_elem")
                            .unwrap()
                            .as_basic_value_enum();
                    }

                    other => {
                        panic!("Unsupported target type in IndexAccess: {:?}", other);
                    }
                }
            }

            BasicValueEnum::ArrayValue(arr_val) => {
                // arr_val: [N x T]
                let tmp = env
                    .builder
                    .build_alloca(arr_val.get_type(), "tmp_arr")
                    .unwrap();

                env.builder.build_store(tmp, arr_val).unwrap();

                let elem_bt = arr_val.get_type().get_element_type(); // ✅ ArrayType element type is OK

                let gep = env
                    .builder
                    .build_in_bounds_gep(arr_val.get_type(), tmp, &[zero, index_int], "array_index_gep_tmp")
                    .unwrap();

                if elem_bt.is_array_type() || elem_bt.is_struct_type() {
                    return gep.as_basic_value_enum();
                }

                env.builder
                    .build_load(elem_bt, gep, "load_array_elem_tmp")
                    .unwrap()
                    .as_basic_value_enum()
            }

            other => {
                panic!("Unsupported target in IndexAccess: {:?}", other)
            }
        }
    }
}
