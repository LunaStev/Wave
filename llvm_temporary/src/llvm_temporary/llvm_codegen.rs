use parser::ast::{ASTNode, FunctionNode, Mutability, Value};
use inkwell::context::Context;
use inkwell::values::{FunctionValue, BasicValue};

use std::collections::HashMap;
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum};
use crate::llvm_temporary::statement::generate_statement_ir;
use crate::llvm_temporary::type_utils::{wave_type_to_llvm_type, VariableInfo};

pub unsafe fn generate_ir(ast_nodes: &[ASTNode]) -> String {
    let context = Context::create();

    let ir = {
        let module = Box::leak(Box::new(context.create_module("main")));
        let builder = Box::leak(Box::new(context.create_builder()));
        let mut functions: HashMap<String, FunctionValue> = HashMap::new();

        for ast in ast_nodes {
            if let ASTNode::Function(FunctionNode { name, parameters, return_type, .. }) = ast {
                let param_types: Vec<BasicMetadataTypeEnum> = parameters.iter()
                    .map(|p| wave_type_to_llvm_type(&context, &p.param_type).into())
                    .collect();

                let fn_type = match return_type {
                    Some(wave_ret_ty) => {
                        let llvm_ret_type = wave_type_to_llvm_type(&context, wave_ret_ty);
                        match llvm_ret_type {
                            BasicTypeEnum::IntType(int_ty) => int_ty.fn_type(&param_types, false),
                            BasicTypeEnum::FloatType(float_ty) => float_ty.fn_type(&param_types, false),
                            BasicTypeEnum::PointerType(ptr_ty) => ptr_ty.fn_type(&param_types, false),
                            _ => panic!("Unsupported return type"),
                        }
                    }
                    None => context.void_type().fn_type(&param_types, false),
                };

                let function = module.add_function(name, fn_type, None);
                functions.insert(name.clone(), function);
            }
        }

        for ast in ast_nodes {
            if let ASTNode::Function(FunctionNode { name, parameters, return_type, body }) = ast {
                let function = *functions.get(name).unwrap();

                let entry_block = context.append_basic_block(function, "entry");
                builder.position_at_end(entry_block);

                let mut variables: HashMap<String, VariableInfo> = HashMap::new();
                let mut string_counter = 0;
                let mut loop_exit_stack = vec![];
                let mut loop_continue_stack = vec![];

                for (i, param) in parameters.iter().enumerate() {
                    let llvm_type = wave_type_to_llvm_type(&context, &param.param_type);
                    let alloca = builder.build_alloca(llvm_type, &param.name).unwrap();

                    let init_value = if let Some(initial) = &param.initial_value {
                        match (initial, llvm_type) {
                            (Value::Int(v), BasicTypeEnum::IntType(int_ty)) => {
                                Some(int_ty.const_int(*v as u64, false).as_basic_value_enum())
                            }
                            (Value::Float(f), BasicTypeEnum::FloatType(float_ty)) => {
                                Some(float_ty.const_float(*f).as_basic_value_enum())
                            }
                            (Value::Text(s), BasicTypeEnum::PointerType(ptr_ty)) => unsafe {
                                let mut bytes = s.as_bytes().to_vec();
                                bytes.push(0);
                                let const_str = context.const_string(&bytes, false);
                                let global = module.add_global(
                                    context.i8_type().array_type(bytes.len() as u32),
                                    None,
                                    &format!("param_str_{}", param.name),
                                );
                                global.set_initializer(&const_str);
                                global.set_constant(true);
                                let zero = context.i32_type().const_zero();
                                let gep = builder.build_gep(global.as_pointer_value(), &[zero, zero], "gep").unwrap();
                                Some(gep.as_basic_value_enum())
                            }
                            _ => None,
                        }
                    } else {
                        Some(function.get_nth_param(i as u32).unwrap())
                    };

                    if let Some(init_val) = init_value {
                        builder.build_store(alloca, init_val).unwrap();
                    }

                    variables.insert(
                        param.name.clone(),
                        VariableInfo {
                            ptr: alloca,
                            mutability: Mutability::Let,
                        },
                    );
                }

                let is_void_fn = return_type.is_none();
                let did_return = false;

                for stmt in body {
                    match stmt {
                        ASTNode::Variable(_) | ASTNode::Statement(_) => {
                            generate_statement_ir(
                                &context,
                                &builder,
                                &module,
                                &mut string_counter,
                                stmt,
                                &mut variables,
                                &mut loop_exit_stack,
                                &mut loop_continue_stack,
                                function,
                            );
                        }
                        _ => panic!("Unsupported ASTNode in function body"),
                    }
                }

                if !did_return && is_void_fn {
                    let _ = builder.build_return(None);
                }
            }
        }

        module.print_to_string().to_string()
    };
    ir
}