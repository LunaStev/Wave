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

use crate::codegen::VariableInfo;
use crate::expression::rvalue::generate_expression_ir;
use inkwell::basic_block::BasicBlock;
use inkwell::module::Module;
use inkwell::types::{StructType};
use inkwell::values::{AnyValue, BasicValueEnum, FunctionValue};
use inkwell::types::StringRadix;
use inkwell::{FloatPredicate, IntPredicate};
use parser::ast::{ASTNode, Expression, MatchArm, MatchPattern, WaveType};
use std::collections::{HashMap, HashSet};
use inkwell::targets::TargetData;
use crate::codegen::abi_c::ExternCInfo;
use crate::statement::variable::{coerce_basic_value, CoercionMode};

fn truthy_to_i1<'ctx>(
    context: &'ctx inkwell::context::Context,
    builder: &'ctx inkwell::builder::Builder<'ctx>,
    v: BasicValueEnum<'ctx>,
    name: &str,
) -> inkwell::values::IntValue<'ctx> {
    match v {
        BasicValueEnum::IntValue(iv) => {
            if iv.get_type().get_bit_width() == 1 {
                iv
            } else {
                let zero = iv.get_type().const_zero();
                builder
                    .build_int_compare(IntPredicate::NE, iv, zero, name)
                    .unwrap()
            }
        }
        BasicValueEnum::FloatValue(fv) => {
            let zero = fv.get_type().const_float(0.0);
            builder
                .build_float_compare(FloatPredicate::ONE, fv, zero, name)
                .unwrap()
        }
        BasicValueEnum::PointerValue(pv) => builder.build_is_not_null(pv, name).unwrap(),
        _ => panic!("Unsupported condition type"),
    }
}

fn parse_signed_decimal<'a>(s: &'a str) -> (bool, &'a str) {
    if let Some(rest) = s.strip_prefix('-') {
        (true, rest)
    } else {
        (false, s)
    }
}

fn parse_int_radix(s: &str) -> (StringRadix, &str) {
    if let Some(rest) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        (StringRadix::Binary, rest)
    } else if let Some(rest) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        (StringRadix::Hexadecimal, rest)
    } else if let Some(rest) = s.strip_prefix("0o").or_else(|| s.strip_prefix("0O")) {
        (StringRadix::Octal, rest)
    } else {
        (StringRadix::Decimal, s)
    }
}

fn eval_match_case_const<'ctx>(
    discr_ty: inkwell::types::IntType<'ctx>,
    pattern: &MatchPattern,
    global_consts: &HashMap<String, BasicValueEnum<'ctx>>,
) -> inkwell::values::IntValue<'ctx> {
    match pattern {
        MatchPattern::Int(raw) => {
            let text = raw.as_str();
            let (neg, digits_src) = parse_signed_decimal(text);
            let (radix, digits) = parse_int_radix(digits_src);

            let mut iv = discr_ty
                .const_int_from_string(digits, radix)
                .unwrap_or_else(|| panic!("invalid integer literal in match case: {}", raw));
            if neg {
                iv = iv.const_neg();
            }
            iv
        }
        MatchPattern::Ident(name) => {
            let Some(v) = global_consts.get(name) else {
                panic!(
                    "match case identifier '{}' is not a known integer/enum constant",
                    name
                );
            };

            match *v {
                BasicValueEnum::IntValue(iv) => {
                    if iv.get_type().get_bit_width() != discr_ty.get_bit_width() {
                        panic!(
                            "match case '{}' type width mismatch: case i{}, match i{}",
                            name,
                            iv.get_type().get_bit_width(),
                            discr_ty.get_bit_width()
                        );
                    }
                    iv
                }
                other => panic!(
                    "match case identifier '{}' must resolve to integer/enum constant, got {:?}",
                    name,
                    other.get_type()
                ),
            }
        }
        MatchPattern::Wildcard => {
            panic!("internal error: wildcard cannot be lowered as a switch case constant");
        }
    }
}

pub(super) fn gen_if_ir<'ctx>(
    context: &'ctx inkwell::context::Context,
    builder: &'ctx inkwell::builder::Builder<'ctx>,
    module: &'ctx Module<'ctx>,
    string_counter: &mut usize,
    condition: &Expression,
    body: &[ASTNode],
    else_if_blocks: &Option<Box<Vec<(Expression, Vec<ASTNode>)>>>,
    else_block: &Option<Box<Vec<ASTNode>>>,
    variables: &mut HashMap<String, VariableInfo<'ctx>>,
    loop_exit_stack: &mut Vec<BasicBlock<'ctx>>,
    loop_continue_stack: &mut Vec<BasicBlock<'ctx>>,
    current_function: FunctionValue<'ctx>,
    global_consts: &HashMap<String, BasicValueEnum<'ctx>>,
    struct_types: &HashMap<String, StructType<'ctx>>,
    struct_field_indices: &HashMap<String, HashMap<String, u32>>,
    struct_field_types: &HashMap<String, HashMap<String, WaveType>>,
    target_data: &'ctx TargetData,
    extern_c_info: &HashMap<String, ExternCInfo<'ctx>>,
) {
    let current_fn = builder.get_insert_block().unwrap().get_parent().unwrap();

    let cond_any = generate_expression_ir(
        context,
        builder,
        condition,
        variables,
        module,
        None,
        global_consts,
        struct_types,
        struct_field_indices,
        target_data,
        extern_c_info,
    );

    let cond_i1 = truthy_to_i1(context, builder, cond_any, "if_cond");

    let then_block = context.append_basic_block(current_fn, "then");
    let else_block_bb = context.append_basic_block(current_fn, "else");
    let merge_block = context.append_basic_block(current_fn, "merge");

    builder
        .build_conditional_branch(cond_i1, then_block, else_block_bb)
        .unwrap();

    builder.position_at_end(then_block);
    for stmt in body {
        super::generate_statement_ir(
            context,
            builder,
            module,
            string_counter,
            stmt,
            variables,
            loop_exit_stack,
            loop_continue_stack,
            current_function,
            global_consts,
            struct_types,
            struct_field_indices,
            struct_field_types,
            target_data,
            extern_c_info,
        );
    }

    let then_end = builder.get_insert_block().unwrap();
    if then_end.get_terminator().is_none() {
        builder.build_unconditional_branch(merge_block).unwrap();
    }

    builder.position_at_end(else_block_bb);
    let mut current_check_bb = else_block_bb;

    if let Some(else_ifs) = else_if_blocks {
        for (else_if_cond, else_if_body) in else_ifs.iter() {
            builder.position_at_end(current_check_bb);

            let c_any = generate_expression_ir(
                context,
                builder,
                else_if_cond,
                variables,
                module,
                None,
                global_consts,
                struct_types,
                struct_field_indices,
                target_data,
                extern_c_info,
            );
            let c_i1 = truthy_to_i1(context, builder, c_any, "elif_cond");

            let then_bb = context.append_basic_block(current_fn, "else_if_then");
            let next_check_bb = context.append_basic_block(current_fn, "next_else_if");

            builder
                .build_conditional_branch(c_i1, then_bb, next_check_bb)
                .unwrap();

            builder.position_at_end(then_bb);
            for stmt in else_if_body {
                super::generate_statement_ir(
                    context,
                    builder,
                    module,
                    string_counter,
                    stmt,
                    variables,
                    loop_exit_stack,
                    loop_continue_stack,
                    current_function,
                    global_consts,
                    struct_types,
                    struct_field_indices,
                    struct_field_types,
                    target_data,
                    extern_c_info,
                );
            }

            let end_bb = builder.get_insert_block().unwrap();
            if end_bb.get_terminator().is_none() {
                builder.build_unconditional_branch(merge_block).unwrap();
            }

            current_check_bb = next_check_bb;
        }

        builder.position_at_end(current_check_bb);

        if let Some(else_body) = else_block {
            for stmt in else_body.iter() {
                super::generate_statement_ir(
                    context,
                    builder,
                    module,
                    string_counter,
                    stmt,
                    variables,
                    loop_exit_stack,
                    loop_continue_stack,
                    current_function,
                    global_consts,
                    struct_types,
                    struct_field_indices,
                    struct_field_types,
                    target_data,
                    extern_c_info,
                );
            }
        }

        let else_end = builder.get_insert_block().unwrap();
        if else_end.get_terminator().is_none() {
            builder.build_unconditional_branch(merge_block).unwrap();
        }

        builder.position_at_end(merge_block);
        return;
    }

    builder.position_at_end(current_check_bb);

    if let Some(else_body) = else_block.as_deref() {
        for stmt in else_body.iter() {
            super::generate_statement_ir(
                context,
                builder,
                module,
                string_counter,
                stmt,
                variables,
                loop_exit_stack,
                loop_continue_stack,
                current_function,
                global_consts,
                struct_types,
                struct_field_indices,
                struct_field_types,
                target_data,
                extern_c_info,
            );
        }
    }

    let else_end = builder.get_insert_block().unwrap();
    if else_end.get_terminator().is_none() {
        builder.build_unconditional_branch(merge_block).unwrap();
    }

    builder.position_at_end(merge_block);
}

pub(super) fn gen_while_ir<'ctx>(
    context: &'ctx inkwell::context::Context,
    builder: &'ctx inkwell::builder::Builder<'ctx>,
    module: &'ctx Module<'ctx>,
    string_counter: &mut usize,
    condition: &Expression,
    body: &[ASTNode],
    variables: &mut HashMap<String, VariableInfo<'ctx>>,
    loop_exit_stack: &mut Vec<BasicBlock<'ctx>>,
    loop_continue_stack: &mut Vec<BasicBlock<'ctx>>,
    current_function: FunctionValue<'ctx>,
    global_consts: &HashMap<String, BasicValueEnum<'ctx>>,
    struct_types: &HashMap<String, StructType<'ctx>>,
    struct_field_indices: &HashMap<String, HashMap<String, u32>>,
    struct_field_types: &HashMap<String, HashMap<String, WaveType>>,
    target_data: &'ctx TargetData,
    extern_c_info: &HashMap<String, ExternCInfo<'ctx>>,
) {
    let current_fn = builder.get_insert_block().unwrap().get_parent().unwrap();

    let cond_block = context.append_basic_block(current_fn, "while.cond");
    let body_block = context.append_basic_block(current_fn, "while.body");
    let merge_block = context.append_basic_block(current_fn, "while.end");

    loop_exit_stack.push(merge_block);
    loop_continue_stack.push(cond_block);

    builder.build_unconditional_branch(cond_block).unwrap();
    builder.position_at_end(cond_block);

    let cond_val = generate_expression_ir(
        context,
        builder,
        condition,
        variables,
        module,
        None,
        global_consts,
        struct_types,
        struct_field_indices,
        target_data,
        extern_c_info,
    );

    let cond_bool = truthy_to_i1(context, builder, cond_val, "while_cond");
    builder
        .build_conditional_branch(cond_bool, body_block, merge_block)
        .unwrap();

    builder.position_at_end(body_block);
    for stmt in body {
        super::generate_statement_ir(
            context,
            builder,
            module,
            string_counter,
            stmt,
            variables,
            loop_exit_stack,
            loop_continue_stack,
            current_function,
            global_consts,
            struct_types,
            struct_field_indices,
            struct_field_types,
            target_data,
            extern_c_info,
        );
    }

   let end_bb = builder.get_insert_block().unwrap();
    if end_bb.get_terminator().is_none() {
        builder.build_unconditional_branch(cond_block).unwrap();
    }

    loop_exit_stack.pop();
    loop_continue_stack.pop();

    builder.position_at_end(merge_block);
}

pub(super) fn gen_match_ir<'ctx>(
    context: &'ctx inkwell::context::Context,
    builder: &'ctx inkwell::builder::Builder<'ctx>,
    module: &'ctx Module<'ctx>,
    string_counter: &mut usize,
    value: &Expression,
    arms: &[MatchArm],
    variables: &mut HashMap<String, VariableInfo<'ctx>>,
    loop_exit_stack: &mut Vec<BasicBlock<'ctx>>,
    loop_continue_stack: &mut Vec<BasicBlock<'ctx>>,
    current_function: FunctionValue<'ctx>,
    global_consts: &HashMap<String, BasicValueEnum<'ctx>>,
    struct_types: &HashMap<String, StructType<'ctx>>,
    struct_field_indices: &HashMap<String, HashMap<String, u32>>,
    struct_field_types: &HashMap<String, HashMap<String, WaveType>>,
    target_data: &'ctx TargetData,
    extern_c_info: &HashMap<String, ExternCInfo<'ctx>>,
) {
    let current_fn = builder.get_insert_block().unwrap().get_parent().unwrap();

    let discr_any = generate_expression_ir(
        context,
        builder,
        value,
        variables,
        module,
        None,
        global_consts,
        struct_types,
        struct_field_indices,
        target_data,
        extern_c_info,
    );

    let discr = match discr_any {
        BasicValueEnum::IntValue(iv) => iv,
        other => panic!(
            "match value must be integer/enum type, got {:?}",
            other.get_type()
        ),
    };
    let discr_ty = discr.get_type();

    let merge_block = context.append_basic_block(current_fn, "match.end");

    let mut default_arm: Option<&MatchArm> = None;
    let mut case_entries: Vec<(inkwell::values::IntValue<'ctx>, BasicBlock<'ctx>, &MatchArm)> =
        Vec::new();
    let mut seen_case_values: HashSet<String> = HashSet::new();

    for (idx, arm) in arms.iter().enumerate() {
        match &arm.pattern {
            MatchPattern::Wildcard => {
                if default_arm.is_some() {
                    panic!("duplicate wildcard match arm (`_`)");
                }
                default_arm = Some(arm);
            }
            pat @ (MatchPattern::Int(_) | MatchPattern::Ident(_)) => {
                let case_value = eval_match_case_const(discr_ty, pat, global_consts);
                let case_key = case_value.print_to_string().to_string();
                if !seen_case_values.insert(case_key.clone()) {
                    panic!("duplicate match case value: {}", case_key);
                }

                let case_block = context.append_basic_block(current_fn, &format!("match.case.{}", idx));
                case_entries.push((case_value, case_block, arm));
            }
        }
    }

    let default_block = if default_arm.is_some() {
        context.append_basic_block(current_fn, "match.default")
    } else {
        merge_block
    };

    let switch_cases: Vec<(inkwell::values::IntValue<'ctx>, BasicBlock<'ctx>)> = case_entries
        .iter()
        .map(|(v, bb, _)| (*v, *bb))
        .collect();

    builder
        .build_switch(discr, default_block, &switch_cases)
        .unwrap();

    for (_, case_block, arm) in case_entries {
        builder.position_at_end(case_block);
        for stmt in &arm.body {
            super::generate_statement_ir(
                context,
                builder,
                module,
                string_counter,
                stmt,
                variables,
                loop_exit_stack,
                loop_continue_stack,
                current_function,
                global_consts,
                struct_types,
                struct_field_indices,
                struct_field_types,
                target_data,
                extern_c_info,
            );
        }

        let end_bb = builder.get_insert_block().unwrap();
        if end_bb.get_terminator().is_none() {
            builder.build_unconditional_branch(merge_block).unwrap();
        }
    }

    if let Some(default_arm) = default_arm {
        builder.position_at_end(default_block);
        for stmt in &default_arm.body {
            super::generate_statement_ir(
                context,
                builder,
                module,
                string_counter,
                stmt,
                variables,
                loop_exit_stack,
                loop_continue_stack,
                current_function,
                global_consts,
                struct_types,
                struct_field_indices,
                struct_field_types,
                target_data,
                extern_c_info,
            );
        }

        let end_bb = builder.get_insert_block().unwrap();
        if end_bb.get_terminator().is_none() {
            builder.build_unconditional_branch(merge_block).unwrap();
        }
    }

    builder.position_at_end(merge_block);
}

pub(super) fn gen_for_ir<'ctx>(
    context: &'ctx inkwell::context::Context,
    builder: &'ctx inkwell::builder::Builder<'ctx>,
    module: &'ctx Module<'ctx>,
    string_counter: &mut usize,
    initialization: &ASTNode,
    condition: &Expression,
    increment: &Expression,
    body: &[ASTNode],
    variables: &mut HashMap<String, VariableInfo<'ctx>>,
    loop_exit_stack: &mut Vec<BasicBlock<'ctx>>,
    loop_continue_stack: &mut Vec<BasicBlock<'ctx>>,
    current_function: FunctionValue<'ctx>,
    global_consts: &HashMap<String, BasicValueEnum<'ctx>>,
    struct_types: &HashMap<String, StructType<'ctx>>,
    struct_field_indices: &HashMap<String, HashMap<String, u32>>,
    struct_field_types: &HashMap<String, HashMap<String, WaveType>>,
    target_data: &'ctx TargetData,
    extern_c_info: &HashMap<String, ExternCInfo<'ctx>>,
) {
    let current_fn = builder.get_insert_block().unwrap().get_parent().unwrap();
    let outer_scope_variables = variables.clone();

    super::generate_statement_ir(
        context,
        builder,
        module,
        string_counter,
        initialization,
        variables,
        loop_exit_stack,
        loop_continue_stack,
        current_function,
        global_consts,
        struct_types,
        struct_field_indices,
        struct_field_types,
        target_data,
        extern_c_info,
    );

    let cond_block = context.append_basic_block(current_fn, "for.cond");
    let body_block = context.append_basic_block(current_fn, "for.body");
    let inc_block = context.append_basic_block(current_fn, "for.inc");
    let merge_block = context.append_basic_block(current_fn, "for.end");

    loop_exit_stack.push(merge_block);
    loop_continue_stack.push(inc_block);

    builder.build_unconditional_branch(cond_block).unwrap();
    builder.position_at_end(cond_block);

    let cond_val = generate_expression_ir(
        context,
        builder,
        condition,
        variables,
        module,
        None,
        global_consts,
        struct_types,
        struct_field_indices,
        target_data,
        extern_c_info,
    );

    let cond_bool = truthy_to_i1(context, builder, cond_val, "for_cond");
    builder
        .build_conditional_branch(cond_bool, body_block, merge_block)
        .unwrap();

    builder.position_at_end(body_block);
    for stmt in body {
        super::generate_statement_ir(
            context,
            builder,
            module,
            string_counter,
            stmt,
            variables,
            loop_exit_stack,
            loop_continue_stack,
            current_function,
            global_consts,
            struct_types,
            struct_field_indices,
            struct_field_types,
            target_data,
            extern_c_info,
        );
    }

    let body_end = builder.get_insert_block().unwrap();
    if body_end.get_terminator().is_none() {
        builder.build_unconditional_branch(inc_block).unwrap();
    }

    builder.position_at_end(inc_block);
    let _ = generate_expression_ir(
        context,
        builder,
        increment,
        variables,
        module,
        None,
        global_consts,
        struct_types,
        struct_field_indices,
        target_data,
        extern_c_info,
    );

    let inc_end = builder.get_insert_block().unwrap();
    if inc_end.get_terminator().is_none() {
        builder.build_unconditional_branch(cond_block).unwrap();
    }

    loop_exit_stack.pop();
    loop_continue_stack.pop();

    builder.position_at_end(merge_block);
    *variables = outer_scope_variables;
}

pub(super) fn gen_break_ir<'ctx>(
    builder: &'ctx inkwell::builder::Builder<'ctx>,
    loop_exit_stack: &mut Vec<BasicBlock<'ctx>>,
) {
    if let Some(target_block) = loop_exit_stack.last() {
        builder.build_unconditional_branch(*target_block).unwrap();
    } else {
        panic!("break used outside of loop!");
    }
}

pub(super) fn gen_continue_ir<'ctx>(
    builder: &'ctx inkwell::builder::Builder<'ctx>,
    loop_continue_stack: &mut Vec<BasicBlock<'ctx>>,
) {
    if let Some(target_block) = loop_continue_stack.last() {
        builder.build_unconditional_branch(*target_block).unwrap();
    } else {
        panic!("continue used outside of loop!");
    }
}

pub(super) fn gen_return_ir<'ctx>(
    context: &'ctx inkwell::context::Context,
    builder: &'ctx inkwell::builder::Builder<'ctx>,
    module: &'ctx Module<'ctx>,
    expr_opt: Option<&Expression>,
    variables: &mut HashMap<String, VariableInfo<'ctx>>,
    current_function: FunctionValue<'ctx>,
    global_consts: &HashMap<String, BasicValueEnum<'ctx>>,
    struct_types: &HashMap<String, StructType<'ctx>>,
    struct_field_indices: &HashMap<String, HashMap<String, u32>>,
    target_data: &'ctx TargetData,
    extern_c_info: &HashMap<String, ExternCInfo<'ctx>>,
) {
    let expected_ret = current_function.get_type().get_return_type(); // Option<BasicTypeEnum>

    match (expected_ret, expr_opt) {
        (None, None) => {
            builder.build_return(None).unwrap();
        }

        (None, Some(_)) => {
            panic!("Void function cannot return a value");
        }

        (Some(ret_ty), None) => {
            let is_i32_main = current_function.get_name().to_str().ok() == Some("main")
                && matches!(ret_ty, inkwell::types::BasicTypeEnum::IntType(it) if it.get_bit_width() == 32);

            if is_i32_main {
                let zero = context.i32_type().const_zero();
                builder.build_return(Some(&zero)).unwrap();
            } else {
                panic!("Non-void function must return a value");
            }
        }

        (Some(ret_ty), Some(expr)) => {
            let mut v = generate_expression_ir(
                context,
                builder,
                expr,
                variables,
                module,
                Some(ret_ty),
                global_consts,
                struct_types,
                struct_field_indices,
                target_data,
                extern_c_info,
            );

            if v.get_type() != ret_ty {
                v = coerce_basic_value(
                    context,
                    builder,
                    v,
                    ret_ty,
                    "ret_cast",
                    CoercionMode::Explicit,
                );
            }

            builder.build_return(Some(&v)).unwrap();
        }
    }
}
