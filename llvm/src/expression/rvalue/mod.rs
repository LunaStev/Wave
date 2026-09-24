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

//! Shared environment and entry point for expression value lowering.
//!
//! `ExprGenEnv` carries the LLVM construction state plus Wave semantic tables.
//! An optional expected type flows downward to resolve literals, null pointers,
//! aggregates, and ABI-sensitive coercions without reconstructing types from AST
//! shape.

use crate::codegen::abi_c::ExternCInfo;
use crate::codegen::VariableInfo;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::targets::TargetData;
use inkwell::types::{BasicTypeEnum, StructType};
use inkwell::values::BasicValueEnum;
use parser::ast::{Expression, WaveType};
use parser::hir::{HirExpressionType, TypedProgram};
use std::collections::HashMap;

pub mod dispatch;
pub mod utils;

pub mod arrays;
pub mod asm;
pub mod assign;
mod async_runtime;
pub mod binary;
pub mod calls;
pub mod cast;
pub mod const_projection;
pub mod incdec;
pub mod index;
pub mod literals;
pub mod pointers;
pub mod structs;
pub mod unary;
pub mod variables;
pub mod variants;

pub struct ProtoInfo<'ctx> {
    pub vtable_ty: StructType<'ctx>,
    pub fat_ty: StructType<'ctx>,
    pub methods: Vec<String>,
}

pub(crate) struct ExprGenEnv<'ctx, 'a> {
    pub program: &'a TypedProgram,
    pub context: &'ctx Context,
    pub builder: &'ctx Builder<'ctx>,
    pub variables: &'a mut HashMap<String, VariableInfo<'ctx>>,
    pub module: &'ctx Module<'ctx>,
    pub global_consts: &'a HashMap<String, BasicValueEnum<'ctx>>,
    pub struct_types: &'a HashMap<String, StructType<'ctx>>,
    pub struct_field_indices: &'a HashMap<String, HashMap<String, u32>>,
    pub target_data: &'a TargetData,
    pub extern_c_info: &'a HashMap<String, ExternCInfo<'ctx>>,
}

impl<'ctx, 'a> ExprGenEnv<'ctx, 'a> {
    #[inline]
    pub fn gen(
        &mut self,
        expr: &Expression,
        expected_type: Option<BasicTypeEnum<'ctx>>,
    ) -> BasicValueEnum<'ctx> {
        // Semantic analysis resolves literal-only integer arithmetic, including
        // comparison operands. Never evaluate it at a narrower fallback width
        // or turn integer division into floating division at a conversion site.
        let expected_type = if expr.is_contextual_integer() {
            let integer_type = match self.program.type_of(expr) {
                Some(HirExpressionType::Resolved(ty @ (WaveType::Int(_) | WaveType::Uint(_)))) => {
                    Some(ty)
                }
                _ => self
                    .program
                    .expected_type_of(expr)
                    .filter(|ty| matches!(ty, WaveType::Int(_) | WaveType::Uint(_))),
            };
            integer_type
                .map(|ty| {
                    crate::codegen::types::wave_type_to_llvm_type(
                        self.context,
                        ty,
                        self.struct_types,
                        crate::codegen::types::TypeFlavor::Value,
                    )
                })
                .or(expected_type)
        } else {
            expected_type
        };
        // LLVM integer types erase signedness. Use the validated destination
        // at the conversion boundary and evaluate the floating expression in
        // its own type, including grouped expressions and arithmetic.
        if let (Some(BasicTypeEnum::IntType(destination)), Some(destination_type)) =
            (expected_type, self.program.expected_type_of(expr))
        {
            if matches!(self.wave_type(expr), Some(WaveType::Float(_))) {
                let unsigned =
                    crate::statement::variable::wave_type_is_unsigned(Some(destination_type));
                let value = dispatch::gen_expr(self, expr, None).into_float_value();
                let converted = if unsigned {
                    self.builder
                        .build_float_to_unsigned_int(value, destination, "float_to_uint")
                } else {
                    self.builder
                        .build_float_to_signed_int(value, destination, "float_to_int")
                };
                return converted.unwrap().into();
            }
        }
        dispatch::gen_expr(self, expr, expected_type)
    }

    pub fn wave_type(&self, expr: &Expression) -> Option<WaveType> {
        match self.program.type_of(expr) {
            Some(HirExpressionType::Resolved(ty)) => Some(ty.clone()),
            Some(HirExpressionType::IntegerLiteral) => Some(
                self.program
                    .expected_type_of(expr)
                    .filter(|ty| matches!(ty, WaveType::Int(_) | WaveType::Uint(_)))
                    .cloned()
                    .unwrap_or(WaveType::Int(32)),
            ),
            Some(HirExpressionType::FloatLiteral) => Some(WaveType::Float(32)),
            _ => None,
        }
    }
}

pub fn generate_expression_ir<'ctx, 'a>(
    program: &'a TypedProgram,
    context: &'ctx Context,
    builder: &'ctx Builder<'ctx>,
    expr: &Expression,
    variables: &mut HashMap<String, VariableInfo<'ctx>>,
    module: &'ctx Module<'ctx>,
    expected_type: Option<BasicTypeEnum<'ctx>>,
    global_consts: &HashMap<String, BasicValueEnum<'ctx>>,
    struct_types: &HashMap<String, StructType<'ctx>>,
    struct_field_indices: &HashMap<String, HashMap<String, u32>>,
    target_data: &'ctx TargetData,
    extern_c_info: &HashMap<String, ExternCInfo<'ctx>>,
) -> BasicValueEnum<'ctx> {
    let mut env = ExprGenEnv {
        program,
        context,
        builder,
        variables,
        module,
        global_consts,
        struct_types,
        struct_field_indices,
        target_data,
        extern_c_info,
    };

    env.gen(expr, expected_type)
}
