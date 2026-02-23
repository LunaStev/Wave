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

use crate::codegen::abi_c::ExternCInfo;
use crate::codegen::types::{wave_type_to_llvm_type, TypeFlavor};
use crate::codegen::{wave_format_to_c, wave_format_to_scanf, VariableInfo};
use crate::expression::lvalue::generate_lvalue_ir;
use crate::expression::rvalue::generate_expression_ir;

use inkwell::module::{Linkage, Module};
use inkwell::targets::TargetData;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicMetadataValueEnum, BasicValue, BasicValueEnum, ValueKind};
use inkwell::{AddressSpace, IntPredicate};

use parser::ast::{Expression, Literal, WaveType};

use std::collections::HashMap;

fn is_cstr_like(wt: &WaveType) -> bool {
    match wt {
        WaveType::String => true,
        WaveType::Pointer(inner) => matches!(inner.as_ref(), WaveType::Byte | WaveType::Char),
        _ => false,
    }
}

fn struct_name_from_wave_type(wt: &WaveType) -> Option<&str> {
    match wt {
        WaveType::Struct(name) => Some(name.as_str()),
        WaveType::Pointer(inner) => match inner.as_ref() {
            WaveType::Struct(name) => Some(name.as_str()),
            _ => None,
        },
        _ => None,
    }
}

fn wave_type_of_expr<'ctx>(
    e: &Expression,
    vars: &HashMap<String, VariableInfo<'ctx>>,
    struct_field_types: &HashMap<String, HashMap<String, WaveType>>,
) -> Option<WaveType> {
    match e {
        Expression::Variable(name) => vars.get(name).map(|v| v.ty.clone()),
        Expression::Grouped(inner) => wave_type_of_expr(inner, vars, struct_field_types),
        Expression::Literal(Literal::String(_)) => Some(WaveType::String),
        Expression::AddressOf(inner) => wave_type_of_expr(inner, vars, struct_field_types)
            .map(|t| WaveType::Pointer(Box::new(t))),

        // *p -> pointee type
        Expression::Deref(inner) => {
            let inner_ty = wave_type_of_expr(inner, vars, struct_field_types)?;
            match inner_ty {
                WaveType::Pointer(t) => Some((*t).clone()),
                WaveType::String => Some(WaveType::Byte),
                _ => None,
            }
        }

        Expression::IndexAccess { target, .. } => {
            let target_ty = wave_type_of_expr(target, vars, struct_field_types)?;
            match target_ty {
                WaveType::Array(inner, _) => Some((*inner).clone()),
                WaveType::Pointer(inner) => match *inner {
                    WaveType::Array(elem, _) => Some(*elem),
                    other => Some(other),
                },
                WaveType::String => Some(WaveType::Byte),
                _ => None,
            }
        }

        Expression::FieldAccess { object, field } => {
            let object_ty = wave_type_of_expr(object, vars, struct_field_types)?;
            let struct_name = struct_name_from_wave_type(&object_ty)?;
            struct_field_types
                .get(struct_name)
                .and_then(|fields| fields.get(field))
                .cloned()
        }

        _ => None,
    }
}

fn llvm_type_of_wave<'ctx>(
    context: &'ctx inkwell::context::Context,
    wt: &WaveType,
    struct_types: &HashMap<String, inkwell::types::StructType<'ctx>>,
) -> BasicTypeEnum<'ctx> {
    wave_type_to_llvm_type(context, wt, struct_types, TypeFlavor::Value)
}

fn wave_type_from_basic_for_scanf<'ctx>(
    context: &'ctx inkwell::context::Context,
    bt: BasicTypeEnum<'ctx>,
) -> WaveType {
    match bt {
        BasicTypeEnum::IntType(it) => {
            let bw = it.get_bit_width();
            match bw {
                1 => WaveType::Bool,
                8 => WaveType::Char,
                16 => WaveType::Int(16),
                32 => WaveType::Int(32),
                64 => WaveType::Int(64),
                128 => WaveType::Int(128),
                other => WaveType::Int(other as u16),
            }
        }
        BasicTypeEnum::FloatType(ft) => {
            if ft == context.f32_type() {
                WaveType::Float(32)
            } else {
                WaveType::Float(64)
            }
        }
        other => panic!("Unsupported scanf lvalue type: {:?}", other),
    }
}

fn lvalue_elem_basic_type<'ctx>(
    context: &'ctx inkwell::context::Context,
    e: &Expression,
    vars: &HashMap<String, VariableInfo<'ctx>>,
    struct_types: &HashMap<String, inkwell::types::StructType<'ctx>>,
    struct_field_indices: &HashMap<String, HashMap<String, u32>>,
    struct_field_types: &HashMap<String, HashMap<String, WaveType>>,
) -> BasicTypeEnum<'ctx> {
    match e {
        Expression::Grouped(inner) => {
            lvalue_elem_basic_type(
                context,
                inner,
                vars,
                struct_types,
                struct_field_indices,
                struct_field_types,
            )
        }

        Expression::Variable(name) => {
            let vi = vars.get(name).unwrap_or_else(|| panic!("var '{}' not found", name));
            llvm_type_of_wave(context, &vi.ty, struct_types)
        }

        Expression::Deref(inner) => {
            if let Expression::Variable(name) = inner.as_ref() {
                let vi = vars.get(name).unwrap_or_else(|| panic!("ptr var '{}' not found", name));
                match &vi.ty {
                    WaveType::Pointer(t) => llvm_type_of_wave(context, t.as_ref(), struct_types),
                    WaveType::String => context.i8_type().as_basic_type_enum(),
                    other => panic!("deref lvalue expects pointer/string, got {:?}", other),
                }
            } else {
                panic!("unsupported deref lvalue: {:?}", inner);
            }
        }

        Expression::FieldAccess { object, field } => {
            let obj_wt = wave_type_of_expr(object, vars, struct_field_types)
                .unwrap_or_else(|| panic!("cannot infer object type for field access"));

            let struct_name = struct_name_from_wave_type(&obj_wt)
                .unwrap_or_else(|| panic!("field access on non-struct: {:?}", obj_wt))
                .to_string();

            let st = *struct_types
                .get(&struct_name)
                .unwrap_or_else(|| panic!("Struct '{}' not found", struct_name));

            let fmap = struct_field_indices
                .get(&struct_name)
                .unwrap_or_else(|| panic!("Field map '{}' not found", struct_name));

            let idx = *fmap
                .get(field)
                .unwrap_or_else(|| panic!("Field '{}' not found in '{}'", field, struct_name));

            st.get_field_type_at_index(idx)
                .unwrap_or_else(|| panic!("No field type at index {} for '{}'", idx, struct_name))
        }

        other => panic!("unsupported input lvalue: {:?}", other),
    }
}

pub(super) fn gen_print_literal_ir<'ctx>(
    context: &'ctx inkwell::context::Context,
    builder: &'ctx inkwell::builder::Builder<'ctx>,
    module: &'ctx Module<'ctx>,
    string_counter: &mut usize,
    message: &str,
) {
    let global_name = format!("str_{}", *string_counter);
    *string_counter += 1;

    let mut bytes = message.as_bytes().to_vec();
    bytes.push(0);

    let const_str = context.const_string(&bytes, false);
    let str_ty = context.i8_type().array_type(bytes.len() as u32);

    let global = module.add_global(str_ty, None, &global_name);
    global.set_initializer(&const_str);
    global.set_linkage(Linkage::Private);
    global.set_constant(true);

    let printf_type = context.i32_type().fn_type(
        &[context.i8_type().ptr_type(AddressSpace::default()).into()],
        true,
    );
    let printf_func = module
        .get_function("printf")
        .unwrap_or_else(|| module.add_function("printf", printf_type, None));

    let zero = context.i32_type().const_zero();
    let indices = [zero, zero];

    let gep = unsafe {
        builder
            .build_gep(str_ty, global.as_pointer_value(), &indices, "gep")
            .unwrap()
    };

    builder
        .build_call(printf_func, &[gep.into()], "printf_call")
        .unwrap();
}

pub(super) fn gen_print_format_ir<'ctx>(
    context: &'ctx inkwell::context::Context,
    builder: &'ctx inkwell::builder::Builder<'ctx>,
    module: &'ctx Module<'ctx>,
    string_counter: &mut usize,
    format: &str,
    args: &[Expression],
    variables: &mut HashMap<String, VariableInfo<'ctx>>,
    global_consts: &HashMap<String, BasicValueEnum<'ctx>>,
    struct_types: &HashMap<String, inkwell::types::StructType<'ctx>>,
    struct_field_indices: &HashMap<String, HashMap<String, u32>>,
    struct_field_types: &HashMap<String, HashMap<String, WaveType>>,
    target_data: &'ctx TargetData,
    extern_c_info: &HashMap<String, ExternCInfo<'ctx>>,
) {
    let mut fmt_types: Vec<BasicTypeEnum<'ctx>> = Vec::with_capacity(args.len());
    let mut arg_is_cstr: Vec<bool> = Vec::with_capacity(args.len());

    let mut printf_vals: Vec<BasicMetadataValueEnum<'ctx>> = Vec::with_capacity(args.len());

    let void_ptr_ty = context.i8_type().ptr_type(AddressSpace::default()).as_basic_type_enum();

    for arg in args {
        let val = generate_expression_ir(
            context,
            builder,
            arg,
            variables,
            module,
            None,
            global_consts,
            struct_types,
            struct_field_indices,
            target_data,
            extern_c_info,
        );

        let wt = wave_type_of_expr(arg, variables, struct_field_types);
        let is_cstr = wt.as_ref().map(|t| is_cstr_like(t)).unwrap_or(false);

        match val {
            BasicValueEnum::IntValue(iv) => {
                let bw = iv.get_type().get_bit_width();
                if bw < 32 {
                    // C varargs: promote small ints to i32
                    let signed = wt.as_ref().map(|t| matches!(t, WaveType::Int(_))).unwrap_or(false);

                    let promoted = if signed {
                        builder
                            .build_int_s_extend(iv, context.i32_type(), "int_promote")
                            .unwrap()
                            .as_basic_value_enum()
                    } else {
                        builder
                            .build_int_z_extend(iv, context.i32_type(), "int_promote")
                            .unwrap()
                            .as_basic_value_enum()
                    };

                    fmt_types.push(context.i32_type().as_basic_type_enum());
                    arg_is_cstr.push(false);
                    printf_vals.push(promoted.into());
                } else {
                    fmt_types.push(iv.get_type().as_basic_type_enum());
                    arg_is_cstr.push(false);
                    printf_vals.push(iv.as_basic_value_enum().into());
                }
            }

            BasicValueEnum::FloatValue(fv) => {
                // C varargs: float -> double
                let dv = if fv.get_type() == context.f64_type() {
                    fv.as_basic_value_enum()
                } else {
                    builder
                        .build_float_ext(fv, context.f64_type(), "cast_to_double")
                        .unwrap()
                        .as_basic_value_enum()
                };

                fmt_types.push(context.f64_type().as_basic_type_enum());
                arg_is_cstr.push(false);
                printf_vals.push(dv.into());
            }

            BasicValueEnum::PointerValue(pv) => {
                let vp = builder
                    .build_bit_cast(pv, void_ptr_ty.into_pointer_type(), "ptr_to_void")
                    .unwrap()
                    .as_basic_value_enum();

                fmt_types.push(void_ptr_ty);
                arg_is_cstr.push(is_cstr);
                printf_vals.push(vp.into());
            }

            other => {
                let ty = other.get_type();
                let tmp = builder.build_alloca(ty, "printf_tmp").unwrap();
                builder.build_store(tmp, other).unwrap();

                let vp = builder
                    .build_bit_cast(tmp, void_ptr_ty.into_pointer_type(), "tmp_to_void")
                    .unwrap()
                    .as_basic_value_enum();

                fmt_types.push(void_ptr_ty);
                arg_is_cstr.push(false);
                printf_vals.push(vp.into());
            }
        }
    }

    let c_format_string = wave_format_to_c(context, format, &fmt_types, &arg_is_cstr);

    let global_name = format!("str_{}", *string_counter);
    *string_counter += 1;

    let mut bytes = c_format_string.as_bytes().to_vec();
    bytes.push(0);

    let const_str = context.const_string(&bytes, false);
    let str_ty = context.i8_type().array_type(bytes.len() as u32);

    let global = module.add_global(str_ty, None, &global_name);
    global.set_initializer(&const_str);
    global.set_linkage(Linkage::Private);
    global.set_constant(true);

    let printf_type = context.i32_type().fn_type(
        &[context.i8_type().ptr_type(AddressSpace::default()).into()],
        true,
    );
    let printf_func = module
        .get_function("printf")
        .unwrap_or_else(|| module.add_function("printf", printf_type, None));

    let zero = context.i32_type().const_zero();
    let indices = [zero, zero];

    let gep = unsafe {
        builder
            .build_gep(str_ty, global.as_pointer_value(), &indices, "gep")
            .unwrap()
    };

    let mut printf_args: Vec<BasicMetadataValueEnum<'ctx>> =
        Vec::with_capacity(1 + printf_vals.len());
    printf_args.push(gep.into());
    printf_args.extend(printf_vals);

    builder
        .build_call(printf_func, &printf_args, "printf_call")
        .unwrap();
}

pub(super) fn gen_input_ir<'ctx>(
    context: &'ctx inkwell::context::Context,
    builder: &'ctx inkwell::builder::Builder<'ctx>,
    module: &'ctx Module<'ctx>,
    string_counter: &mut usize,
    format: &str,
    args: &[Expression],
    variables: &mut HashMap<String, VariableInfo<'ctx>>,
    global_consts: &HashMap<String, BasicValueEnum<'ctx>>,
    struct_types: &HashMap<String, inkwell::types::StructType<'ctx>>,
    struct_field_indices: &HashMap<String, HashMap<String, u32>>,
    struct_field_types: &HashMap<String, HashMap<String, WaveType>>,
    target_data: &'ctx TargetData,
    extern_c_info: &HashMap<String, ExternCInfo<'ctx>>,
) {
    let mut ptrs = Vec::with_capacity(args.len());
    let mut wave_types: Vec<WaveType> = Vec::with_capacity(args.len());

    for arg in args {
        let ptr = generate_lvalue_ir(
            context,
            builder,
            arg,
            variables,
            module,
            global_consts,
            struct_types,
            struct_field_indices,
            target_data,
            extern_c_info,
        );

        let wt = wave_type_of_expr(arg, variables, struct_field_types).unwrap_or_else(|| {
            let elem_bt = lvalue_elem_basic_type(
                context,
                arg,
                variables,
                struct_types,
                struct_field_indices,
                struct_field_types,
            );
            wave_type_from_basic_for_scanf(context, elem_bt)
        });

        wave_types.push(wt);
        ptrs.push(ptr);
    }

    let c_format_string = wave_format_to_scanf(format, &wave_types);

    let global_name = format!("str_{}", *string_counter);
    *string_counter += 1;

    let mut bytes = c_format_string.as_bytes().to_vec();
    bytes.push(0);

    let const_str = context.const_string(&bytes, false);
    let str_ty = context.i8_type().array_type(bytes.len() as u32);

    let global = module.add_global(str_ty, None, &global_name);
    global.set_initializer(&const_str);
    global.set_linkage(Linkage::Private);
    global.set_constant(true);

    let scanf_type = context.i32_type().fn_type(
        &[context.i8_type().ptr_type(AddressSpace::default()).into()],
        true,
    );
    let scanf_func = module
        .get_function("scanf")
        .unwrap_or_else(|| module.add_function("scanf", scanf_type, None));

    let zero = context.i32_type().const_zero();
    let indices = [zero, zero];

    let fmt_gep = unsafe {
        builder
            .build_gep(str_ty, global.as_pointer_value(), &indices, "fmt_gep")
            .unwrap()
    };

    let mut scanf_args: Vec<BasicMetadataValueEnum<'ctx>> =
        Vec::with_capacity(1 + ptrs.len());
    scanf_args.push(fmt_gep.into());
    for p in ptrs {
        scanf_args.push(p.into());
    }

    let call = builder
        .build_call(scanf_func, &scanf_args, "scanf_call")
        .unwrap();

    let ret_i32 = match call.try_as_basic_value() {
        ValueKind::Basic(v) => v.into_int_value(),
        ValueKind::Instruction(_) => panic!("scanf should return i32 value"),
    };

    let expected = context.i32_type().const_int(args.len() as u64, false);
    let ok = builder
        .build_int_compare(IntPredicate::EQ, ret_i32, expected, "scanf_ok")
        .unwrap();

    let cur_bb = builder.get_insert_block().unwrap();
    let cur_fn = cur_bb.get_parent().unwrap();

    let ok_bb = context.append_basic_block(cur_fn, "input_ok");
    let fail_bb = context.append_basic_block(cur_fn, "input_fail");
    let cont_bb = context.append_basic_block(cur_fn, "input_cont");

    builder.build_conditional_branch(ok, ok_bb, fail_bb).unwrap();

    builder.position_at_end(fail_bb);

    let exit_ty = context.void_type().fn_type(&[context.i32_type().into()], false);
    let exit_fn = module
        .get_function("exit")
        .unwrap_or_else(|| module.add_function("exit", exit_ty, None));

    builder
        .build_call(exit_fn, &[context.i32_type().const_int(1, false).into()], "exit_call")
        .unwrap();
    builder.build_unreachable().unwrap();

    builder.position_at_end(ok_bb);
    builder.build_unconditional_branch(cont_bb).unwrap();

    builder.position_at_end(cont_bb);
}
