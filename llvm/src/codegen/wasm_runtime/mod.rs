// This file is part of the Wave language project.
// SPDX-License-Identifier: MPL-2.0
//! Freestanding arithmetic legalization. Helpers contain no host calls and use
//! only operations that WebAssembly can lower without a runtime SDK.
mod templates;
use inkwell::{
    context::Context,
    memory_buffer::MemoryBuffer,
    module::{Linkage, Module},
    values::{BasicValue, InstructionOpcode, InstructionValue},
};
use std::collections::HashMap;

/// Run after optimization so unused operations never pull in runtime support.
/// Functions are private and cached by operation, independently of user symbols.
pub(super) fn lower<'ctx>(context: &'ctx Context, module: &Module<'ctx>) -> Result<(), String> {
    let builder = context.create_builder();
    let mut helpers = HashMap::new();
    loop {
        let instructions: Vec<_> = module
            .get_functions()
            .flat_map(|f| f.get_basic_blocks())
            .flat_map(|b| b.get_instructions())
            .filter(|i| operation(*i).is_some())
            .collect();
        if instructions.is_empty() {
            break;
        }
        for instruction in instructions {
            let Some(key) = operation(instruction) else {
                continue;
            };
            let function = if let Some(function) = helpers.get(&key) {
                *function
            } else {
                let mut name = format!("__wave.runtime.{key}");
                while module.get_function(&name).is_some() || module.get_global(&name).is_some() {
                    name.push('.');
                }
                let text = templates::definition(&name, &key);
                let helper = context
                    .create_module_from_ir(MemoryBuffer::create_from_memory_range_copy(
                        text.as_bytes(),
                        "wave-runtime",
                    ))
                    .map_err(|e| format!("invalid compiler arithmetic runtime: {e}"))?;
                helper.set_triple(&module.get_triple());
                helper.set_data_layout(&module.get_data_layout());
                module.link_in_module(helper).map_err(|e| e.to_string())?;
                let function = module
                    .get_function(&name)
                    .expect("linked arithmetic helper");
                function.set_linkage(Linkage::Private);
                helpers.insert(key.clone(), function);
                function
            };
            builder.position_before(&instruction);
            let arguments: Vec<_> = instruction
                .get_operands()
                .map(|op| op.unwrap().unwrap_value().into())
                .collect();
            let call = builder
                .build_call(function, &arguments, "runtime")
                .map_err(|e| e.to_string())?;
            instruction.replace_all_uses_with(
                &call
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .as_instruction_value()
                    .unwrap(),
            );
            instruction.erase_from_basic_block();
        }
    }
    module.verify().map_err(|e| e.to_string())
}

fn operation(instruction: InstructionValue<'_>) -> Option<String> {
    use InstructionOpcode::*;
    if !matches!(
        instruction.get_opcode(),
        UDiv | SDiv | URem | SRem | Mul | Shl | LShr | AShr | UIToFP | SIToFP | FPToUI | FPToSI
    ) {
        return None;
    }
    let source = instruction
        .get_operand(0)?
        .value()?
        .get_type()
        .print_to_string()
        .to_string();
    let target = instruction.get_type().print_to_string().to_string();
    let opcode = match instruction.get_opcode() {
        UDiv if source == "i128" => "udiv",
        SDiv if source == "i128" => "sdiv",
        URem if source == "i128" => "urem",
        SRem if source == "i128" => "srem",
        Mul if source == "i128" => "mul",
        Shl | LShr | AShr
            if source == "i128"
                && !instruction
                    .get_operand(1)?
                    .value()?
                    .into_int_value()
                    .is_const() =>
        {
            match instruction.get_opcode() {
                Shl => "shl",
                LShr => "lshr",
                AShr => "ashr",
                _ => unreachable!(),
            }
        }
        UIToFP if source == "i128" && matches!(target.as_str(), "float" | "double") => "uitofp",
        SIToFP if source == "i128" && matches!(target.as_str(), "float" | "double") => "sitofp",
        FPToUI if target == "i128" && matches!(source.as_str(), "float" | "double") => "fptoui",
        FPToSI if target == "i128" && matches!(source.as_str(), "float" | "double") => "fptosi",
        _ => return None,
    };
    Some(format!("{opcode}.{source}.{target}"))
}
