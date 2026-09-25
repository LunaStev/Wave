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

//! Lower already-decided Wave numeric semantics. No promotion policy lives here.
use super::types::{wave_type_to_llvm_type, TypeFlavor};
use inkwell::{
    builder::Builder,
    context::Context,
    types::StructType,
    values::{BasicValue, BasicValueEnum},
    FloatPredicate, IntPredicate,
};
use parser::{
    ast::{Operator, WaveType},
    hir::conversions::{unsigned, ConversionInfo, ConversionKind},
};
use std::collections::HashMap;

pub(crate) fn apply<'ctx>(
    context: &'ctx Context,
    builder: &Builder<'ctx>,
    structs: &HashMap<String, StructType<'ctx>>,
    value: BasicValueEnum<'ctx>,
    info: &ConversionInfo,
) -> BasicValueEnum<'ctx> {
    use ConversionKind::*;
    let source = wave_type_to_llvm_type(context, &info.source_type, structs, TypeFlavor::Value);
    let target = wave_type_to_llvm_type(context, &info.target_type, structs, TypeFlavor::Value);
    assert_eq!(
        value.get_type(),
        source,
        "ICE: conversion source differs from verified HIR"
    );
    match info.kind {
        Identity | ReinterpretInteger => {
            assert_eq!(source, target);
            value
        }
        SignExtend => builder
            .build_int_s_extend(
                value.into_int_value(),
                target.into_int_type(),
                "convert.sext",
            )
            .unwrap()
            .into(),
        ZeroExtend => builder
            .build_int_z_extend(
                value.into_int_value(),
                target.into_int_type(),
                "convert.zext",
            )
            .unwrap()
            .into(),
        Truncate => builder
            .build_int_truncate(
                value.into_int_value(),
                target.into_int_type(),
                "convert.trunc",
            )
            .unwrap()
            .into(),
        SignedToFloat => builder
            .build_signed_int_to_float(
                value.into_int_value(),
                target.into_float_type(),
                "convert.sitofp",
            )
            .unwrap()
            .into(),
        UnsignedToFloat => builder
            .build_unsigned_int_to_float(
                value.into_int_value(),
                target.into_float_type(),
                "convert.uitofp",
            )
            .unwrap()
            .into(),
        FloatToSigned => builder
            .build_float_to_signed_int(
                value.into_float_value(),
                target.into_int_type(),
                "convert.fptosi",
            )
            .unwrap()
            .into(),
        FloatToUnsigned => builder
            .build_float_to_unsigned_int(
                value.into_float_value(),
                target.into_int_type(),
                "convert.fptoui",
            )
            .unwrap()
            .into(),
        FloatExtend | FloatTruncate => builder
            .build_float_cast(
                value.into_float_value(),
                target.into_float_type(),
                "convert.float",
            )
            .unwrap()
            .into(),
        PointerToInteger => builder
            .build_ptr_to_int(
                value.into_pointer_value(),
                target.into_int_type(),
                "convert.ptrint",
            )
            .unwrap()
            .into(),
        IntegerToPointer => builder
            .build_int_to_ptr(
                value.into_int_value(),
                target.into_pointer_type(),
                "convert.intptr",
            )
            .unwrap()
            .into(),
        PointerCast => builder
            .build_pointer_cast(
                value.into_pointer_value(),
                target.into_pointer_type(),
                "convert.ptr",
            )
            .unwrap()
            .into(),
    }
}

pub(crate) fn binary<'ctx>(
    builder: &Builder<'ctx>,
    left: BasicValueEnum<'ctx>,
    operator: &Operator,
    right: BasicValueEnum<'ctx>,
    computation: &WaveType,
) -> BasicValueEnum<'ctx> {
    assert_eq!(
        left.get_type(),
        right.get_type(),
        "ICE: HIR operands must have the computation type"
    );
    let operation_unsigned = unsigned(computation);
    match (left, right) {
        (BasicValueEnum::IntValue(l_casted), BasicValueEnum::IntValue(r_casted)) => {
            let result = match operator {
                Operator::Add => builder.build_int_add(l_casted, r_casted, "addtmp"),
                Operator::Subtract => builder.build_int_sub(l_casted, r_casted, "subtmp"),
                Operator::Multiply => builder.build_int_mul(l_casted, r_casted, "multmp"),
                Operator::Divide if operation_unsigned => {
                    builder.build_int_unsigned_div(l_casted, r_casted, "divtmp")
                }
                Operator::Divide => builder.build_int_signed_div(l_casted, r_casted, "divtmp"),
                Operator::Remainder if operation_unsigned => {
                    builder.build_int_unsigned_rem(l_casted, r_casted, "modtmp")
                }
                Operator::Remainder => builder.build_int_signed_rem(l_casted, r_casted, "modtmp"),
                Operator::ShiftLeft => builder.build_left_shift(l_casted, r_casted, "shl"),
                Operator::ShiftRight => {
                    let arithmetic = !operation_unsigned;
                    builder.build_right_shift(l_casted, r_casted, arithmetic, "shr")
                }
                Operator::BitwiseAnd => builder.build_and(l_casted, r_casted, "andtmp"),
                Operator::BitwiseOr => builder.build_or(l_casted, r_casted, "ortmp"),
                Operator::BitwiseXor => builder.build_xor(l_casted, r_casted, "xortmp"),

                Operator::Greater => {
                    let predicate = if operation_unsigned {
                        IntPredicate::UGT
                    } else {
                        IntPredicate::SGT
                    };
                    builder.build_int_compare(predicate, l_casted, r_casted, "cmptmp")
                }
                Operator::Less => {
                    let predicate = if operation_unsigned {
                        IntPredicate::ULT
                    } else {
                        IntPredicate::SLT
                    };
                    builder.build_int_compare(predicate, l_casted, r_casted, "cmptmp")
                }
                Operator::Equal => {
                    builder.build_int_compare(IntPredicate::EQ, l_casted, r_casted, "cmptmp")
                }
                Operator::NotEqual => {
                    builder.build_int_compare(IntPredicate::NE, l_casted, r_casted, "cmptmp")
                }
                Operator::GreaterEqual => {
                    let predicate = if operation_unsigned {
                        IntPredicate::UGE
                    } else {
                        IntPredicate::SGE
                    };
                    builder.build_int_compare(predicate, l_casted, r_casted, "cmptmp")
                }
                Operator::LessEqual => {
                    let predicate = if operation_unsigned {
                        IntPredicate::ULE
                    } else {
                        IntPredicate::SLE
                    };
                    builder.build_int_compare(predicate, l_casted, r_casted, "cmptmp")
                }

                Operator::LogicalAnd | Operator::LogicalOr => unreachable!(),

                _ => panic!("Unsupported binary operator"),
            }
            .unwrap();

            result.into()
        }
        (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
            let result: BasicValueEnum<'ctx> = match operator {
                Operator::Add => builder
                    .build_float_add(l, r, "faddtmp")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::Subtract => builder
                    .build_float_sub(l, r, "fsubtmp")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::Multiply => builder
                    .build_float_mul(l, r, "fmultmp")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::Divide => builder
                    .build_float_div(l, r, "fdivtmp")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::Remainder => builder
                    .build_float_rem(l, r, "fmodtmp")
                    .unwrap()
                    .as_basic_value_enum(),

                Operator::Greater => builder
                    .build_float_compare(FloatPredicate::OGT, l, r, "fcmpgt")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::Less => builder
                    .build_float_compare(FloatPredicate::OLT, l, r, "fcmplt")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::Equal => builder
                    .build_float_compare(FloatPredicate::OEQ, l, r, "fcmpeq")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::NotEqual => builder
                    .build_float_compare(FloatPredicate::UNE, l, r, "fcmpne")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::GreaterEqual => builder
                    .build_float_compare(FloatPredicate::OGE, l, r, "fcmpge")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::LessEqual => builder
                    .build_float_compare(FloatPredicate::OLE, l, r, "fcmple")
                    .unwrap()
                    .as_basic_value_enum(),

                _ => panic!("Unsupported float operator"),
            };

            result
        }
        _ => panic!("ICE: nonnumeric computation"),
    }
}
