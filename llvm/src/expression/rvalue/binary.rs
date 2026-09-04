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

//! Binary-expression lowering, including numeric inference and pointer offsets.
//!
//! Unsuffixed numeric literals borrow a concrete LLVM type from the surrounding
//! expectation or the opposite operand. Pointer arithmetic is scaled by the
//! inferred pointee type and retains Wave's unchecked, C-like memory contract.

use super::{utils::to_bool, ExprGenEnv};
use crate::codegen::types::{wave_type_to_llvm_type, TypeFlavor};
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValue, BasicValueEnum, IntValue, PointerValue};
use inkwell::{FloatPredicate, IntPredicate};
use parser::ast::{Expression, Literal, Operator, WaveType};

fn is_numeric_literal(expr: &Expression) -> bool {
    match expr {
        Expression::Literal(Literal::Int(_)) | Expression::Literal(Literal::Float(_)) => true,
        Expression::Grouped(inner) => is_numeric_literal(inner),
        _ => false,
    }
}

fn is_integer_literal(expr: &Expression) -> bool {
    match expr {
        Expression::Literal(Literal::Int(_)) => true,
        Expression::Grouped(inner) => is_integer_literal(inner),
        _ => false,
    }
}

fn value_numeric_basic_type<'ctx>(v: BasicValueEnum<'ctx>) -> Option<BasicTypeEnum<'ctx>> {
    match v {
        BasicValueEnum::IntValue(iv) => Some(iv.get_type().as_basic_type_enum()),
        BasicValueEnum::FloatValue(fv) => Some(fv.get_type().as_basic_type_enum()),
        _ => None,
    }
}

fn is_unsigned_integer_type(ty: Option<WaveType>) -> bool {
    matches!(
        ty,
        Some(WaveType::Uint(_) | WaveType::Bool | WaveType::Byte | WaveType::Char)
    )
}

fn integer_width(ty: &WaveType) -> Option<u16> {
    match ty {
        WaveType::Int(bits) | WaveType::Uint(bits) => Some(*bits),
        WaveType::Bool => Some(1),
        WaveType::Byte | WaveType::Char => Some(8),
        _ => None,
    }
}

fn promoted_integer_type<'ctx, 'a>(
    env: &ExprGenEnv<'ctx, 'a>,
    left: &Expression,
    right: &Expression,
) -> Option<WaveType> {
    let mut left_type = env.wave_type(left)?;
    let mut right_type = env.wave_type(right)?;

    // Integer literals borrow the concrete type of the opposite operand. For
    // two concrete integer types, mirror the frontend's wider-type selection;
    // equal-width mixed signedness therefore follows the left operand until
    // Wave defines a different mixed-signed promotion contract.
    if is_integer_literal(left) && !is_integer_literal(right) {
        left_type = right_type.clone();
    }
    if is_integer_literal(right) && !is_integer_literal(left) {
        right_type = left_type.clone();
    }

    let left_width = integer_width(&left_type)?;
    let right_width = integer_width(&right_type)?;
    if left_width >= right_width {
        Some(left_type)
    } else {
        Some(right_type)
    }
}

fn integer_operation_is_unsigned<'ctx, 'a>(
    env: &ExprGenEnv<'ctx, 'a>,
    left: &Expression,
    right: &Expression,
) -> bool {
    is_unsigned_integer_type(promoted_integer_type(env, left, right))
}

fn build_int_to_float<'ctx, 'a>(
    env: &ExprGenEnv<'ctx, 'a>,
    value: IntValue<'ctx>,
    expression: &Expression,
    float_type: inkwell::types::FloatType<'ctx>,
    tag: &str,
) -> inkwell::values::FloatValue<'ctx> {
    if is_unsigned_integer_type(env.wave_type(expression)) {
        env.builder
            .build_unsigned_int_to_float(value, float_type, tag)
            .unwrap()
    } else {
        env.builder
            .build_signed_int_to_float(value, float_type, tag)
            .unwrap()
    }
}

fn cast_int_to_i64<'ctx, 'a>(
    env: &ExprGenEnv<'ctx, 'a>,
    v: IntValue<'ctx>,
    expression: &Expression,
    tag: &str,
) -> IntValue<'ctx> {
    let i64_ty = env.context.i64_type();
    let src_bits = v.get_type().get_bit_width();

    if src_bits == 64 {
        v
    } else if src_bits < 64 {
        if is_unsigned_integer_type(env.wave_type(expression)) {
            env.builder
                .build_int_z_extend(v, i64_ty, &format!("{}_zext", tag))
                .unwrap()
        } else {
            env.builder
                .build_int_s_extend(v, i64_ty, &format!("{}_sext", tag))
                .unwrap()
        }
    } else {
        env.builder
            .build_int_truncate(v, i64_ty, &format!("{}_trunc", tag))
            .unwrap()
    }
}

fn infer_ptr_pointee_ty<'ctx, 'a>(
    env: &ExprGenEnv<'ctx, 'a>,
    expr: &Expression,
) -> BasicTypeEnum<'ctx> {
    match expr {
        Expression::Grouped(inner) => infer_ptr_pointee_ty(env, inner),

        Expression::Variable(name) => {
            if let Some(vi) = env.variables.get(name) {
                match &vi.ty {
                    WaveType::Pointer(inner) => wave_type_to_llvm_type(
                        env.context,
                        inner,
                        env.struct_types,
                        TypeFlavor::AbiC,
                    ),
                    WaveType::String => env.context.i8_type().as_basic_type_enum(),
                    _ => env.context.i8_type().as_basic_type_enum(),
                }
            } else {
                env.context.i8_type().as_basic_type_enum()
            }
        }

        Expression::AddressOf(inner) => {
            if let Expression::Variable(name) = &**inner {
                if let Some(vi) = env.variables.get(name) {
                    return wave_type_to_llvm_type(
                        env.context,
                        &vi.ty,
                        env.struct_types,
                        TypeFlavor::AbiC,
                    );
                }
            }
            env.context.i8_type().as_basic_type_enum()
        }

        Expression::Cast { target_type, .. } => match target_type {
            WaveType::Pointer(inner) => {
                wave_type_to_llvm_type(env.context, inner, env.struct_types, TypeFlavor::AbiC)
            }
            WaveType::String => env.context.i8_type().as_basic_type_enum(),
            _ => env.context.i8_type().as_basic_type_enum(),
        },

        _ => env.context.i8_type().as_basic_type_enum(),
    }
}

fn gep_with_i64_offset<'ctx, 'a>(
    env: &ExprGenEnv<'ctx, 'a>,
    ptr: PointerValue<'ctx>,
    ptr_expr: &Expression,
    idx_i64: IntValue<'ctx>,
    tag: &str,
) -> PointerValue<'ctx> {
    let pointee_ty = infer_ptr_pointee_ty(env, ptr_expr);
    // SAFETY: Wave pointer arithmetic is explicitly unchecked. A source program
    // must keep an inbounds result within the original allocation (or one past
    // it), which is the contract required by LLVM's `inbounds` GEP.
    unsafe {
        env.builder
            .build_in_bounds_gep(pointee_ty, ptr, &[idx_i64], tag)
            .unwrap()
    }
}

pub(crate) fn gen<'ctx, 'a>(
    env: &mut ExprGenEnv<'ctx, 'a>,
    left: &Expression,
    operator: &Operator,
    right: &Expression,
    expected_type: Option<inkwell::types::BasicTypeEnum<'ctx>>,
) -> BasicValueEnum<'ctx> {
    if matches!(operator, Operator::LogicalAnd | Operator::LogicalOr) {
        let left_value = env.gen(left, None).into_int_value();
        let left_bool = to_bool(env.builder, left_value);
        let left_block = env.builder.get_insert_block().unwrap();
        let function = left_block.get_parent().unwrap();
        let right_block = env.context.append_basic_block(function, "logical.rhs");
        let merge_block = env.context.append_basic_block(function, "logical.end");

        if matches!(operator, Operator::LogicalAnd) {
            env.builder
                .build_conditional_branch(left_bool, right_block, merge_block)
                .unwrap();
        } else {
            env.builder
                .build_conditional_branch(left_bool, merge_block, right_block)
                .unwrap();
        }

        env.builder.position_at_end(right_block);
        let right_value = env.gen(right, None).into_int_value();
        let right_bool = to_bool(env.builder, right_value);
        let right_end = env.builder.get_insert_block().unwrap();
        env.builder.build_unconditional_branch(merge_block).unwrap();

        env.builder.position_at_end(merge_block);
        let short_value = env.context.bool_type().const_int(
            if matches!(operator, Operator::LogicalOr) {
                1
            } else {
                0
            },
            false,
        );
        let phi = env
            .builder
            .build_phi(env.context.bool_type(), "logical.result")
            .unwrap();
        phi.add_incoming(&[(&short_value, left_block), (&right_bool, right_end)]);
        let mut result = phi.as_basic_value().into_int_value();

        if let Some(BasicTypeEnum::IntType(expected)) = expected_type {
            if result.get_type() != expected {
                result = env
                    .builder
                    .build_int_z_extend(result, expected, "logical.cast")
                    .unwrap();
            }
        }

        return result.as_basic_value_enum();
    }

    // A comparison's expected type describes its boolean result, not either
    // operand. Feeding that i8 result type into numeric literals truncates
    // values before comparison (for example, `i64_value == -36` became a
    // comparison with 220). Infer comparison operands from each other instead.
    let comparison_result = matches!(
        operator,
        Operator::Greater
            | Operator::Less
            | Operator::Equal
            | Operator::NotEqual
            | Operator::GreaterEqual
            | Operator::LessEqual
            | Operator::LogicalAnd
            | Operator::LogicalOr
    );
    let numeric_expected = if comparison_result {
        None
    } else {
        match expected_type {
            Some(BasicTypeEnum::IntType(_)) | Some(BasicTypeEnum::FloatType(_)) => expected_type,
            _ => None,
        }
    };

    let (left_val, right_val) = if let Some(exp) = numeric_expected {
        (env.gen(left, Some(exp)), env.gen(right, Some(exp)))
    } else if is_numeric_literal(left) && is_numeric_literal(right) {
        (env.gen(left, None), env.gen(right, None))
    } else if is_numeric_literal(left) {
        let r = env.gen(right, None);
        let l = if let Some(hint) = value_numeric_basic_type(r) {
            env.gen(left, Some(hint))
        } else {
            env.gen(left, None)
        };
        (l, r)
    } else {
        let l = env.gen(left, None);
        let r = if is_numeric_literal(right) {
            if let Some(hint) = value_numeric_basic_type(l) {
                env.gen(right, Some(hint))
            } else {
                env.gen(right, None)
            }
        } else {
            env.gen(right, None)
        };
        (l, r)
    };

    match (left_val, right_val) {
        (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
            let l_type = l.get_type();
            let r_type = r.get_type();
            let left_unsigned = is_unsigned_integer_type(env.wave_type(left));
            let right_unsigned = is_unsigned_integer_type(env.wave_type(right));
            let operation_unsigned = integer_operation_is_unsigned(env, left, right);

            let (l_casted, r_casted) = match operator {
                Operator::ShiftLeft | Operator::ShiftRight => {
                    let r2 = if r_type != l_type {
                        env.builder.build_int_cast(r, l_type, "shamt").unwrap()
                    } else {
                        r
                    };
                    (l, r2)
                }
                _ => {
                    if l_type != r_type {
                        if l_type.get_bit_width() < r_type.get_bit_width() {
                            let new_l = if left_unsigned {
                                env.builder.build_int_z_extend(l, r_type, "zext_l").unwrap()
                            } else {
                                env.builder.build_int_s_extend(l, r_type, "sext_l").unwrap()
                            };
                            (new_l, r)
                        } else {
                            let new_r = if right_unsigned {
                                env.builder.build_int_z_extend(r, l_type, "zext_r").unwrap()
                            } else {
                                env.builder.build_int_s_extend(r, l_type, "sext_r").unwrap()
                            };
                            (l, new_r)
                        }
                    } else {
                        (l, r)
                    }
                }
            };

            let mut result = match operator {
                Operator::Add => env.builder.build_int_add(l_casted, r_casted, "addtmp"),
                Operator::Subtract => env.builder.build_int_sub(l_casted, r_casted, "subtmp"),
                Operator::Multiply => env.builder.build_int_mul(l_casted, r_casted, "multmp"),
                Operator::Divide if operation_unsigned => env
                    .builder
                    .build_int_unsigned_div(l_casted, r_casted, "divtmp"),
                Operator::Divide => env
                    .builder
                    .build_int_signed_div(l_casted, r_casted, "divtmp"),
                Operator::Remainder if operation_unsigned => env
                    .builder
                    .build_int_unsigned_rem(l_casted, r_casted, "modtmp"),
                Operator::Remainder => env
                    .builder
                    .build_int_signed_rem(l_casted, r_casted, "modtmp"),
                Operator::ShiftLeft => env.builder.build_left_shift(l_casted, r_casted, "shl"),
                Operator::ShiftRight => {
                    let arithmetic = !matches!(
                        env.wave_type(left),
                        Some(WaveType::Uint(_) | WaveType::Bool | WaveType::Byte | WaveType::Char)
                    );
                    env.builder
                        .build_right_shift(l_casted, r_casted, arithmetic, "shr")
                }
                Operator::BitwiseAnd => env.builder.build_and(l_casted, r_casted, "andtmp"),
                Operator::BitwiseOr => env.builder.build_or(l_casted, r_casted, "ortmp"),
                Operator::BitwiseXor => env.builder.build_xor(l_casted, r_casted, "xortmp"),

                Operator::Greater => {
                    let predicate = if operation_unsigned {
                        IntPredicate::UGT
                    } else {
                        IntPredicate::SGT
                    };
                    env.builder
                        .build_int_compare(predicate, l_casted, r_casted, "cmptmp")
                }
                Operator::Less => {
                    let predicate = if operation_unsigned {
                        IntPredicate::ULT
                    } else {
                        IntPredicate::SLT
                    };
                    env.builder
                        .build_int_compare(predicate, l_casted, r_casted, "cmptmp")
                }
                Operator::Equal => {
                    env.builder
                        .build_int_compare(IntPredicate::EQ, l_casted, r_casted, "cmptmp")
                }
                Operator::NotEqual => {
                    env.builder
                        .build_int_compare(IntPredicate::NE, l_casted, r_casted, "cmptmp")
                }
                Operator::GreaterEqual => {
                    let predicate = if operation_unsigned {
                        IntPredicate::UGE
                    } else {
                        IntPredicate::SGE
                    };
                    env.builder
                        .build_int_compare(predicate, l_casted, r_casted, "cmptmp")
                }
                Operator::LessEqual => {
                    let predicate = if operation_unsigned {
                        IntPredicate::ULE
                    } else {
                        IntPredicate::SLE
                    };
                    env.builder
                        .build_int_compare(predicate, l_casted, r_casted, "cmptmp")
                }

                Operator::LogicalAnd | Operator::LogicalOr => unreachable!(),

                _ => panic!("Unsupported binary operator"),
            }
            .unwrap();

            if let Some(inkwell::types::BasicTypeEnum::IntType(target_ty)) = expected_type {
                let result_ty = result.get_type();
                if result_ty != target_ty {
                    result = if result_ty.get_bit_width() == 1 {
                        env.builder
                            .build_int_z_extend(result, target_ty, "cast_result")
                            .unwrap()
                    } else {
                        env.builder
                            .build_int_cast(result, target_ty, "cast_result")
                            .unwrap()
                    };
                }
            }

            result.as_basic_value_enum()
        }

        (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
            let mut result: BasicValueEnum<'ctx> = match operator {
                Operator::Add => env
                    .builder
                    .build_float_add(l, r, "faddtmp")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::Subtract => env
                    .builder
                    .build_float_sub(l, r, "fsubtmp")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::Multiply => env
                    .builder
                    .build_float_mul(l, r, "fmultmp")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::Divide => env
                    .builder
                    .build_float_div(l, r, "fdivtmp")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::Remainder => env
                    .builder
                    .build_float_rem(l, r, "fmodtmp")
                    .unwrap()
                    .as_basic_value_enum(),

                Operator::Greater => env
                    .builder
                    .build_float_compare(FloatPredicate::OGT, l, r, "fcmpgt")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::Less => env
                    .builder
                    .build_float_compare(FloatPredicate::OLT, l, r, "fcmplt")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::Equal => env
                    .builder
                    .build_float_compare(FloatPredicate::OEQ, l, r, "fcmpeq")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::NotEqual => env
                    .builder
                    .build_float_compare(FloatPredicate::ONE, l, r, "fcmpne")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::GreaterEqual => env
                    .builder
                    .build_float_compare(FloatPredicate::OGE, l, r, "fcmpge")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::LessEqual => env
                    .builder
                    .build_float_compare(FloatPredicate::OLE, l, r, "fcmple")
                    .unwrap()
                    .as_basic_value_enum(),

                _ => panic!("Unsupported float operator"),
            };

            #[allow(clippy::collapsible_match)]
            if let Some(exp) = expected_type {
                match (result, exp) {
                    (
                        BasicValueEnum::FloatValue(fv),
                        inkwell::types::BasicTypeEnum::FloatType(target_ty),
                    ) => {
                        if fv.get_type() != target_ty {
                            result = env
                                .builder
                                .build_float_cast(fv, target_ty, "fcast_result")
                                .unwrap()
                                .as_basic_value_enum();
                        }
                    }
                    (
                        BasicValueEnum::IntValue(iv),
                        inkwell::types::BasicTypeEnum::IntType(target_ty),
                    ) => {
                        if iv.get_type() != target_ty {
                            result = if iv.get_type().get_bit_width() == 1 {
                                env.builder
                                    .build_int_z_extend(iv, target_ty, "icast_result")
                                    .unwrap()
                                    .as_basic_value_enum()
                            } else {
                                env.builder
                                    .build_int_cast(iv, target_ty, "icast_result")
                                    .unwrap()
                                    .as_basic_value_enum()
                            };
                        }
                    }
                    _ => {}
                }
            }

            result
        }

        (BasicValueEnum::IntValue(int_val), BasicValueEnum::FloatValue(float_val)) => {
            let casted = build_int_to_float(env, int_val, left, float_val.get_type(), "cast_lhs");

            match operator {
                Operator::Add => env
                    .builder
                    .build_float_add(casted, float_val, "addtmp")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::Subtract => env
                    .builder
                    .build_float_sub(casted, float_val, "subtmp")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::Multiply => env
                    .builder
                    .build_float_mul(casted, float_val, "multmp")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::Divide => env
                    .builder
                    .build_float_div(casted, float_val, "divtmp")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::Remainder => env
                    .builder
                    .build_float_rem(casted, float_val, "modtmp")
                    .unwrap()
                    .as_basic_value_enum(),

                Operator::Greater => env
                    .builder
                    .build_float_compare(FloatPredicate::OGT, casted, float_val, "fcmpgt")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::Less => env
                    .builder
                    .build_float_compare(FloatPredicate::OLT, casted, float_val, "fcmplt")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::Equal => env
                    .builder
                    .build_float_compare(FloatPredicate::OEQ, casted, float_val, "fcmpeq")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::NotEqual => env
                    .builder
                    .build_float_compare(FloatPredicate::ONE, casted, float_val, "fcmpne")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::GreaterEqual => env
                    .builder
                    .build_float_compare(FloatPredicate::OGE, casted, float_val, "fcmpge")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::LessEqual => env
                    .builder
                    .build_float_compare(FloatPredicate::OLE, casted, float_val, "fcmple")
                    .unwrap()
                    .as_basic_value_enum(),

                _ => panic!("Unsupported mixed-type operator (int + float)"),
            }
        }

        (BasicValueEnum::FloatValue(float_val), BasicValueEnum::IntValue(int_val)) => {
            let casted = build_int_to_float(env, int_val, right, float_val.get_type(), "cast_rhs");

            match operator {
                Operator::Add => env
                    .builder
                    .build_float_add(float_val, casted, "addtmp")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::Subtract => env
                    .builder
                    .build_float_sub(float_val, casted, "subtmp")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::Multiply => env
                    .builder
                    .build_float_mul(float_val, casted, "multmp")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::Divide => env
                    .builder
                    .build_float_div(float_val, casted, "divtmp")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::Remainder => env
                    .builder
                    .build_float_rem(float_val, casted, "modtmp")
                    .unwrap()
                    .as_basic_value_enum(),

                Operator::Greater => env
                    .builder
                    .build_float_compare(FloatPredicate::OGT, float_val, casted, "fcmpgt")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::Less => env
                    .builder
                    .build_float_compare(FloatPredicate::OLT, float_val, casted, "fcmplt")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::Equal => env
                    .builder
                    .build_float_compare(FloatPredicate::OEQ, float_val, casted, "fcmpeq")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::NotEqual => env
                    .builder
                    .build_float_compare(FloatPredicate::ONE, float_val, casted, "fcmpne")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::GreaterEqual => env
                    .builder
                    .build_float_compare(FloatPredicate::OGE, float_val, casted, "fcmpge")
                    .unwrap()
                    .as_basic_value_enum(),
                Operator::LessEqual => env
                    .builder
                    .build_float_compare(FloatPredicate::OLE, float_val, casted, "fcmple")
                    .unwrap()
                    .as_basic_value_enum(),

                _ => panic!("Unsupported mixed-type operator (float + int)"),
            }
        }
        (BasicValueEnum::PointerValue(lp), BasicValueEnum::PointerValue(rp)) => {
            let i64_ty = env.context.i64_type();
            let li = env
                .builder
                .build_ptr_to_int(lp, i64_ty, "l_ptr2int")
                .unwrap();
            let ri = env
                .builder
                .build_ptr_to_int(rp, i64_ty, "r_ptr2int")
                .unwrap();

            let mut result = match operator {
                Operator::Equal => env
                    .builder
                    .build_int_compare(IntPredicate::EQ, li, ri, "ptreq")
                    .unwrap(),
                Operator::NotEqual => env
                    .builder
                    .build_int_compare(IntPredicate::NE, li, ri, "ptrne")
                    .unwrap(),
                Operator::Subtract => env.builder.build_int_sub(li, ri, "ptrdiff").unwrap(),
                _ => panic!("Unsupported pointer operator: {:?}", operator),
            };

            match operator {
                Operator::Equal | Operator::NotEqual => {
                    if let Some(inkwell::types::BasicTypeEnum::IntType(target_ty)) = expected_type {
                        if result.get_type() != target_ty {
                            if result.get_type().get_bit_width() > target_ty.get_bit_width() {
                                panic!(
                                    "implicit integer narrowing is forbidden in binary result: i{} -> i{}",
                                    result.get_type().get_bit_width(),
                                    target_ty.get_bit_width()
                                );
                            }
                            result = env
                                .builder
                                .build_int_cast(result, target_ty, "cast_result")
                                .unwrap();
                        }
                    }
                }
                Operator::Subtract => {
                    if let Some(inkwell::types::BasicTypeEnum::IntType(target_ty)) = expected_type {
                        if result.get_type() != target_ty {
                            result = env
                                .builder
                                .build_int_cast(result, target_ty, "cast_result")
                                .unwrap();
                        }
                    }
                }
                _ => {}
            }

            return result.as_basic_value_enum();
        }

        (BasicValueEnum::PointerValue(lp), BasicValueEnum::IntValue(ri)) => {
            match operator {
                Operator::Add | Operator::Subtract => {
                    let mut idx = cast_int_to_i64(env, ri, right, "ptr_idx");
                    if matches!(operator, Operator::Subtract) {
                        idx = env.builder.build_int_neg(idx, "ptr_idx_neg").unwrap();
                    }
                    let p = gep_with_i64_offset(env, lp, left, idx, "ptr_gep");
                    return p.as_basic_value_enum();
                }
                _ => {}
            };

            let i64_ty = env.context.i64_type();
            let li = env
                .builder
                .build_ptr_to_int(lp, i64_ty, "l_ptr2int")
                .unwrap();

            let ri = cast_int_to_i64(env, ri, right, "r_i64");

            let mut result = match operator {
                Operator::Equal => env
                    .builder
                    .build_int_compare(IntPredicate::EQ, li, ri, "ptreq0")
                    .unwrap(),
                Operator::NotEqual => env
                    .builder
                    .build_int_compare(IntPredicate::NE, li, ri, "ptrne0")
                    .unwrap(),
                _ => panic!("Unsupported ptr/int operator: {:?}", operator),
            };

            if let Some(inkwell::types::BasicTypeEnum::IntType(target_ty)) = expected_type {
                if result.get_type() != target_ty {
                    if result.get_type().get_bit_width() > target_ty.get_bit_width() {
                        panic!(
                            "implicit integer narrowing is forbidden in binary result: i{} -> i{}",
                            result.get_type().get_bit_width(),
                            target_ty.get_bit_width()
                        );
                    }
                    result = env
                        .builder
                        .build_int_cast(result, target_ty, "cast_result")
                        .unwrap();
                }
            }

            return result.as_basic_value_enum();
        }

        (BasicValueEnum::IntValue(li), BasicValueEnum::PointerValue(rp)) => {
            if matches!(operator, Operator::Add) {
                let idx = cast_int_to_i64(env, li, left, "ptr_idx");
                let p = gep_with_i64_offset(env, rp, right, idx, "ptr_gep");
                return p.as_basic_value_enum();
            }

            let i64_ty = env.context.i64_type();
            let li = cast_int_to_i64(env, li, left, "l_i64");

            let ri = env
                .builder
                .build_ptr_to_int(rp, i64_ty, "r_ptr2int")
                .unwrap();

            let mut result = match operator {
                Operator::Equal => env
                    .builder
                    .build_int_compare(IntPredicate::EQ, li, ri, "ptreq0")
                    .unwrap(),
                Operator::NotEqual => env
                    .builder
                    .build_int_compare(IntPredicate::NE, li, ri, "ptrne0")
                    .unwrap(),
                _ => panic!("Unsupported int/ptr operator: {:?}", operator),
            };

            if let Some(inkwell::types::BasicTypeEnum::IntType(target_ty)) = expected_type {
                if result.get_type() != target_ty {
                    if result.get_type().get_bit_width() > target_ty.get_bit_width() {
                        panic!(
                            "implicit integer narrowing is forbidden in binary result: i{} -> i{}",
                            result.get_type().get_bit_width(),
                            target_ty.get_bit_width()
                        );
                    }
                    result = env
                        .builder
                        .build_int_cast(result, target_ty, "cast_result")
                        .unwrap();
                }
            }

            return result.as_basic_value_enum();
        }

        _ => panic!("Type mismatch in binary expression"),
    }
}
