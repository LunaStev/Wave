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

//! Address calculation for assignable expressions.
//!
//! LLVM pointers are opaque, so lvalue lowering must recover pointee and field
//! types from Wave semantic types rather than from the LLVM pointer itself.
//! This module returns both the address and its storage type to keep subsequent
//! loads and stores consistent. Index evaluation does not add bounds checks:
//! in-bounds GEP still requires the resulting address to stay within its source
//! allocation (negative pointer offsets may address earlier elements).

use crate::expression::rvalue::ExprGenEnv;
use hir::TypedProgram;
use inkwell::context::Context;
use inkwell::types::{AsTypeRef, BasicType, BasicTypeEnum, StructType};
use inkwell::values::{IntValue, PointerValue};
use parser::ast::{Expression, WaveType};

use std::collections::HashMap;

use crate::codegen::types::TypeFlavor;
use crate::codegen::wave_type_to_llvm_type;

use super::types::VariableInfo;

fn normalize_struct_name(raw: &str) -> &str {
    raw.strip_prefix("struct.").unwrap_or(raw).trim_start_matches('%')
}

/// Evaluate once in the expression's own integer type, then adapt the offset
/// to the target address width. In particular, u8/u32 offsets must not become
/// negative when LLVM sign-extends a narrow GEP index.
pub(crate) fn generate_index_ir<'ctx>(
    env: &mut ExprGenEnv<'ctx, '_>,
    expr: &Expression,
) -> IntValue<'ctx> {
    let index_ty = env.context.ptr_sized_int_type(env.target_data, None);
    let expected = match env.program.type_of(expr) {
        Some(hir::HirExpressionType::IntegerLiteral) => Some(index_ty.into()),
        _ => None,
    };
    let value = env.gen(expr, expected).into_int_value();
    let source_unsigned = matches!(
        env.wave_type(expr),
        Some(WaveType::Uint(_) | WaveType::Byte | WaveType::Char | WaveType::Bool)
    );
    match value.get_type().get_bit_width().cmp(&index_ty.get_bit_width()) {
        std::cmp::Ordering::Equal => value,
        std::cmp::Ordering::Less if source_unsigned => {
            env.builder.build_int_z_extend(value, index_ty, "idx_zext").unwrap()
        },
        std::cmp::Ordering::Less => {
            env.builder.build_int_s_extend(value, index_ty, "idx_sext").unwrap()
        },
        std::cmp::Ordering::Greater => {
            env.builder.build_int_truncate(value, index_ty, "idx_trunc").unwrap()
        },
    }
}

fn resolve_struct_key<'ctx>(
    st: StructType<'ctx>,
    struct_types: &HashMap<String, StructType<'ctx>>,
) -> String {
    if let Some(raw) = st.get_name().and_then(|n| n.to_str().ok()) {
        return normalize_struct_name(raw).to_string();
    }

    let st_ref = st.as_type_ref();
    for (name, ty) in struct_types {
        if ty.as_type_ref() == st_ref {
            return name.clone();
        }
    }

    panic!("LLVM struct type has no name and cannot be matched to struct_types");
}

fn storage_ty_of_var<'ctx>(
    context: &'ctx Context,
    vi: &VariableInfo<'ctx>,
    struct_types: &HashMap<String, StructType<'ctx>>,
) -> BasicTypeEnum<'ctx> {
    wave_type_to_llvm_type(context, &vi.ty, struct_types, TypeFlavor::AbiC)
}

fn pointee_ty_of_ptr_expr<'ctx>(
    context: &'ctx Context,
    expr: &Expression,
    program: &TypedProgram,
    variables: &HashMap<String, VariableInfo<'ctx>>,
    struct_types: &HashMap<String, StructType<'ctx>>,
) -> BasicTypeEnum<'ctx> {
    if let Some(hir::HirExpressionType::Resolved(WaveType::Pointer(inner))) = program.type_of(expr)
    {
        return wave_type_to_llvm_type(context, inner, struct_types, TypeFlavor::AbiC);
    }
    match expr {
        Expression::Grouped(inner) => {
            pointee_ty_of_ptr_expr(context, inner, program, variables, struct_types)
        },

        Expression::Variable(name) => {
            let vi = variables.get(name).unwrap_or_else(|| panic!("Variable {} not found", name));

            match &vi.ty {
                WaveType::Pointer(inner) => {
                    wave_type_to_llvm_type(context, inner, struct_types, TypeFlavor::AbiC)
                },
                WaveType::String => context.i8_type().as_basic_type_enum(),
                other => panic!("deref/index expects pointer type, got {:?} for {}", other, name),
            }
        },

        // ptr coming from field/index: LLVM pointer is opaque -> pointee unknown
        _ => context.i8_type().as_basic_type_enum(),
    }
}

/// internal: returns (address, value_type_at_address)
fn addr_and_ty<'ctx>(
    env: &mut ExprGenEnv<'ctx, '_>,
    expr: &Expression,
) -> (PointerValue<'ctx>, BasicTypeEnum<'ctx>) {
    match expr {
        Expression::Cast { expr: inner, target_type: WaveType::Pointer(_) }
        | Expression::Grouped(inner) => addr_and_ty(env, inner),

        Expression::Variable(name) => {
            let vi =
                env.variables.get(name).unwrap_or_else(|| panic!("Variable {} not found", name));
            (vi.ptr, storage_ty_of_var(env.context, vi, env.struct_types))
        },

        // legacy behavior: treat &x as "address of x" when someone asks for address again
        Expression::AddressOf(inner) => addr_and_ty(env, inner),

        // A dereference consumes a pointer value, which may be returned by a
        // call or computed expression rather than stored in a variable slot.
        Expression::Deref(inner) => {
            if matches!(
                inner.as_ref(),
                Expression::IndexAccess { .. } | Expression::FieldAccess { .. }
            ) {
                return addr_and_ty(env, inner);
            }
            let pointer = env.gen(inner, None).into_pointer_value();
            let pointee = pointee_ty_of_ptr_expr(
                env.context,
                inner,
                env.program,
                env.variables,
                env.struct_types,
            );
            (pointer, pointee)
        },

        Expression::FieldAccess { object, field } => {
            let (struct_ptr, struct_ty) = match env.wave_type(object) {
                Some(WaveType::Pointer(inner)) => {
                    let WaveType::Struct(name) = inner.as_ref() else {
                        panic!("field receiver does not point to a struct: {inner:?}");
                    };
                    let pointer = env.gen(object, None).into_pointer_value();
                    (pointer, env.struct_types[name])
                },
                Some(WaveType::Struct(_)) => {
                    let (address, ty) = addr_and_ty(env, object);
                    (address, ty.into_struct_type())
                },
                other => panic!("field receiver lacks a resolved struct type: {other:?}"),
            };

            let sname = resolve_struct_key(struct_ty, env.struct_types);

            let idx = *env
                .struct_field_indices
                .get(&sname)
                .unwrap_or_else(|| panic!("Struct '{}' missing in struct_field_indices", sname))
                .get(field)
                .unwrap_or_else(|| {
                    panic!("Field '{}.{}' missing in struct_field_indices", sname, field)
                });

            let field_ty = struct_ty
                .get_field_type_at_index(idx)
                .unwrap_or_else(|| panic!("No field type at index {} for struct '{}'", idx, sname));

            let field_ptr =
                env.builder.build_struct_gep(struct_ty, struct_ptr, idx, "field_ptr").unwrap();

            (field_ptr, field_ty)
        },

        Expression::IndexAccess { target, index } => {
            // A pointer base is a value, even when produced by a call or a
            // field. Read it before the index can mutate its original slot.
            // Array lvalues keep their storage; aggregate rvalues get a
            // temporary through addr_and_ty without reevaluating the base.
            let (base, ty) = match env.wave_type(target) {
                Some(WaveType::Array(_, _)) => addr_and_ty(env, target),
                Some(WaveType::Pointer(inner)) => {
                    let pointer = env.gen(target, None).into_pointer_value();
                    let pointee = wave_type_to_llvm_type(
                        env.context,
                        &inner,
                        env.struct_types,
                        TypeFlavor::AbiC,
                    );
                    (pointer, pointee)
                },
                Some(WaveType::String) => {
                    (env.gen(target, None).into_pointer_value(), env.context.i8_type().into())
                },
                other => panic!("index base lacks a resolved array or pointer type: {other:?}"),
            };
            let idx = generate_index_ir(env, index);
            match ty {
                BasicTypeEnum::ArrayType(at) => {
                    let zero = idx.get_type().const_zero();
                    let ep = unsafe {
                        env.builder.build_in_bounds_gep(at, base, &[zero, idx], "arr_gep").unwrap()
                    };
                    (ep, at.get_element_type())
                },
                pointee => {
                    let ep = unsafe {
                        env.builder.build_in_bounds_gep(pointee, base, &[idx], "ptr_gep").unwrap()
                    };
                    (ep, pointee)
                },
            }
        },

        other => {
            let ty = env.wave_type(other).expect("aggregate temporary has a resolved HIR type");
            if !matches!(ty, WaveType::Array(_, _) | WaveType::Struct(_)) {
                panic!("Cannot take address of this expression: {other:?}");
            }
            let ty = wave_type_to_llvm_type(env.context, &ty, env.struct_types, TypeFlavor::AbiC);
            let value = env.gen(other, Some(ty));
            assert_eq!(value.get_type(), ty, "aggregate temporary must contain its HIR value");
            let storage = crate::statement::variable::build_entry_alloca(
                env.context,
                env.builder,
                ty,
                "aggregate_index_base",
            );
            env.builder.build_store(storage, value).unwrap();
            (storage, ty)
        },
    }
}

pub(crate) fn generate_address_ir<'ctx>(
    env: &mut ExprGenEnv<'ctx, '_>,
    expr: &Expression,
) -> PointerValue<'ctx> {
    addr_and_ty(env, expr).0
}

pub(crate) fn generate_address_and_type_ir<'ctx>(
    env: &mut ExprGenEnv<'ctx, '_>,
    expr: &Expression,
) -> (PointerValue<'ctx>, BasicTypeEnum<'ctx>) {
    addr_and_ty(env, expr)
}
