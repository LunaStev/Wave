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

use crate::llvm_temporary::expression::rvalue::generate_expression_ir;
use crate::llvm_temporary::llvm_codegen::VariableInfo;
use inkwell::module::Module;
use inkwell::types::StructType;
use inkwell::values::BasicValueEnum;
use parser::ast::Expression;
use std::collections::HashMap;
use inkwell::targets::TargetData;
use crate::llvm_temporary::llvm_codegen::abi_c::ExternCInfo;

pub(super) fn gen_expr_stmt_ir<'ctx>(
    context: &'ctx inkwell::context::Context,
    builder: &'ctx inkwell::builder::Builder<'ctx>,
    module: &'ctx Module<'ctx>,
    expr: &Expression,
    variables: &mut HashMap<String, VariableInfo<'ctx>>,
    global_consts: &HashMap<String, BasicValueEnum<'ctx>>,
    struct_types: &HashMap<String, StructType<'ctx>>,
    struct_field_indices: &HashMap<String, HashMap<String, u32>>,
    target_data: &'ctx TargetData,
    extern_c_info: &HashMap<String, ExternCInfo<'ctx>>,
) {
    let _ = generate_expression_ir(
        context,
        builder,
        expr,
        variables,
        module,
        None,
        global_consts,
        struct_types,
        struct_field_indices,
        target_data,
        extern_c_info,
    );
}
