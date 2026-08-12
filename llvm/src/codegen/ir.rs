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

use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::context::Context;
use inkwell::module::{FlagBehavior, Linkage, Module};
use inkwell::passes::PassBuilderOptions;
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum};
use inkwell::values::{
    BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue, PointerValue, ValueKind,
};
use inkwell::OptimizationLevel;

use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetData, TargetMachine,
    TargetMachineOptions, TargetTriple,
};
use parser::ast::{
    ASTNode, EnumNode, ExternFunctionNode, FunctionNode, Mutability, ParameterNode, ProtoImplNode,
    StructNode, TypeAliasNode, VariableNode, WaveType,
};
use std::collections::{HashMap, HashSet};
use std::sync::Once;

use crate::backend::BackendOptions;
use crate::codegen::target::{require_supported_target_from_triple, CodegenTarget};
use crate::statement::generate_statement_ir;

use super::consts::{create_llvm_const_value, ConstEvalError};
use super::types::{wave_type_to_llvm_type, TypeFlavor, VariableInfo};

use crate::codegen::abi_c::{
    apply_extern_c_attrs, lower_extern_c, ExternCInfo, ParamLowering, RetLowering,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodegenFileKind {
    Bitcode,
    Assembly,
    Object,
}

struct GeneratedModule {
    module: &'static Module<'static>,
    target_machine: TargetMachine,
}

struct ExportCWrapper<'ctx> {
    wrapper: FunctionValue<'ctx>,
    implementation: FunctionValue<'ctx>,
    info: ExternCInfo<'ctx>,
    wave_param_types: Vec<BasicTypeEnum<'ctx>>,
    wave_ret_type: Option<BasicTypeEnum<'ctx>>,
}

fn reinterpret_abi_value<'ctx>(
    context: &'ctx Context,
    builder: &inkwell::builder::Builder<'ctx>,
    td: &TargetData,
    value: BasicValueEnum<'ctx>,
    target: BasicTypeEnum<'ctx>,
    tag: &str,
) -> BasicValueEnum<'ctx> {
    if value.get_type() == target {
        return value;
    }

    let source = value.get_type();
    let source_size = td.get_store_size(&source);
    let target_size = td.get_store_size(&target);
    let source_is_aggregate = matches!(
        source,
        BasicTypeEnum::ArrayType(_) | BasicTypeEnum::StructType(_)
    );
    let target_is_aggregate = matches!(
        target,
        BasicTypeEnum::ArrayType(_) | BasicTypeEnum::StructType(_)
    );
    if source_size != target_size && !(source_is_aggregate || target_is_aggregate) {
        panic!(
            "cannot reinterpret C ABI value '{}' from {} bytes to {} bytes",
            tag, source_size, target_size
        );
    }

    let source_ptr = builder
        .build_alloca(source, &format!("{}_source", tag))
        .unwrap();
    builder.build_store(source_ptr, value).unwrap();
    let target_ptr = builder
        .build_alloca(target, &format!("{}_target", tag))
        .unwrap();
    builder
        .build_store(target_ptr, target.const_zero())
        .unwrap();
    let size = context
        .i64_type()
        .const_int(source_size.min(target_size), false);
    builder
        .build_memcpy(
            target_ptr,
            td.get_abi_alignment(&target),
            source_ptr,
            td.get_abi_alignment(&source),
            size,
        )
        .unwrap();
    builder
        .build_load(target, target_ptr, &format!("{}_load", tag))
        .unwrap()
        .as_basic_value_enum()
}

fn rebuild_split_abi_value<'ctx>(
    context: &'ctx Context,
    builder: &inkwell::builder::Builder<'ctx>,
    td: &TargetData,
    parts: &[BasicValueEnum<'ctx>],
    target: BasicTypeEnum<'ctx>,
    tag: &str,
) -> BasicValueEnum<'ctx> {
    let target_ptr = builder
        .build_alloca(target, &format!("{}_target", tag))
        .unwrap();
    let target_size = td.get_store_size(&target);
    builder
        .build_store(target_ptr, target.const_zero())
        .unwrap();
    let mut offset = 0u64;

    for (index, part) in parts.iter().enumerate() {
        let part_type = part.get_type();
        let part_size = td.get_store_size(&part_type);
        let part_ptr = builder
            .build_alloca(part_type, &format!("{}_part_{}", tag, index))
            .unwrap();
        builder.build_store(part_ptr, *part).unwrap();
        let offset_value = context.i64_type().const_int(offset, false);
        let destination = unsafe {
            builder
                .build_gep(
                    context.i8_type(),
                    target_ptr,
                    &[offset_value],
                    &format!("{}_offset_{}", tag, index),
                )
                .unwrap()
        };
        let copy_size = part_size.min(target_size.saturating_sub(offset));
        if copy_size > 0 {
            builder
                .build_memcpy(
                    destination,
                    1,
                    part_ptr,
                    td.get_abi_alignment(&part_type),
                    context.i64_type().const_int(copy_size, false),
                )
                .unwrap();
        }
        offset += part_size;
    }
    builder
        .build_load(target, target_ptr, &format!("{}_load", tag))
        .unwrap()
        .as_basic_value_enum()
}

fn build_export_c_wrapper<'ctx>(
    context: &'ctx Context,
    builder: &inkwell::builder::Builder<'ctx>,
    td: &TargetData,
    export: &ExportCWrapper<'ctx>,
) {
    let entry = context.append_basic_block(export.wrapper, "entry");
    builder.position_at_end(entry);

    let mut llvm_index = 0u32;
    let sret_ptr: Option<PointerValue<'ctx>> =
        if matches!(export.info.ret, RetLowering::SRet { .. }) {
            let pointer = export
                .wrapper
                .get_nth_param(0)
                .expect("C ABI sret wrapper requires a hidden pointer")
                .into_pointer_value();
            llvm_index += 1;
            Some(pointer)
        } else {
            None
        };

    let mut implementation_args = Vec::<BasicMetadataValueEnum<'ctx>>::new();
    for (wave_index, (lowering, wave_type)) in export
        .info
        .params
        .iter()
        .zip(export.wave_param_types.iter())
        .enumerate()
    {
        let value = match lowering {
            ParamLowering::Ignore => wave_type.const_zero(),
            ParamLowering::Direct(_) => {
                let incoming = export
                    .wrapper
                    .get_nth_param(llvm_index)
                    .expect("missing direct C ABI wrapper argument");
                llvm_index += 1;
                reinterpret_abi_value(
                    context,
                    builder,
                    td,
                    incoming,
                    *wave_type,
                    &format!("export_arg_{}", wave_index),
                )
            }
            ParamLowering::Indirect { .. } | ParamLowering::ByVal { .. } => {
                let pointer = export
                    .wrapper
                    .get_nth_param(llvm_index)
                    .expect("missing indirect C ABI wrapper argument")
                    .into_pointer_value();
                llvm_index += 1;
                builder
                    .build_load(*wave_type, pointer, &format!("export_byval_{}", wave_index))
                    .unwrap()
                    .as_basic_value_enum()
            }
            ParamLowering::Split(parts) => {
                let mut incoming = Vec::with_capacity(parts.len());
                for _ in parts {
                    incoming.push(
                        export
                            .wrapper
                            .get_nth_param(llvm_index)
                            .expect("missing split C ABI wrapper argument"),
                    );
                    llvm_index += 1;
                }
                rebuild_split_abi_value(
                    context,
                    builder,
                    td,
                    &incoming,
                    *wave_type,
                    &format!("export_split_{}", wave_index),
                )
            }
        };
        implementation_args.push(value.into());
    }

    let call = builder
        .build_call(
            export.implementation,
            &implementation_args,
            "export_implementation",
        )
        .unwrap();

    match &export.info.ret {
        RetLowering::Void => {
            builder.build_return(None).unwrap();
        }
        RetLowering::SRet { .. } => {
            let value = match call.try_as_basic_value() {
                ValueKind::Basic(value) => value,
                ValueKind::Instruction(_) => {
                    panic!("C ABI sret wrapper implementation returned void")
                }
            };
            builder
                .build_store(sret_ptr.expect("missing C ABI sret pointer"), value)
                .unwrap();
            builder.build_return(None).unwrap();
        }
        RetLowering::Direct(lowered_type) => {
            let value = match call.try_as_basic_value() {
                ValueKind::Basic(value) => value,
                ValueKind::Instruction(_) => {
                    panic!("C ABI direct wrapper implementation returned void")
                }
            };
            let wave_type = export
                .wave_ret_type
                .expect("direct C ABI wrapper requires a Wave return type");
            if value.get_type() != wave_type {
                panic!("C ABI wrapper implementation return type changed unexpectedly");
            }
            let lowered =
                reinterpret_abi_value(context, builder, td, value, *lowered_type, "export_return");
            builder.build_return(Some(&lowered)).unwrap();
        }
    }
}

fn is_implicit_i32_main(name: &str, return_type: &Option<WaveType>) -> bool {
    name == "main" && matches!(return_type, None | Some(WaveType::Void))
}

fn is_supported_extern_abi(abi: &str) -> bool {
    abi.eq_ignore_ascii_case("c")
}

fn normalize_opt_flag_for_passes(opt_flag: &str) -> &str {
    match opt_flag {
        // LLVM's pass pipeline has no dedicated Ofast preset; keep it aligned with codegen tools.
        "-Ofast" => "-O3",
        other => other,
    }
}

fn target_opt_level_from_flag(opt_flag: &str) -> OptimizationLevel {
    match normalize_opt_flag_for_passes(opt_flag) {
        "" | "-O0" => OptimizationLevel::None,
        "-O1" => OptimizationLevel::Less,
        "-O2" | "-Os" | "-Oz" => OptimizationLevel::Default,
        "-O3" => OptimizationLevel::Aggressive,
        other => panic!("unknown opt flag for target machine: {}", other),
    }
}

fn code_model_from_backend(backend: &BackendOptions, target: CodegenTarget) -> CodeModel {
    if let Some(model) = backend.code_model.as_deref() {
        return match model {
            "default" => CodeModel::Default,
            "jitdefault" | "jit-default" => CodeModel::JITDefault,
            "small" => CodeModel::Small,
            "kernel" => CodeModel::Kernel,
            "medium" => CodeModel::Medium,
            "large" => CodeModel::Large,
            other => panic!("unsupported -C code-model={}", other),
        };
    }

    match target {
        CodegenTarget::FreestandingX86_64 => CodeModel::Kernel,
        _ => CodeModel::Default,
    }
}

fn reloc_mode_from_backend(backend: &BackendOptions, target: CodegenTarget) -> RelocMode {
    if let Some(model) = backend.relocation_model.as_deref() {
        return match model {
            "default" => RelocMode::Default,
            "static" => RelocMode::Static,
            "pic" | "pie" => RelocMode::PIC,
            "dynamic-no-pic" | "dynamic_no_pic" => RelocMode::DynamicNoPic,
            other => panic!("unsupported -C relocation-model={}", other),
        };
    }

    if backend.freestanding
        || matches!(
            target,
            CodegenTarget::FreestandingX86_64
                | CodegenTarget::FreestandingArm64
                | CodegenTarget::FreestandingRISCV64
        )
    {
        RelocMode::Static
    } else {
        RelocMode::Default
    }
}

static INIT_LLVM_TARGETS: Once = Once::new();

fn codegen_trace(step: &str) {
    if std::env::var_os("WAVE_CODEGEN_TRACE").is_some() {
        eprintln!("[wavec-codegen] {}", step);
    }
}

fn initialize_llvm_targets() {
    INIT_LLVM_TARGETS.call_once(|| {
        let config = InitializationConfig::default();

        #[cfg(feature = "llvm-target-all")]
        {
            Target::initialize_all(&config);
        }

        #[cfg(all(not(feature = "llvm-target-all"), feature = "llvm-target-x86"))]
        {
            Target::initialize_x86(&config);
        }

        #[cfg(all(not(feature = "llvm-target-all"), feature = "llvm-target-aarch64"))]
        {
            Target::initialize_aarch64(&config);
        }

        #[cfg(all(not(feature = "llvm-target-all"), feature = "llvm-target-riscv"))]
        {
            Target::initialize_riscv(&config);
        }
    });
}

fn should_run_llvm_pass_pipeline() -> bool {
    // LLVM 21's C pass pipeline can jump through a null callback in the
    // MinGW-built Windows package. Code generation still uses the target
    // machine's optimization level, so keep Windows codegen usable by skipping
    // the in-process IR pass pipeline there.
    !cfg!(target_os = "windows")
}

fn should_disable_red_zone(backend: &BackendOptions, target: CodegenTarget) -> bool {
    backend.freestanding
        || matches!(
            target,
            CodegenTarget::FreestandingX86_64
                | CodegenTarget::FreestandingArm64
                | CodegenTarget::FreestandingRISCV64
        )
}

fn apply_function_codegen_attrs<'ctx>(
    context: &'ctx Context,
    function: FunctionValue<'ctx>,
    disable_red_zone: bool,
    cpu: &str,
    features: &str,
) {
    if !cpu.is_empty() {
        function.add_attribute(
            AttributeLoc::Function,
            context.create_string_attribute("target-cpu", cpu),
        );
    }
    if !features.is_empty() {
        function.add_attribute(
            AttributeLoc::Function,
            context.create_string_attribute("target-features", features),
        );
    }
    if disable_red_zone {
        let no_red_zone = Attribute::get_named_enum_kind_id("noredzone");
        let attr = context.create_enum_attribute(no_red_zone, 0);
        function.add_attribute(AttributeLoc::Function, attr);

        let no_unwind = Attribute::get_named_enum_kind_id("nounwind");
        let attr = context.create_enum_attribute(no_unwind, 0);
        function.add_attribute(AttributeLoc::Function, attr);
    }
}

pub unsafe fn generate_ir(
    ast_nodes: &[ASTNode],
    opt_flag: &str,
    backend: &BackendOptions,
) -> String {
    let generated = build_module(ast_nodes, opt_flag, backend);
    generated.module.print_to_string().to_string()
}

pub unsafe fn emit_codegen_file(
    ast_nodes: &[ASTNode],
    opt_flag: &str,
    backend: &BackendOptions,
    output: &std::path::Path,
    kind: CodegenFileKind,
) {
    let generated = build_module(ast_nodes, opt_flag, backend);

    match kind {
        CodegenFileKind::Bitcode => {
            if !generated.module.write_bitcode_to_path(output) {
                panic!("failed to write LLVM bitcode to '{}'", output.display());
            }
        }
        CodegenFileKind::Assembly => generated
            .target_machine
            .write_to_file(generated.module, FileType::Assembly, output)
            .unwrap_or_else(|e| {
                panic!(
                    "failed to emit LLVM assembly to '{}': {}",
                    output.display(),
                    e.to_string()
                )
            }),
        CodegenFileKind::Object => generated
            .target_machine
            .write_to_file(generated.module, FileType::Object, output)
            .unwrap_or_else(|e| {
                panic!(
                    "failed to emit object file to '{}': {}",
                    output.display(),
                    e.to_string()
                )
            }),
    }
}

fn build_module(
    ast_nodes: &[ASTNode],
    opt_flag: &str,
    backend: &BackendOptions,
) -> GeneratedModule {
    codegen_trace("initialize targets");
    initialize_llvm_targets();

    codegen_trace("create context");
    let context: &'static Context = Box::leak(Box::new(Context::create()));
    codegen_trace("create module");
    let module: &'static _ = Box::leak(Box::new(context.create_module("main")));
    codegen_trace("create builder");
    let builder: &'static _ = Box::leak(Box::new(context.create_builder()));

    codegen_trace("resolve named types");
    let named_types = collect_named_types(ast_nodes);
    let mut ast_nodes: Vec<ASTNode> = ast_nodes
        .iter()
        .map(|n| resolve_ast_node(n, &named_types))
        .collect();
    let proto_functions: Vec<ASTNode> = ast_nodes
        .iter()
        .filter_map(|node| match node {
            ASTNode::ProtoImpl(implementation) => Some(implementation),
            _ => None,
        })
        .flat_map(|implementation| {
            implementation.methods.iter().map(|method| {
                let mut function = method.clone();
                function.name = format!("{}_{}", implementation.target, method.name);
                ASTNode::Function(function)
            })
        })
        .collect();
    ast_nodes.extend(proto_functions);
    let semantic_types =
        parser::verification::analyze_expression_types(&ast_nodes).unwrap_or_else(|diagnostic| {
            panic!("semantic analysis before codegen failed: {diagnostic}")
        });
    super::semantic::install_expression_types(semantic_types);

    codegen_trace("resolve target triple");
    let triple = if let Some(raw) = &backend.target {
        TargetTriple::create(raw)
    } else {
        TargetMachine::get_default_triple()
    };
    let abi_target = require_supported_target_from_triple(&triple);
    let disable_red_zone = should_disable_red_zone(backend, abi_target);
    codegen_trace("lookup target");
    let target = Target::from_triple(&triple).unwrap();
    let cpu = backend.cpu.as_deref().unwrap_or("generic");
    let features = backend.features.as_deref().unwrap_or("");
    let reloc_mode = reloc_mode_from_backend(backend, abi_target);
    let code_model = code_model_from_backend(backend, abi_target);

    codegen_trace("create target machine");
    let mut target_options = TargetMachineOptions::new()
        .set_cpu(cpu)
        .set_features(features)
        .set_level(target_opt_level_from_flag(opt_flag))
        .set_reloc_mode(reloc_mode)
        .set_code_model(code_model);
    if let Some(abi) = backend.abi.as_deref() {
        target_options = target_options.set_abi(abi);
    }
    let tm = target
        .create_target_machine_from_options(&triple, target_options)
        .unwrap();

    codegen_trace("set target metadata");
    module.set_triple(&triple);

    let td_val: TargetData = tm.get_target_data();
    module.set_data_layout(&td_val.get_data_layout());
    if abi_target.architecture() == super::arch::Architecture::Riscv64 {
        if let Some(abi) = backend.abi.as_deref() {
            module.add_metadata_flag(
                "target-abi",
                FlagBehavior::Error,
                context.metadata_string(abi),
            );
        }
        if let Some(isa) = backend.isa.as_deref() {
            let isa_node = context.metadata_node(&[context.metadata_string(isa).into()]);
            module.add_metadata_flag("riscv-isa", FlagBehavior::AppendUnique, isa_node);
        }
    }
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

        let llvm_ty =
            wave_type_to_llvm_type(context, &v.type_name, &struct_types, TypeFlavor::AbiC);
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

    let mut functions: HashMap<String, FunctionValue> = HashMap::new();
    let mut export_wrappers: Vec<ExportCWrapper> = Vec::new();

    let function_nodes: Vec<&FunctionNode> = ast_nodes
        .iter()
        .filter_map(|ast| {
            if let ASTNode::Function(f) = ast {
                Some(f)
            } else {
                None
            }
        })
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
        export,
        ..
    } in function_nodes.iter().copied()
    {
        if let Some(export) = export {
            if !is_supported_extern_abi(&export.abi) {
                panic!(
                    "unsupported export ABI '{}' for function '{}': only export(c) is currently supported",
                    export.abi, name
                );
            }
        }

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

        if let Some(export) = export {
            let wave_param_types = parameters
                .iter()
                .map(|parameter| {
                    wave_type_to_llvm_type(
                        context,
                        &parameter.param_type,
                        &struct_types,
                        TypeFlavor::AbiC,
                    )
                })
                .collect::<Vec<_>>();
            let wave_ret_type = return_type.as_ref().and_then(|return_type| {
                if *return_type == WaveType::Void {
                    None
                } else {
                    Some(wave_type_to_llvm_type(
                        context,
                        return_type,
                        &struct_types,
                        TypeFlavor::AbiC,
                    ))
                }
            });
            let export_decl = ExternFunctionNode {
                name: name.clone(),
                abi: export.abi.clone(),
                symbol: export.symbol.clone(),
                params: parameters
                    .iter()
                    .map(|parameter| (parameter.name.clone(), parameter.param_type.clone()))
                    .collect(),
                variadic: false,
                return_type: return_type.clone().unwrap_or(WaveType::Void),
            };
            let lowered = lower_extern_c(context, td, abi_target, &export_decl, &struct_types);
            let wrapper = module.add_function(&lowered.llvm_name, lowered.fn_type, None);
            apply_extern_c_attrs(context, wrapper, &lowered.info);
            apply_function_codegen_attrs(context, wrapper, disable_red_zone, cpu, features);

            let implementation_name = format!("__wave_export_impl_{}", name);
            let implementation =
                module.add_function(&implementation_name, fn_type, Some(Linkage::Internal));
            apply_function_codegen_attrs(context, implementation, disable_red_zone, cpu, features);

            functions.insert(name.clone(), implementation);
            extern_c_info.insert(name.clone(), lowered.info.clone());
            export_wrappers.push(ExportCWrapper {
                wrapper,
                implementation,
                info: lowered.info,
                wave_param_types,
                wave_ret_type,
            });
        } else {
            let function = module.add_function(name, fn_type, None);
            apply_function_codegen_attrs(context, function, disable_red_zone, cpu, features);
            functions.insert(name.clone(), function);
        }
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
        apply_function_codegen_attrs(context, f, disable_red_zone, cpu, features);

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
            if builder
                .get_insert_block()
                .is_some_and(|block| block.get_terminator().is_some())
            {
                break;
            }
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

    for export in &export_wrappers {
        build_export_c_wrapper(context, builder, td, export);
    }

    if should_run_llvm_pass_pipeline() {
        let pbo = PassBuilderOptions::create();
        let pipeline = pipeline_from_opt_flag(opt_flag);

        codegen_trace("run optimization passes");
        module
            .run_passes(pipeline, &tm, pbo)
            .expect("failed to run optimization passes");
    } else {
        codegen_trace("skip optimization passes");
    }

    codegen_trace("finish module");
    GeneratedModule {
        module,
        target_machine: tm,
    }
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

        ASTNode::TypeAlias(_) => n.clone(),

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
