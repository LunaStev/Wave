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
use crate::codegen::plan::*;
use crate::codegen::target::{require_supported_target_from_module, CodegenTarget};
use crate::codegen::types::{wave_type_to_llvm_type, TypeFlavor};
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, StringRadix};
use inkwell::values::{
    AsValueRef, BasicMetadataValueEnum, BasicValue, BasicValueEnum, PointerValue, ValueKind,
};
use inkwell::InlineAsmDialect;
use parser::ast::{Expression, Literal, WaveType};

fn inline_asm_dialect_for_target(target: CodegenTarget) -> InlineAsmDialect {
    match target {
        CodegenTarget::LinuxX86_64 => InlineAsmDialect::Intel,
        CodegenTarget::DarwinArm64 => InlineAsmDialect::ATT,
    }
}

pub(crate) fn gen<'ctx, 'a>(
    env: &mut ExprGenEnv<'ctx, 'a>,
    instructions: &[String],
    inputs: &[(String, Expression)],
    outputs: &[(String, Expression)],
    clobbers: &[String],
) -> BasicValueEnum<'ctx> {
    let target = require_supported_target_from_module(env.module);
    let plan = AsmPlan::build(
        target,
        instructions,
        inputs,
        outputs,
        clobbers,
        AsmSafetyMode::ConservativeKernel,
    );
    let constraints_str = plan.constraints_string();

    let mut operand_vals: Vec<BasicMetadataValueEnum<'ctx>> = Vec::with_capacity(plan.inputs.len());
    for inp in &plan.inputs {
        let v = eval_asm_in_expr(env, inp.value);
        operand_vals.push(v.into());
    }

    let param_types: Vec<BasicMetadataTypeEnum<'ctx>> =
        operand_vals.iter().map(meta_val_type).collect();

    // void asm
    if plan.outputs.is_empty() {
        let fn_type = env.context.void_type().fn_type(&param_types, false);

        let inline_asm = env.context.create_inline_asm(
            fn_type,
            plan.asm_code.clone(),
            constraints_str,
            plan.has_side_effects,
            false,
            Some(inline_asm_dialect_for_target(target)),
            false,
        );

        let callee = unsafe { PointerValue::new(inline_asm.as_value_ref()) };

        env.builder
            .build_indirect_call(fn_type, callee, &operand_vals, "inline_asm_void")
            .unwrap();

        return env
            .context
            .i64_type()
            .const_int(0, false)
            .as_basic_value_enum();
    }

    // asm expr must have exactly 1 output
    if plan.outputs.len() != 1 {
        panic!(
            "asm expression requires exactly 1 output (got {})",
            plan.outputs.len()
        );
    }

    let out_ty = resolve_expr_out_type(env, plan.outputs[0].target);
    let fn_type = out_ty.fn_type(&param_types, false);

    let inline_asm = env.context.create_inline_asm(
        fn_type,
        plan.asm_code.clone(),
        constraints_str,
        plan.has_side_effects,
        false,
        Some(inline_asm_dialect_for_target(target)),
        false,
    );

    let callee = unsafe { PointerValue::new(inline_asm.as_value_ref()) };

    let call = env
        .builder
        .build_indirect_call(fn_type, callee, &operand_vals, "inline_asm_expr")
        .unwrap();

    match call.try_as_basic_value() {
        ValueKind::Basic(v) => v,
        ValueKind::Instruction(_) => {
            panic!("inline asm expr expected to return a value, but got instruction-only result");
        }
    }
}

fn llvm_type_of_wave<'ctx, 'a>(env: &ExprGenEnv<'ctx, 'a>, wt: &WaveType) -> BasicTypeEnum<'ctx> {
    wave_type_to_llvm_type(env.context, wt, env.struct_types, TypeFlavor::Value)
}

fn resolve_expr_out_type<'ctx, 'a>(
    env: &ExprGenEnv<'ctx, 'a>,
    target: &Expression,
) -> BasicTypeEnum<'ctx> {
    match target {
        Expression::Variable(name) => {
            let info = env
                .variables
                .get(name)
                .unwrap_or_else(|| panic!("Output var '{}' not found", name));
            llvm_type_of_wave(env, &info.ty)
        }

        Expression::Deref(inner) => match inner.as_ref() {
            Expression::Variable(name) => {
                let info = env
                    .variables
                    .get(name)
                    .unwrap_or_else(|| panic!("Pointer var '{}' not found", name));

                match &info.ty {
                    WaveType::Pointer(inner_ty) => llvm_type_of_wave(env, inner_ty),
                    WaveType::String => env.context.i8_type().as_basic_type_enum(),
                    other => panic!(
                        "asm expr out(*{}) requires pointer/string, got {:?}",
                        name, other
                    ),
                }
            }
            other => panic!("Unsupported expr deref output: {:?}", other),
        },

        other => panic!(
            "asm expr out(...) target must be variable or deref(var) for now: {:?}",
            other
        ),
    }
}

fn eval_asm_in_expr<'ctx, 'a>(
    env: &mut ExprGenEnv<'ctx, 'a>,
    e: &Expression,
) -> BasicValueEnum<'ctx> {
    match e {
        Expression::Literal(Literal::Int(n)) => {
            let s = n.as_str();
            let (neg, digits) = if let Some(rest) = s.strip_prefix('-') {
                (true, rest)
            } else {
                (false, s)
            };

            let ty = env.context.i64_type();
            let mut iv = ty
                .const_int_from_string(digits, StringRadix::Decimal)
                .unwrap_or_else(|| panic!("invalid int literal: {}", s));

            if neg {
                iv = iv.const_neg();
            }
            iv.as_basic_value_enum()
        }

        Expression::Variable(name) => {
            let info = env
                .variables
                .get(name)
                .unwrap_or_else(|| panic!("Input variable '{}' not found", name));

            let ty = llvm_type_of_wave(env, &info.ty);
            env.builder
                .build_load(ty, info.ptr, &format!("asm_in_load_{}", name))
                .unwrap()
                .as_basic_value_enum()
        }

        Expression::AddressOf(inner) => match inner.as_ref() {
            Expression::Variable(name) => {
                let info = env
                    .variables
                    .get(name)
                    .unwrap_or_else(|| panic!("Input variable '{}' not found", name));
                info.ptr.as_basic_value_enum()
            }
            _ => panic!("Unsupported address-of input: {:?}", inner),
        },

        Expression::Deref(inner) => match inner.as_ref() {
            Expression::Variable(name) => {
                let info = env
                    .variables
                    .get(name)
                    .unwrap_or_else(|| panic!("Input pointer '{}' not found", name));

                // 1) load pointer value from the variable slot (typed load)
                let ptr_ty = match &info.ty {
                    WaveType::Pointer(_) | WaveType::String => llvm_type_of_wave(env, &info.ty),
                    other => panic!("deref input '{}' is not a pointer type: {:?}", name, other),
                };

                let pv_val = env
                    .builder
                    .build_load(ptr_ty, info.ptr, "asm_in_ptr")
                    .unwrap();

                let pv = match pv_val {
                    BasicValueEnum::PointerValue(p) => p,
                    _ => panic!("deref input '{}' loaded value is not a pointer", name),
                };

                // 2) load pointee value (must be typed, opaque pointer safe)
                let pointee_ty = match &info.ty {
                    WaveType::Pointer(inner_ty) => llvm_type_of_wave(env, inner_ty),
                    WaveType::String => env.context.i8_type().as_basic_type_enum(),
                    _ => unreachable!(),
                };

                env.builder
                    .build_load(pointee_ty, pv, "asm_in_deref")
                    .unwrap()
                    .as_basic_value_enum()
            }
            _ => panic!("Unsupported deref input: {:?}", inner),
        },

        other => panic!("Unsupported asm input expr: {:?}", other),
    }
}

fn meta_val_type<'ctx>(v: &BasicMetadataValueEnum<'ctx>) -> BasicMetadataTypeEnum<'ctx> {
    match v {
        BasicMetadataValueEnum::IntValue(iv) => iv.get_type().into(),
        BasicMetadataValueEnum::FloatValue(fv) => fv.get_type().into(),
        BasicMetadataValueEnum::PointerValue(pv) => pv.get_type().into(),
        BasicMetadataValueEnum::StructValue(sv) => sv.get_type().into(),
        BasicMetadataValueEnum::VectorValue(vv) => vv.get_type().into(),
        BasicMetadataValueEnum::ArrayValue(av) => av.get_type().into(),
        BasicMetadataValueEnum::ScalableVectorValue(svv) => svv.get_type().into(),
        BasicMetadataValueEnum::MetadataValue(_) => {
            panic!("MetadataValue cannot be used as an inline asm operand");
        }
    }
}
