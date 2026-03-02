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

use crate::expression::rvalue::generate_expression_ir;
use crate::codegen::{generate_address_and_type_ir, generate_address_ir, wave_type_to_llvm_type, VariableInfo};
use inkwell::module::Module;
use inkwell::types::{AnyTypeEnum, BasicType, BasicTypeEnum, StructType};
use inkwell::values::{BasicValue, BasicValueEnum};
use parser::ast::{Expression, Mutability};
use std::collections::HashMap;
use inkwell::targets::TargetData;
use crate::codegen::abi_c::ExternCInfo;
use crate::codegen::types::TypeFlavor;
use crate::statement::variable::{coerce_basic_value, CoercionMode};

pub(super) fn gen_assign_ir<'ctx>(
    context: &'ctx inkwell::context::Context,
    builder: &'ctx inkwell::builder::Builder<'ctx>,
    module: &'ctx Module<'ctx>,
    variable: &str,
    value: &Expression,
    variables: &mut HashMap<String, VariableInfo<'ctx>>,
    global_consts: &HashMap<String, BasicValueEnum<'ctx>>,
    struct_types: &HashMap<String, StructType<'ctx>>,
    struct_field_indices: &HashMap<String, HashMap<String, u32>>,
    target_data: &'ctx TargetData,
    extern_c_info: &HashMap<String, ExternCInfo<'ctx>>,
) {
    if variable == "deref" {
        if let Expression::BinaryExpression { left, right, .. } = value {
            if let Expression::Deref(inner_expr) = &**left {
                let target_ptr =
                    generate_address_ir(context, builder, inner_expr, variables, module, struct_types, struct_field_indices);

                let expected_elem_ty: BasicTypeEnum<'ctx> = match &**inner_expr {
                    Expression::Variable(name) => {
                        let info = variables.get(name).unwrap_or_else(|| panic!("Pointer var '{}' not declared", name));
                        match &info.ty {
                            parser::ast::WaveType::Pointer(inner) => {
                                wave_type_to_llvm_type(context, inner.as_ref(), struct_types, TypeFlavor::Value)
                            }
                            parser::ast::WaveType::String => context.i8_type().as_basic_type_enum(),
                            other => panic!("deref target is not a pointer/string: {:?}", other),
                        }
                    }
                    _ => {
                        let (_, ty) = generate_address_and_type_ir(
                            context,
                            builder,
                            inner_expr,
                            variables,
                            module,
                            struct_types,
                            struct_field_indices,
                        );
                        ty
                    }
                };

                let mut val = generate_expression_ir(
                    context,
                    builder,
                    right,
                    variables,
                    module,
                    Some(expected_elem_ty),
                    global_consts,
                    struct_types,
                    struct_field_indices,
                    target_data,
                    extern_c_info,
                );

                if val.get_type() != expected_elem_ty {
                    val = coerce_basic_value(context, builder, val, expected_elem_ty, "deref_assign_cast", CoercionMode::Implicit);
                }

                builder.build_store(target_ptr, val).unwrap();

            }
        }
        return;
    }

    let (dst_ptr, dst_mutability, dst_wave_ty) = {
        let info = variables
            .get(variable)
            .unwrap_or_else(|| panic!("Variable {} not declared", variable));
        (info.ptr, info.mutability.clone(), info.ty.clone())
    };

    if matches!(dst_mutability, Mutability::Let | Mutability::Const) {
        panic!("Cannot assign to immutable variable '{}'", variable);
    }

    let element_type: BasicTypeEnum<'ctx> =
        wave_type_to_llvm_type(context, &dst_wave_ty, struct_types, TypeFlavor::Value);

    if matches!(value, Expression::Null) && !matches!(dst_wave_ty, parser::ast::WaveType::Pointer(_))
    {
        panic!(
            "null literal can only be assigned to ptr<T> (variable '{}': {:?})",
            variable, dst_wave_ty
        );
    }

    let val = generate_expression_ir(
        context,
        builder,
        value,
        variables,
        module,
        Some(element_type),
        global_consts,
        struct_types,
        struct_field_indices,
        target_data,
        extern_c_info,
    );

    let mut casted_val = val;

    if casted_val.get_type() != element_type {
        casted_val = coerce_basic_value(
            context,
            builder,
            casted_val,
            element_type,
            "assign_cast",
            CoercionMode::Implicit,
        );
    }

    builder.build_store(dst_ptr, casted_val).unwrap();

}
