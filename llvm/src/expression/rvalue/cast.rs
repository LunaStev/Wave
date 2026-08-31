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

//! Explicit `as` cast lowering.
//!
//! Casts use the explicit coercion policy, which permits conversions that
//! implicit assignment rejects. Integer literals cast to pointers receive a
//! pointer-width source hint before conversion.

use super::ExprGenEnv;
use crate::codegen::types::{wave_type_to_llvm_type, TypeFlavor};
use crate::statement::variable::{coerce_basic_value, wave_type_is_unsigned, CoercionMode};
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValue, BasicValueEnum};
use parser::ast::{Expression, Literal, Operator, WaveType};

#[derive(Clone, Copy, PartialEq, Eq)]
enum NumericLiteralKind {
    Integer,
    Float,
}

fn numeric_literal_kind(expr: &Expression) -> Option<NumericLiteralKind> {
    match expr {
        Expression::Literal(Literal::Int(_)) => Some(NumericLiteralKind::Integer),
        Expression::Literal(Literal::Float(_)) => Some(NumericLiteralKind::Float),
        Expression::Grouped(inner) => numeric_literal_kind(inner),
        Expression::Unary {
            operator: Operator::Neg,
            expr: inner,
        } => numeric_literal_kind(inner),
        _ => None,
    }
}

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

    // Integer literals default to i32 without context. A pointer cast needs a
    // width capable of carrying the supported 64-bit target addresses.
    let literal_kind = numeric_literal_kind(expr);
    let src_hint = match (literal_kind, dst_ty) {
        (Some(NumericLiteralKind::Integer), BasicTypeEnum::PointerType(_)) => {
            Some(env.context.i64_type().as_basic_type_enum())
        }
        (Some(NumericLiteralKind::Integer), BasicTypeEnum::IntType(target)) => {
            Some(target.as_basic_type_enum())
        }
        (Some(_), BasicTypeEnum::FloatType(target)) => Some(target.as_basic_type_enum()),
        _ => None,
    };

    let src = env.gen(expr, src_hint);

    // LLVM integers are signless, so numeric casts must select their signed or
    // unsigned instruction from Wave's semantic source and destination types.
    match (src, dst_ty) {
        (BasicValueEnum::IntValue(src_int), BasicTypeEnum::FloatType(dst_float)) => {
            let source_unsigned = matches!(
                env.wave_type(expr),
                Some(WaveType::Uint(_) | WaveType::Bool | WaveType::Byte | WaveType::Char)
            );
            return if source_unsigned {
                env.builder
                    .build_unsigned_int_to_float(src_int, dst_float, "as_cast")
                    .unwrap()
                    .as_basic_value_enum()
            } else {
                env.builder
                    .build_signed_int_to_float(src_int, dst_float, "as_cast")
                    .unwrap()
                    .as_basic_value_enum()
            };
        }
        (BasicValueEnum::FloatValue(src_float), BasicTypeEnum::IntType(dst_int)) => {
            let target_unsigned = matches!(
                target_type,
                WaveType::Uint(_) | WaveType::Bool | WaveType::Byte | WaveType::Char
            );
            return if target_unsigned {
                env.builder
                    .build_float_to_unsigned_int(src_float, dst_int, "as_cast")
                    .unwrap()
                    .as_basic_value_enum()
            } else {
                env.builder
                    .build_float_to_signed_int(src_float, dst_int, "as_cast")
                    .unwrap()
                    .as_basic_value_enum()
            };
        }
        _ => {}
    }

    // LLVM integers do not carry signedness. Preserve Wave's source type when
    // widening an explicit integer cast so u8/u16/u32 values never acquire
    // sign bits merely because their high bit is set.
    if let (BasicValueEnum::IntValue(src_int), BasicTypeEnum::IntType(dst_int)) = (src, dst_ty) {
        let src_bits = src_int.get_type().get_bit_width();
        let dst_bits = dst_int.get_bit_width();
        if src_bits < dst_bits {
            let unsigned = matches!(
                env.wave_type(expr),
                Some(WaveType::Uint(_) | WaveType::Bool | WaveType::Byte | WaveType::Char)
            );
            return if unsigned {
                env.builder
                    .build_int_z_extend(src_int, dst_int, "as_cast")
                    .unwrap()
                    .as_basic_value_enum()
            } else {
                env.builder
                    .build_int_s_extend(src_int, dst_int, "as_cast")
                    .unwrap()
                    .as_basic_value_enum()
            };
        }
    }

    coerce_basic_value(
        env.context,
        env.builder,
        src,
        dst_ty,
        "as_cast",
        CoercionMode::Explicit,
        wave_type_is_unsigned(env.wave_type(expr).as_ref()),
    )
}
