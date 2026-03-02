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

use crate::codegen::plan::*;
use crate::codegen::target::{require_supported_target_from_module, CodegenTarget};
use crate::codegen::types::{wave_type_to_llvm_type, TypeFlavor};
use crate::codegen::VariableInfo;

use inkwell::module::Module;
use inkwell::values::{
    BasicMetadataValueEnum, BasicValue, BasicValueEnum, PointerValue, ValueKind,
};
use inkwell::InlineAsmDialect;

use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, StringRadix};

use parser::ast::{Expression, Literal, WaveType};
use std::collections::HashMap;

enum AsmOutPlace<'ctx> {
    VarAlloca {
        ptr: PointerValue<'ctx>,
        elem_ty: BasicTypeEnum<'ctx>,
    },
    MemPtr {
        ptr: PointerValue<'ctx>,
        elem_ty: BasicTypeEnum<'ctx>,
    },
}

fn llvm_type_of_wave<'ctx>(
    context: &'ctx inkwell::context::Context,
    wt: &WaveType,
    struct_types: &HashMap<String, inkwell::types::StructType<'ctx>>,
) -> BasicTypeEnum<'ctx> {
    wave_type_to_llvm_type(context, wt, struct_types, TypeFlavor::Value)
}

fn reg_width_bits(reg: &str) -> Option<u32> {
    match reg {
        "al" | "bl" | "cl" | "dl" | "sil" | "dil" | "r8b" | "r9b" | "r10b" | "r11b" | "r12b"
        | "r13b" | "r14b" | "r15b" => Some(8),

        "ax" | "bx" | "cx" | "dx" | "si" | "di" | "r8w" | "r9w" | "r10w" | "r11w" | "r12w"
        | "r13w" | "r14w" | "r15w" => Some(16),

        "eax" | "ebx" | "ecx" | "edx" | "esi" | "edi" | "r8d" | "r9d" | "r10d" | "r11d"
        | "r12d" | "r13d" | "r14d" | "r15d" => Some(32),

        "rax" | "rbx" | "rcx" | "rdx" | "rsi" | "rdi" | "rbp" | "rsp" | "r8" | "r9" | "r10"
        | "r11" | "r12" | "r13" | "r14" | "r15" => Some(64),

        _ => None,
    }
}

fn reg_width_bits_for_target(target: CodegenTarget, reg: &str) -> Option<u32> {
    match target {
        CodegenTarget::LinuxX86_64 => reg_width_bits(reg),
        CodegenTarget::DarwinArm64 => {
            if reg.len() >= 2 {
                let (prefix, num) = reg.split_at(1);
                if num.chars().all(|c| c.is_ascii_digit()) && !num.is_empty() {
                    if let Ok(n) = num.parse::<u32>() {
                        if n <= 30 {
                            return match prefix {
                                "w" => Some(32),
                                "x" => Some(64),
                                _ => None,
                            };
                        }
                    }
                }
            }
            None
        }
    }
}

fn extract_reg_from_constraint(c: &str) -> Option<String> {
    if let Some(inner) = c.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
        return Some(inner.to_ascii_lowercase());
    }

    let token = c.trim().trim_start_matches('%').to_ascii_lowercase();
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

fn inline_asm_dialect_for_target(target: CodegenTarget) -> InlineAsmDialect {
    match target {
        CodegenTarget::LinuxX86_64 => InlineAsmDialect::Intel,
        CodegenTarget::DarwinArm64 => InlineAsmDialect::ATT,
    }
}

pub(super) fn gen_asm_stmt_ir<'ctx>(
    context: &'ctx inkwell::context::Context,
    builder: &'ctx inkwell::builder::Builder<'ctx>,
    module: &'ctx Module<'ctx>,
    instructions: &[String],
    inputs: &[(String, Expression)],
    outputs: &[(String, Expression)],
    clobbers: &[String],
    variables: &mut HashMap<String, VariableInfo<'ctx>>,
    global_consts: &HashMap<String, BasicValueEnum<'ctx>>,
    struct_types: &HashMap<String, inkwell::types::StructType<'ctx>>,
) {
    let target = require_supported_target_from_module(module);
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
    let mut param_types: Vec<BasicMetadataTypeEnum<'ctx>> = Vec::with_capacity(plan.inputs.len());

    for inp in &plan.inputs {
        let mut val = asm_operand_to_value(
            context,
            builder,
            variables,
            global_consts,
            struct_types,
            inp.value,
        );

        // reg width forcing
        if let Some(reg) = extract_reg_from_constraint(&inp.constraint) {
            if let Some(bits) = reg_width_bits_for_target(target, &reg) {
                if val.is_int_value() {
                    let iv = val.into_int_value();
                    let target_ty = context.custom_width_int_type(bits);

                    let src_bits = iv.get_type().get_bit_width();
                    let dst_bits = bits;

                    if src_bits != dst_bits {
                        if src_bits > dst_bits {
                            val = builder
                                .build_int_truncate(iv, target_ty, "asm_in_trunc")
                                .unwrap()
                                .as_basic_value_enum();
                        } else {
                            let signed = infer_signedness(inp.value, variables).unwrap_or(false);
                            val = if signed {
                                builder
                                    .build_int_s_extend(iv, target_ty, "asm_in_sext")
                                    .unwrap()
                                    .as_basic_value_enum()
                            } else {
                                builder
                                    .build_int_z_extend(iv, target_ty, "asm_in_zext")
                                    .unwrap()
                                    .as_basic_value_enum()
                            };
                        }
                    }
                }
            }
        }

        param_types.push(val.get_type().into());
        operand_vals.push(val.into());
    }

    let mut out_places: Vec<AsmOutPlace<'ctx>> = Vec::with_capacity(plan.outputs.len());
    let mut out_tys: Vec<BasicTypeEnum<'ctx>> = Vec::with_capacity(plan.outputs.len());

    for o in &plan.outputs {
        let (place, dst_ty) =
            resolve_out_place_and_type(context, builder, variables, struct_types, o.target);

        let mut asm_ty = dst_ty;
        if let Some(reg) = extract_reg_from_constraint(&o.reg_norm) {
            if let Some(bits) = reg_width_bits_for_target(target, &reg) {
                if dst_ty.is_int_type() {
                    asm_ty = context.custom_width_int_type(bits).as_basic_type_enum();
                }
            }
        }

        out_places.push(place);
        out_tys.push(asm_ty);
    }

    let fn_type = if out_tys.is_empty() {
        context.void_type().fn_type(&param_types, false)
    } else if out_tys.len() == 1 {
        out_tys[0].fn_type(&param_types, false)
    } else {
        let st = context.struct_type(&out_tys, false);
        st.fn_type(&param_types, false)
    };

    let inline_asm = context.create_inline_asm(
        fn_type,
        plan.asm_code.clone(),
        constraints_str,
        plan.has_side_effects,
        false,
        Some(inline_asm_dialect_for_target(target)),
        false,
    );

    let call = builder
        .build_indirect_call(fn_type, inline_asm, &operand_vals, "inline_asm")
        .unwrap();

    if out_places.is_empty() {
        return;
    }

    let ret_val = match call.try_as_basic_value() {
        ValueKind::Basic(v) => v,
        ValueKind::Instruction(_) => {
            panic!("asm stmt expected return value but got instruction-only result");
        }
    };

    if out_places.len() == 1 {
        store_asm_out_place(context, builder, &out_places[0], ret_val, "asm_out");
        return;
    }

    let struct_val = ret_val.into_struct_value();
    for (idx, place) in out_places.iter().enumerate() {
        let elem = builder
            .build_extract_value(struct_val, idx as u32, "asm_out_elem")
            .unwrap();
        store_asm_out_place(context, builder, place, elem, "asm_out");
    }
}

fn infer_signedness<'ctx>(
    expr: &Expression,
    variables: &HashMap<String, VariableInfo<'ctx>>,
) -> Option<bool> {
    match expr {
        Expression::Variable(name) => variables.get(name).map(|v| match &v.ty {
            WaveType::Int(_) => true,
            WaveType::Uint(_) => false,
            _ => true,
        }),
        Expression::Grouped(inner) => infer_signedness(inner, variables),
        Expression::Deref(inner) => {
            if let Expression::Variable(name) = inner.as_ref() {
                variables.get(name).and_then(|v| match &v.ty {
                    WaveType::Pointer(inner_ty) => match inner_ty.as_ref() {
                        WaveType::Int(_) => Some(true),
                        WaveType::Uint(_) => Some(false),
                        _ => None,
                    },
                    _ => None,
                })
            } else {
                None
            }
        }
        Expression::Literal(Literal::Int(s)) => {
            if s.trim_start().starts_with('-') {
                Some(true)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn resolve_out_place_and_type<'ctx>(
    context: &'ctx inkwell::context::Context,
    builder: &'ctx inkwell::builder::Builder<'ctx>,
    variables: &HashMap<String, VariableInfo<'ctx>>,
    struct_types: &HashMap<String, inkwell::types::StructType<'ctx>>,
    target: &Expression,
) -> (AsmOutPlace<'ctx>, BasicTypeEnum<'ctx>) {
    match target {
        Expression::Variable(name) => {
            let info = variables
                .get(name)
                .unwrap_or_else(|| panic!("Output var '{}' not found", name));
            let elem_ty = llvm_type_of_wave(context, &info.ty, struct_types);
            (
                AsmOutPlace::VarAlloca {
                    ptr: info.ptr,
                    elem_ty,
                },
                elem_ty,
            )
        }

        Expression::Deref(inner) => {
            match inner.as_ref() {
                Expression::Variable(name) => {
                    let info = variables
                        .get(name)
                        .unwrap_or_else(|| panic!("Pointer var '{}' not found", name));

                    // 1) load pointer value from var slot (typed)
                    let ptr_ty = llvm_type_of_wave(context, &info.ty, struct_types);
                    let loaded = builder
                        .build_load(ptr_ty, info.ptr, "asm_out_ptr")
                        .unwrap()
                        .as_basic_value_enum();

                    let dst_ptr = match loaded {
                        BasicValueEnum::PointerValue(p) => p,
                        _ => panic!("deref target '{}' is not a pointer value", name),
                    };

                    // 2) pointee type from WaveType (opaque pointer safe)
                    let elem_ty = match &info.ty {
                        WaveType::Pointer(inner_ty) => {
                            llvm_type_of_wave(context, inner_ty, struct_types)
                        }
                        WaveType::String => context.i8_type().as_basic_type_enum(),
                        other => panic!("out(*{}) requires pointer/string, got {:?}", name, other),
                    };

                    (
                        AsmOutPlace::MemPtr {
                            ptr: dst_ptr,
                            elem_ty,
                        },
                        elem_ty,
                    )
                }
                other => panic!("Unsupported deref out target: {:?}", other),
            }
        }

        other => panic!(
            "out(...) target must be variable or deref var for now: {:?}",
            other
        ),
    }
}

fn store_asm_out_place<'ctx>(
    context: &'ctx inkwell::context::Context,
    builder: &'ctx inkwell::builder::Builder<'ctx>,
    place: &AsmOutPlace<'ctx>,
    value: BasicValueEnum<'ctx>,
    name: &str,
) {
    match place {
        AsmOutPlace::VarAlloca { ptr, elem_ty } => {
            let v = coerce_basic_value_for_store(context, builder, value, *elem_ty, name);
            builder.build_store(*ptr, v).unwrap();
        }
        AsmOutPlace::MemPtr { ptr, elem_ty } => {
            let v = coerce_basic_value_for_store(context, builder, value, *elem_ty, name);
            builder.build_store(*ptr, v).unwrap();
        }
    }
}

fn coerce_basic_value_for_store<'ctx>(
    _context: &'ctx inkwell::context::Context,
    builder: &'ctx inkwell::builder::Builder<'ctx>,
    value: BasicValueEnum<'ctx>,
    dst_ty: BasicTypeEnum<'ctx>,
    name: &str,
) -> BasicValueEnum<'ctx> {
    if value.get_type() == dst_ty {
        return value;
    }

    // dst: int
    if dst_ty.is_int_type() {
        let dst_int = dst_ty.into_int_type();

        if value.is_int_value() {
            let v = value.into_int_value();
            let src_bits = v.get_type().get_bit_width();
            let dst_bits = dst_int.get_bit_width();

            if src_bits == dst_bits {
                return v.as_basic_value_enum();
            } else if src_bits > dst_bits {
                return builder
                    .build_int_truncate(v, dst_int, "asm_int_trunc")
                    .unwrap()
                    .as_basic_value_enum();
            } else {
                return builder
                    .build_int_z_extend(v, dst_int, "asm_int_zext")
                    .unwrap()
                    .as_basic_value_enum();
            }
        }

        if value.is_pointer_value() {
            return builder
                .build_ptr_to_int(value.into_pointer_value(), dst_int, "asm_ptr_to_int")
                .unwrap()
                .as_basic_value_enum();
        }

        if value.is_float_value() {
            return builder
                .build_float_to_signed_int(value.into_float_value(), dst_int, "asm_fptosi")
                .unwrap()
                .as_basic_value_enum();
        }

        panic!("Cannot coerce asm output '{}' to int {:?}", name, dst_ty);
    }

    // dst: float
    if dst_ty.is_float_type() {
        let dst_float = dst_ty.into_float_type();

        if value.is_float_value() {
            let v = value.into_float_value();

            if let Ok(tr) = builder.build_float_trunc(v, dst_float, "asm_fptrunc") {
                return tr.as_basic_value_enum();
            }
            if let Ok(ex) = builder.build_float_ext(v, dst_float, "asm_fpext") {
                return ex.as_basic_value_enum();
            }

            panic!(
                "Cannot coerce asm output '{}' from {:?} to float {:?}",
                name,
                value.get_type(),
                dst_ty
            );
        }

        if value.is_int_value() {
            return builder
                .build_signed_int_to_float(value.into_int_value(), dst_float, "asm_sitofp")
                .unwrap()
                .as_basic_value_enum();
        }

        panic!(
            "Cannot coerce asm output '{}' from {:?} to float {:?}",
            name,
            value.get_type(),
            dst_ty
        );
    }

    // dst: pointer
    if dst_ty.is_pointer_type() {
        let dst_ptr = dst_ty.into_pointer_type();

        if value.is_pointer_value() {
            return builder
                .build_bit_cast(value, dst_ptr.as_basic_type_enum(), "asm_ptrcast")
                .unwrap()
                .as_basic_value_enum();
        }

        if value.is_int_value() {
            return builder
                .build_int_to_ptr(value.into_int_value(), dst_ptr, "asm_inttoptr")
                .unwrap()
                .as_basic_value_enum();
        }

        panic!(
            "Cannot coerce asm output '{}' from {:?} to ptr {:?}",
            name,
            value.get_type(),
            dst_ty
        );
    }

    panic!(
        "Unsupported destination type for asm output '{}': {:?}",
        name, dst_ty
    );
}

fn asm_operand_to_value<'ctx>(
    context: &'ctx inkwell::context::Context,
    builder: &'ctx inkwell::builder::Builder<'ctx>,
    variables: &HashMap<String, VariableInfo<'ctx>>,
    global_consts: &HashMap<String, BasicValueEnum<'ctx>>,
    struct_types: &HashMap<String, inkwell::types::StructType<'ctx>>,
    expr: &Expression,
) -> BasicValueEnum<'ctx> {
    match expr {
        Expression::Literal(Literal::Int(n)) => {
            let s = n.as_str();
            let (neg, digits) = if let Some(rest) = s.strip_prefix('-') {
                (true, rest)
            } else {
                (false, s)
            };

            let mut iv = context
                .i64_type()
                .const_int_from_string(digits, StringRadix::Decimal)
                .unwrap_or_else(|| panic!("invalid int literal: {}", s));

            if neg {
                iv = iv.const_neg();
            }
            iv.as_basic_value_enum()
        }

        Expression::Variable(name) => {
            if let Some(const_val) = global_consts.get(name) {
                *const_val
            } else {
                let info = variables
                    .get(name)
                    .unwrap_or_else(|| panic!("Input variable '{}' not found", name));
                let ty = llvm_type_of_wave(context, &info.ty, struct_types);

                builder
                    .build_load(ty, info.ptr, &format!("asm_in_load_{}", name))
                    .unwrap()
                    .as_basic_value_enum()
            }
        }

        Expression::AddressOf(inner) => match inner.as_ref() {
            Expression::Variable(name) => {
                let info = variables
                    .get(name)
                    .unwrap_or_else(|| panic!("Input variable '{}' not found", name));
                info.ptr.as_basic_value_enum()
            }
            _ => panic!("Unsupported asm address-of operand: {:?}", inner),
        },

        Expression::Grouped(inner) => asm_operand_to_value(
            context,
            builder,
            variables,
            global_consts,
            struct_types,
            inner,
        ),

        Expression::Deref(inner) => match inner.as_ref() {
            Expression::Variable(name) => {
                let info = variables
                    .get(name)
                    .unwrap_or_else(|| panic!("Input pointer var '{}' not found", name));

                // 1) load pointer value from slot (typed)
                let ptr_ty = llvm_type_of_wave(context, &info.ty, struct_types);
                let pv_val = builder
                    .build_load(ptr_ty, info.ptr, "asm_in_ptr")
                    .unwrap()
                    .as_basic_value_enum();

                let p = match pv_val {
                    BasicValueEnum::PointerValue(p) => p,
                    _ => panic!("deref input '{}' is not a pointer", name),
                };

                // 2) load pointee value (typed)
                let pointee_ty = match &info.ty {
                    WaveType::Pointer(inner_ty) => {
                        llvm_type_of_wave(context, inner_ty, struct_types)
                    }
                    WaveType::String => context.i8_type().as_basic_type_enum(),
                    other => panic!("deref input '{}' is not pointer/string: {:?}", name, other),
                };

                builder
                    .build_load(pointee_ty, p, "asm_in_deref")
                    .unwrap()
                    .as_basic_value_enum()
            }
            _ => panic!("Unsupported asm deref input: {:?}", inner),
        },

        _ => panic!("Unsupported asm operand expression: {:?}", expr),
    }
}
