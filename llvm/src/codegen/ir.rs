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

use inkwell::context::Context;
use inkwell::passes::PassBuilderOptions;
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum};
use inkwell::values::{BasicValue, BasicValueEnum, FunctionValue};
use inkwell::OptimizationLevel;

use inkwell::targets::{
    CodeModel, InitializationConfig, RelocMode, Target, TargetData, TargetMachine,
};
use parser::ast::{
    ASTNode, EnumNode, ExternFunctionNode, FunctionNode, Mutability, ParameterNode, ProtoImplNode,
    StructNode, TypeAliasNode, VariableNode, WaveType,
};
use std::collections::{HashMap, HashSet};

use crate::codegen::target::require_supported_target_from_triple;
use crate::statement::generate_statement_ir;

use super::consts::{create_llvm_const_value, ConstEvalError};
use super::types::{wave_type_to_llvm_type, TypeFlavor, VariableInfo};

use crate::codegen::abi_c::{apply_extern_c_attrs, lower_extern_c, ExternCInfo};

fn is_implicit_i32_main(name: &str, return_type: &Option<WaveType>) -> bool {
    name == "main" && matches!(return_type, None | Some(WaveType::Void))
}

fn is_supported_extern_abi(abi: &str) -> bool {
    abi.eq_ignore_ascii_case("c")
}

fn normalize_opt_flag_for_passes(opt_flag: &str) -> &str {
    match opt_flag {
        // Keep consistent with backend clang optimization mapping.
        "-Ofast" => "-O3",
        other => other,
    }
}

pub unsafe fn generate_ir(ast_nodes: &[ASTNode], opt_flag: &str) -> String {
    let context: &'static Context = Box::leak(Box::new(Context::create()));
    let module: &'static _ = Box::leak(Box::new(context.create_module("main")));
    let builder: &'static _ = Box::leak(Box::new(context.create_builder()));

    let named_types = collect_named_types(ast_nodes);
    let ast_nodes: Vec<ASTNode> = ast_nodes
        .iter()
        .map(|n| resolve_ast_node(n, &named_types))
        .collect();

    Target::initialize_native(&InitializationConfig::default()).unwrap();
    let triple = TargetMachine::get_default_triple();
    let abi_target = require_supported_target_from_triple(&triple);
    let target = Target::from_triple(&triple).unwrap();

    let tm = target
        .create_target_machine(
            &triple,
            "generic",
            "",
            OptimizationLevel::Default,
            RelocMode::Default,
            CodeModel::Default,
        )
        .unwrap();

    module.set_triple(&triple);

    let td_val: TargetData = tm.get_target_data();
    module.set_data_layout(&td_val.get_data_layout());
    let td: &'static TargetData = Box::leak(Box::new(td_val));

    let mut extern_c_info: HashMap<String, ExternCInfo<'static>> = HashMap::new();

    let mut global_consts: HashMap<String, BasicValueEnum<'static>> = HashMap::new();
    let mut global_statics: HashMap<String, VariableInfo<'static>> = HashMap::new();

    let mut struct_types: HashMap<String, inkwell::types::StructType> = HashMap::new();
    let mut struct_field_indices: HashMap<String, HashMap<String, u32>> = HashMap::new();
    let mut struct_field_types: HashMap<String, HashMap<String, WaveType>> = HashMap::new();
    // (1) struct opaque + field index map
    for ast in &ast_nodes {
        if let ASTNode::Struct(struct_node) = ast {
            let st = context.opaque_struct_type(&struct_node.name);
            struct_types.insert(struct_node.name.clone(), st);

            let mut index_map = HashMap::new();
            let mut type_map = HashMap::new();
            for (i, (field_name, field_ty)) in struct_node.fields.iter().enumerate() {
                index_map.insert(field_name.clone(), i as u32);
                type_map.insert(field_name.clone(), field_ty.clone());
            }
            struct_field_indices.insert(struct_node.name.clone(), index_map);
            struct_field_types.insert(struct_node.name.clone(), type_map);
        }
    }

    for ast in &ast_nodes {
        if let ASTNode::Struct(struct_node) = ast {
            let st = *struct_types
                .get(&struct_node.name)
                .unwrap_or_else(|| panic!("Opaque struct missing: {}", struct_node.name));

            let field_types: Vec<BasicTypeEnum> = struct_node
                .fields
                .iter()
                .map(|(_, ty)| wave_type_to_llvm_type(context, ty, &struct_types, TypeFlavor::AbiC))
                .collect();

            st.set_body(&field_types, false);
        }
    }

    for ast in &ast_nodes {
        if let ASTNode::Enum(e) = ast {
            add_enum_consts_to_globals(context, e, &mut global_consts);
        }
    }

    let mut pending: Vec<&VariableNode> = ast_nodes
        .iter()
        .filter_map(|ast| match ast {
            ASTNode::Variable(v) if v.mutability == Mutability::Const => Some(v),
            _ => None,
        })
        .collect();

    let mut round = 0;
    while !pending.is_empty() {
        round += 1;

        let mut progressed = false;
        let mut next_pending: Vec<&VariableNode> = Vec::new();

        for v in pending {
            let init = v
                .initial_value
                .as_ref()
                .unwrap_or_else(|| panic!("Constant must be initialized: {}", v.name));

            match create_llvm_const_value(
                context,
                &v.type_name,
                init,
                &struct_types,
                &struct_field_indices,
                &global_consts,
            ) {
                Ok(val) => {
                    global_consts.insert(v.name.clone(), val);
                    progressed = true;
                }
                Err(ConstEvalError::UnknownIdentifier(_)) => {
                    next_pending.push(v);
                }
                Err(e) => {
                    panic!("const '{}' evaluation failed: {}", v.name, e);
                }
            }
        }

        if next_pending.is_empty() {
            break;
        }
        if !progressed {
            let names: Vec<String> = next_pending.iter().map(|v| v.name.clone()).collect();
            panic!(
                "unresolved const cycle or missing symbols after {} rounds: {:?}",
                round, names
            );
        }

        pending = next_pending;
    }

    for ast in &ast_nodes {
        let ASTNode::Variable(v) = ast else {
            continue;
        };
        if v.mutability != Mutability::Static {
            continue;
        }

        let llvm_ty = wave_type_to_llvm_type(context, &v.type_name, &struct_types, TypeFlavor::AbiC);
        let g = module.add_global(llvm_ty, None, &v.name);

        let init = if let Some(expr) = &v.initial_value {
            create_llvm_const_value(
                context,
                &v.type_name,
                expr,
                &struct_types,
                &struct_field_indices,
                &global_consts,
            )
            .unwrap_or_else(|e| panic!("static '{}' initialization failed: {}", v.name, e))
        } else {
            llvm_ty.const_zero().as_basic_value_enum()
        };

        g.set_initializer(&init);
        g.set_constant(false);

        global_statics.insert(
            v.name.clone(),
            VariableInfo {
                ptr: g.as_pointer_value(),
                mutability: Mutability::Static,
                ty: v.type_name.clone(),
            },
        );
    }

    let mut proto_functions: Vec<(String, FunctionNode)> = Vec::new();
    for ast in &ast_nodes {
        if let ASTNode::ProtoImpl(proto_impl) = ast {
            for method in &proto_impl.methods {
                let new_name = format!("{}_{}", proto_impl.target, method.name);
                let mut new_fn = method.clone();
                new_fn.name = new_name.clone();
                proto_functions.push((new_name, new_fn));
            }
        }
    }

    let mut functions: HashMap<String, FunctionValue> = HashMap::new();

    let function_nodes: Vec<FunctionNode> = ast_nodes
        .iter()
        .filter_map(|ast| {
            if let ASTNode::Function(f) = ast {
                Some(f.clone())
            } else {
                None
            }
        })
        .chain(proto_functions.iter().map(|(_, f)| f.clone()))
        .collect();

    let extern_functions: Vec<&ExternFunctionNode> = ast_nodes
        .iter()
        .filter_map(|ast| {
            if let ASTNode::ExternFunction(ext) = ast {
                Some(ext)
            } else {
                None
            }
        })
        .collect();

    for FunctionNode {
        name,
        parameters,
        return_type,
        ..
    } in &function_nodes
    {
        let param_types: Vec<BasicMetadataTypeEnum> = parameters
            .iter()
            .map(|p| {
                wave_type_to_llvm_type(context, &p.param_type, &struct_types, TypeFlavor::AbiC)
                    .into()
            })
            .collect();

        let fn_type = if is_implicit_i32_main(name, return_type) {
            context.i32_type().fn_type(&param_types, false)
        } else {
            match return_type {
                None | Some(WaveType::Void) => context.void_type().fn_type(&param_types, false),
                Some(wave_ret_ty) => {
                    let llvm_ret_type = wave_type_to_llvm_type(
                        context,
                        wave_ret_ty,
                        &struct_types,
                        TypeFlavor::AbiC,
                    );
                    llvm_ret_type.fn_type(&param_types, false)
                }
            }
        };

        let function = module.add_function(name, fn_type, None);
        functions.insert(name.clone(), function);
    }

    for ext in &extern_functions {
        if !is_supported_extern_abi(&ext.abi) {
            panic!(
                "unsupported extern ABI '{}' for function '{}': only extern(c) is currently supported",
                ext.abi, ext.name
            );
        }

        let lowered = lower_extern_c(context, td, abi_target, ext, &struct_types);

        let f = module.add_function(&lowered.llvm_name, lowered.fn_type, None);
        apply_extern_c_attrs(context, f, &lowered.info);

        functions.insert(ext.name.clone(), f);

        extern_c_info.insert(ext.name.clone(), lowered.info);
    }

    for func_node in &function_nodes {
        let function = *functions.get(&func_node.name).unwrap();
        let entry_block = context.append_basic_block(function, "entry");
        builder.position_at_end(entry_block);

        let mut variables: HashMap<String, VariableInfo> = global_statics.clone();
        let mut string_counter = 0;
        let mut loop_exit_stack = vec![];
        let mut loop_continue_stack = vec![];

        for (i, param) in func_node.parameters.iter().enumerate() {
            let llvm_type =
                wave_type_to_llvm_type(context, &param.param_type, &struct_types, TypeFlavor::AbiC);
            let alloca = builder.build_alloca(llvm_type, &param.name).unwrap();
            let param_val = function.get_nth_param(i as u32).unwrap();
            builder.build_store(alloca, param_val).unwrap();

            variables.insert(
                param.name.clone(),
                VariableInfo {
                    ptr: alloca,
                    mutability: Mutability::Let,
                    ty: param.param_type.clone(),
                },
            );
        }

        for stmt in &func_node.body {
            if let ASTNode::Statement(_) | ASTNode::Variable(_) = stmt {
                generate_statement_ir(
                    context,
                    builder,
                    module,
                    &mut string_counter,
                    stmt,
                    &mut variables,
                    &mut loop_exit_stack,
                    &mut loop_continue_stack,
                    function,
                    &global_consts,
                    &struct_types,
                    &struct_field_indices,
                    &struct_field_types,
                    td,
                    &extern_c_info,
                );
            } else {
                panic!("Unsupported node inside function '{}'", func_node.name);
            }
        }

        let current_block = builder.get_insert_block().unwrap();
        if current_block.get_terminator().is_none() {
            let implicit_i32_main = is_implicit_i32_main(&func_node.name, &func_node.return_type);
            let is_void_like = match &func_node.return_type {
                None => true,
                Some(WaveType::Void) => true,
                _ => false,
            };

            if implicit_i32_main {
                let zero = context.i32_type().const_zero();
                builder.build_return(Some(&zero)).unwrap();
            } else if is_void_like {
                builder.build_return(None).unwrap();
            } else {
                panic!(
                    "Non-void function '{}' is missing a return statement",
                    func_node.name
                );
            }
        }
    }

    let pbo = PassBuilderOptions::create();
    let pipeline = pipeline_from_opt_flag(opt_flag);

    module
        .run_passes(pipeline, &tm, pbo)
        .expect("failed to run optimization passes");

    module.print_to_string().to_string()
}

fn pipeline_from_opt_flag(opt_flag: &str) -> &'static str {
    match normalize_opt_flag_for_passes(opt_flag) {
        "" | "-O0" => "default<O0>",
        "-O1" => "default<O1>",
        "-O2" => "default<O2>",
        "-O3" => "default<O3>",
        "-Os" => "default<Os>",
        "-Oz" => "default<Oz>",
        other => panic!("unknown opt flag for LLVM passes: {}", other),
    }
}

fn parse_int_literal(raw: &str) -> Option<i128> {
    let mut s = raw.trim().replace('_', "");
    if s.is_empty() {
        return None;
    }

    let neg = if let Some(rest) = s.strip_prefix('-') {
        s = rest.to_string();
        true
    } else if let Some(rest) = s.strip_prefix('+') {
        s = rest.to_string();
        false
    } else {
        false
    };

    let (radix, digits) = if let Some(rest) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X"))
    {
        (16, rest)
    } else if let Some(rest) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        (2, rest)
    } else if let Some(rest) = s.strip_prefix("0o").or_else(|| s.strip_prefix("0O")) {
        (8, rest)
    } else {
        (10, s.as_str())
    };

    let v = i128::from_str_radix(digits, radix).ok()?;
    Some(if neg { -v } else { v })
}

fn repr_bits_signed(ty: &WaveType) -> Option<(u32, bool)> {
    match ty {
        WaveType::Int(b) => Some((*b as u32, true)),
        WaveType::Uint(b) => Some((*b as u32, false)),
        WaveType::Bool => Some((1, false)),
        WaveType::Byte => Some((8, false)),
        WaveType::Char => Some((8, false)),
        _ => None,
    }
}

fn fits_in_int(v: i128, bits: u32, signed: bool) -> bool {
    if bits == 0 || bits > 64 {
        return false;
    }

    if signed {
        if bits == 64 {
            return v >= i64::MIN as i128 && v <= i64::MAX as i128;
        }
        let min = -(1i128 << (bits - 1));
        let max = (1i128 << (bits - 1)) - 1;
        v >= min && v <= max
    } else {
        if v < 0 {
            return false;
        }
        if bits == 64 {
            return (v as u128) <= u64::MAX as u128;
        }
        let max = (1u128 << bits) - 1;
        (v as u128) <= max
    }
}

fn collect_named_types(nodes: &[ASTNode]) -> HashMap<String, WaveType> {
    let mut m = HashMap::new();
    for n in nodes {
        match n {
            ASTNode::TypeAlias(TypeAliasNode { name, target }) => {
                m.insert(name.clone(), target.clone());
            }
            ASTNode::Enum(EnumNode {
                name, repr_type, ..
            }) => {
                m.insert(name.clone(), repr_type.clone());
            }
            _ => {}
        }
    }
    m
}

fn resolve_wave_type_impl(
    ty: &WaveType,
    named: &HashMap<String, WaveType>,
    visiting: &mut HashSet<String>,
) -> WaveType {
    match ty {
        WaveType::Pointer(inner) => {
            WaveType::Pointer(Box::new(resolve_wave_type_impl(inner, named, visiting)))
        }
        WaveType::Array(inner, n) => {
            WaveType::Array(Box::new(resolve_wave_type_impl(inner, named, visiting)), *n)
        }
        WaveType::Struct(name) => {
            if let Some(t) = named.get(name) {
                if !visiting.insert(name.clone()) {
                    panic!("Type alias/enum cycle detected at '{}'", name);
                }
                let out = resolve_wave_type_impl(t, named, visiting);
                visiting.remove(name);
                out
            } else {
                WaveType::Struct(name.clone())
            }
        }
        _ => ty.clone(),
    }
}

fn resolve_wave_type(ty: &WaveType, named: &HashMap<String, WaveType>) -> WaveType {
    let mut visiting = HashSet::new();
    resolve_wave_type_impl(ty, named, &mut visiting)
}

fn resolve_parameter(p: &ParameterNode, named: &HashMap<String, WaveType>) -> ParameterNode {
    let mut out = p.clone();
    out.param_type = resolve_wave_type(&out.param_type, named);
    out
}

fn resolve_function(f: &FunctionNode, named: &HashMap<String, WaveType>) -> FunctionNode {
    let mut out = f.clone();
    out.parameters = out
        .parameters
        .iter()
        .map(|p| resolve_parameter(p, named))
        .collect();
    out.return_type = out
        .return_type
        .as_ref()
        .map(|t| resolve_wave_type(t, named));
    out.body = out
        .body
        .iter()
        .map(|n| resolve_ast_node(n, named))
        .collect();
    out
}

fn resolve_struct(s: &StructNode, named: &HashMap<String, WaveType>) -> StructNode {
    let mut out = s.clone();
    out.fields = out
        .fields
        .iter()
        .map(|(n, t)| (n.clone(), resolve_wave_type(t, named)))
        .collect();
    out.methods = out
        .methods
        .iter()
        .map(|m| resolve_function(m, named))
        .collect();
    out
}

fn resolve_proto(p: &ProtoImplNode, named: &HashMap<String, WaveType>) -> ProtoImplNode {
    let mut out = p.clone();
    out.methods = out
        .methods
        .iter()
        .map(|m| resolve_function(m, named))
        .collect();
    out
}

fn resolve_extern(e: &ExternFunctionNode, named: &HashMap<String, WaveType>) -> ExternFunctionNode {
    let mut out = e.clone();
    out.params = out
        .params
        .iter()
        .map(|(n, t)| (n.clone(), resolve_wave_type(t, named)))
        .collect();
    out.return_type = resolve_wave_type(&out.return_type, named);
    out
}

fn resolve_variable(v: &VariableNode, named: &HashMap<String, WaveType>) -> VariableNode {
    let mut out = v.clone();
    out.type_name = resolve_wave_type(&out.type_name, named);
    out
}

fn resolve_enum(e: &EnumNode, named: &HashMap<String, WaveType>) -> EnumNode {
    let mut out = e.clone();
    out.repr_type = resolve_wave_type(&out.repr_type, named);
    out
}

fn resolve_ast_node(n: &ASTNode, named: &HashMap<String, WaveType>) -> ASTNode {
    match n {
        ASTNode::Enum(e) => ASTNode::Enum(resolve_enum(e, named)),
        ASTNode::Function(f) => ASTNode::Function(resolve_function(f, named)),
        ASTNode::ExternFunction(e) => ASTNode::ExternFunction(resolve_extern(e, named)),
        ASTNode::Struct(s) => ASTNode::Struct(resolve_struct(s, named)),
        ASTNode::ProtoImpl(p) => ASTNode::ProtoImpl(resolve_proto(p, named)),
        ASTNode::Variable(v) => ASTNode::Variable(resolve_variable(v, named)),

        ASTNode::TypeAlias(_) | ASTNode::Enum(_) => n.clone(),

        _ => n.clone(),
    }
}

fn add_enum_consts_to_globals(
    context: &'static Context,
    e: &EnumNode,
    global_consts: &mut HashMap<String, BasicValueEnum<'static>>,
) {
    let (bits, signed) = repr_bits_signed(&e.repr_type).unwrap_or_else(|| {
        panic!(
            "enum '{}' repr type must be an integer type, got {:?}",
            e.name, e.repr_type
        )
    });

    if bits > 64 || bits == 0 {
        panic!("enum '{}' repr bit-width unsupported: {}", e.name, bits);
    }

    let int_ty = context.custom_width_int_type(bits);

    let mut next: i128 = 0;

    for v in &e.variants {
        if let Some(raw) = &v.explicit_value {
            next = parse_int_literal(raw).unwrap_or_else(|| {
                panic!(
                    "enum '{}' variant '{}' has invalid integer literal: {}",
                    e.name, v.name, raw
                )
            });
        }

        if !fits_in_int(next, bits, signed) {
            panic!(
                "enum '{}' variant '{}' value {} does not fit in {}{}",
                e.name,
                v.name,
                next,
                if signed { "i" } else { "u" },
                bits
            );
        }

        let c = if signed {
            int_ty.const_int(next as u64, true)
        } else {
            int_ty.const_int(next as u64, false)
        };

        global_consts.insert(v.name.clone(), c.into());

        next += 1;
    }
}
