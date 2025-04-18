use crate::parser::ast::{ASTNode, FunctionNode, StatementNode, Expression, VariableNode, Literal, Operator, WaveType, Value};
use inkwell::context::Context;
use inkwell::module::Linkage;
use inkwell::values::{PointerValue, FunctionValue, BasicValue, BasicValueEnum};
use inkwell::{AddressSpace, FloatPredicate};

use std::collections::HashMap;
use inkwell::basic_block::BasicBlock;
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum};
use crate::lexer::TokenType;

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

                let mut variables = HashMap::new();
                let mut string_counter = 0;
                let mut loop_exit_stack = vec![];
                let mut loop_continue_stack = vec![];

                for (i, param) in parameters.iter().enumerate() {
                    let llvm_type = wave_type_to_llvm_type(&context, &param.param_type);
                    let alloca = builder.build_alloca(llvm_type, &param.name).unwrap();

                    let llvm_param = function.get_nth_param(i as u32).unwrap();
                    builder.build_store(alloca, llvm_param).unwrap();

                    variables.insert(param.name.clone(), alloca);
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

fn wave_format_to_c(format: &str, arg_types: &[BasicTypeEnum]) -> String {
    let mut result = String::new();
    let mut chars = format.chars().peekable();
    let mut arg_index = 0;

    while let Some(c) = chars.next() {
        if c == '{' {
            if let Some('}') = chars.peek() {
                chars.next(); // consume '}'

                if let Some(arg_type) = arg_types.get(arg_index) {
                    let fmt = match arg_type {
                        BasicTypeEnum::FloatType(_) => "%f",
                        BasicTypeEnum::IntType(_) => "%d",
                        BasicTypeEnum::PointerType(_) => "%s",
                        _ => "%d", // fallback
                    };
                    result.push_str(fmt);
                    arg_index += 1;
                    continue;
                }
            }
        }
        result.push(c);
    }

    result
}

fn generate_expression_ir<'ctx>(
    context: &'ctx Context,
    builder: &'ctx inkwell::builder::Builder<'ctx>,
    expr: &Expression,
    variables: &mut HashMap<String, PointerValue<'ctx>>,
    module: &'ctx inkwell::module::Module<'ctx>,
) -> BasicValueEnum<'ctx> {
    match expr {
        Expression::Literal(lit) => match lit {
            Literal::Number(value) => {
                context.i32_type().const_int(*value as u64, false).as_basic_value_enum()
            }
            Literal::Float(value) => {
                context.f32_type().const_float(*value).as_basic_value_enum()
            }
            Literal::String(value) => unsafe {
                let bytes = value.as_bytes();
                let mut null_terminated = bytes.to_vec();
                null_terminated.push(0);

                let global_name = format!("str_lit_{}", value.replace(" ", "_"));
                let str_type = context.i8_type().array_type(null_terminated.len() as u32);
                let global = module.add_global(str_type, None, &global_name);
                global.set_initializer(&context.const_string(&null_terminated, false));
                global.set_constant(true);

                let zero = context.i32_type().const_zero();
                let indices = [zero, zero];
                let gep = builder.build_gep(global.as_pointer_value(), &indices, "str_gep").unwrap();

                gep.as_basic_value_enum()
            }
            _ => unimplemented!("Unsupported literal type"),
        },

        Expression::Variable(var_name) => {
            if let Some(alloca) = variables.get(var_name) {
                builder.build_load(*alloca, var_name).unwrap()
            } else if module.get_function(var_name).is_some() {
                panic!("Error: '{}' is a function name, not a variable", var_name);
            } else {
                panic!("variable '{}' not found in current scope", var_name);
            }
        }

        Expression::FunctionCall { name, args } => {
            let function = module
                .get_function(name)
                .unwrap_or_else(|| panic!("Function '{}' not found", name));

            let mut compiled_args = vec![];
            for arg in args {
                let val = generate_expression_ir(context, builder, arg, variables, module);
                compiled_args.push(val.into());
            }

            let call_site = builder.build_call(function, &compiled_args, "calltmp").unwrap();

            if function.get_type().get_return_type().is_some() {
                call_site.try_as_basic_value().left().unwrap()
            } else {
                context.i32_type().const_int(0, false).as_basic_value_enum()
            }
        }

        Expression::BinaryExpression { left, operator, right } => {
            let left_val = generate_expression_ir(context, builder, left, variables, module);
            let right_val = generate_expression_ir(context, builder, right, variables, module);

            // Branch after Type Examination
            match (left_val, right_val) {
                (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                    let result = match operator {
                        Operator::Add => builder.build_int_add(l, r, "addtmp"),
                        Operator::Subtract => builder.build_int_sub(l, r, "subtmp"),
                        Operator::Multiply => builder.build_int_mul(l, r, "multmp"),
                        Operator::Divide => builder.build_int_signed_div(l, r, "divtmp"),
                        Operator::Greater => builder.build_int_compare(inkwell::IntPredicate::SGT, l, r, "cmptmp"),
                        Operator::Less => builder.build_int_compare(inkwell::IntPredicate::SLT, l, r, "cmptmp"),
                        Operator::Equal => builder.build_int_compare(inkwell::IntPredicate::EQ, l, r, "cmptmp"),
                        Operator::NotEqual => builder.build_int_compare(inkwell::IntPredicate::NE, l, r, "cmptmp"),
                        Operator::GreaterEqual => builder.build_int_compare(inkwell::IntPredicate::SGE, l, r, "cmptmp"),
                        Operator::LessEqual => builder.build_int_compare(inkwell::IntPredicate::SLE, l, r, "cmptmp"),
                        _ => panic!("Unsupported binary operator"),
                    };
                    result.unwrap().as_basic_value_enum()
                }

                (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
                    match operator {
                        Operator::Greater => builder.build_float_compare(FloatPredicate::OGT, l, r, "fcmpgt").unwrap().as_basic_value_enum(),
                        Operator::Less => builder.build_float_compare(FloatPredicate::OLT, l, r, "fcmplt").unwrap().as_basic_value_enum(),
                        Operator::Equal => builder.build_float_compare(FloatPredicate::OEQ, l, r, "fcmpeq").unwrap().as_basic_value_enum(),
                        Operator::NotEqual => builder.build_float_compare(FloatPredicate::ONE, l, r, "fcmpne").unwrap().as_basic_value_enum(),
                        Operator::GreaterEqual => builder.build_float_compare(FloatPredicate::OGE, l, r, "fcmpge").unwrap().as_basic_value_enum(),
                        Operator::LessEqual => builder.build_float_compare(FloatPredicate::OLE, l, r, "fcmple").unwrap().as_basic_value_enum(),
                        _ => panic!("Unsupported float operator"),
                    }
                }

                _ => panic!("Type mismatch in binary expression"),
            }
        }

        _ => unimplemented!("Unsupported expression type"),
    }
}

fn wave_type_to_llvm_type<'ctx>(context: &'ctx Context, wave_type: &WaveType) -> BasicTypeEnum<'ctx> {
    match wave_type {
        WaveType::Int(bits) => context.custom_width_int_type(*bits as u32).as_basic_type_enum(),
        WaveType::Uint(bits) => context.custom_width_int_type(*bits as u32).as_basic_type_enum(),
        WaveType::Float(bits) => match bits {
            32 => context.f32_type().as_basic_type_enum(),
            64 => context.f64_type().as_basic_type_enum(),
            _ => panic!("Unsupported float bit width: {}", bits),
        },
        WaveType::Bool => context.bool_type().as_basic_type_enum(),
        WaveType::Char => context.i8_type().as_basic_type_enum(), // assuming 1-byte char
        WaveType::Byte => context.i8_type().as_basic_type_enum(),
        WaveType::String => context.i8_type().ptr_type(AddressSpace::default()).as_basic_type_enum(),
        WaveType::Pointer(inner) => wave_type_to_llvm_type(context, inner).ptr_type(AddressSpace::default()).as_basic_type_enum(),
        WaveType::Array(inner, size) => {
            let inner_type = wave_type_to_llvm_type(context, inner);
            inner_type.array_type(*size).as_basic_type_enum()
        }
    }
}

fn generate_statement_ir<'ctx>(
    context: &'ctx Context,
    builder: &'ctx inkwell::builder::Builder<'ctx>,
    module: &'ctx inkwell::module::Module<'ctx>,
    string_counter: &mut usize,
    stmt: &ASTNode,
    variables: &mut HashMap<String, PointerValue<'ctx>>,
    loop_exit_stack: &mut Vec<BasicBlock<'ctx>>,
    loop_continue_stack: &mut Vec<BasicBlock<'ctx>>,
) {
    match stmt {
        ASTNode::Variable(VariableNode { name, type_name, initial_value }) => {
            // Parse the type
            let llvm_type = wave_type_to_llvm_type(&context, &type_name);

            // Create alloca for the variable
            let alloca = builder.build_alloca(llvm_type, &name).unwrap();
            variables.insert(name.clone(), alloca);

            // Initialize the variable if an initial value is provided
            if let Some(init) = initial_value {
                match (init, llvm_type) {
                    (Literal::Number(value), BasicTypeEnum::IntType(int_type)) => {
                        let init_value = int_type.const_int(*value as u64, false);
                        let _ = builder.build_store(alloca, init_value);
                    }
                    (Literal::Float(value), BasicTypeEnum::FloatType(float_type)) => {
                        let init_value = float_type.const_float(*value);
                        let _ = builder.build_store(alloca, init_value);
                    }
                    (Literal::String(value), BasicTypeEnum::PointerType(ptr_type)) => {
                        let string_name = format!("str_init_{}", name);
                        let mut bytes = value.as_bytes().to_vec();
                        bytes.push(0); // null-terminated

                        let const_str = context.const_string(&bytes, false);

                        let global = module.add_global(
                            context.i8_type().array_type(bytes.len() as u32),
                            None,
                            &string_name,
                        );
                        global.set_initializer(&const_str);
                        global.set_linkage(Linkage::Private);
                        global.set_constant(true);

                        let zero = context.i32_type().const_zero();
                        let indices = [zero, zero];
                        let gep = unsafe {
                            builder.build_gep(global.as_pointer_value(), &indices, "str_gep").unwrap()
                        };

                        let _ = builder.build_store(alloca, gep);
                    }
                    _ => panic!("Unsupported type/value combination for initialization"),
                }
            }
        }
        ASTNode::Statement(StatementNode::Println(message)) |
        ASTNode::Statement(StatementNode::Print(message)) => {
            let global_name = format!("str_{}", *string_counter);
            *string_counter += 1;

            let mut bytes = message.as_bytes().to_vec();
            bytes.push(0);
            let const_str = context.const_string(&bytes, false);

            let global = module.add_global(
                context.i8_type().array_type(bytes.len() as u32),
                None,
                &global_name,
            );
            global.set_initializer(&const_str);
            global.set_linkage(Linkage::Private);
            global.set_constant(true);

            let printf_type = context.i32_type().fn_type(
                &[context.i8_type().ptr_type(AddressSpace::default()).into()],
                true,
            );
            let printf_func = match module.get_function("printf") {
                Some(f) => f,
                None => module.add_function("printf", printf_type, None),
            };

            let zero = context.i32_type().const_zero();
            let indices = [zero, zero];
            let gep = unsafe {
                builder.build_gep(global.as_pointer_value(), &indices, "gep").unwrap()
            };

            let _ = builder.build_call(printf_func, &[gep.into()], "printf_call");
        }
        ASTNode::Statement(StatementNode::PrintlnFormat { format, args }) |
        ASTNode::Statement(StatementNode::PrintFormat { format, args }) => {
            let mut arg_types = vec![];
            for arg in args {
                let val = generate_expression_ir(context, builder, arg, variables, module);
                arg_types.push(val.get_type());
            }
            let c_format_string = wave_format_to_c(&format, &arg_types);

            let global_name = format!("str_{}", *string_counter);
            *string_counter += 1;

            let mut bytes = c_format_string.as_bytes().to_vec();
            bytes.push(0);
            let const_str = context.const_string(&bytes, false);

            let global = module.add_global(
                context.i8_type().array_type(bytes.len() as u32),
                None,
                &global_name,
            );
            global.set_initializer(&const_str);
            global.set_linkage(Linkage::Private);
            global.set_constant(true);

            let printf_type = context.i32_type().fn_type(
                &[context.i8_type().ptr_type(AddressSpace::default()).into()],
                true,
            );
            let printf_func = match module.get_function("printf") {
                Some(func) => func,
                None => module.add_function("printf", printf_type, None),
            };

            let zero = context.i32_type().const_zero();
            let indices = [zero, zero];
            let gep = unsafe {
                builder.build_gep(global.as_pointer_value(), &indices, "gep").unwrap()
            };

            let mut printf_args = vec![gep.into()];
            for arg in args {
                let value = generate_expression_ir(context, builder, arg, variables, module);

                let casted_value = match value {
                    BasicValueEnum::FloatValue(fv) => {
                        let double_ty = context.f64_type();
                        builder
                            .build_float_ext(fv, double_ty, "cast_to_double")
                            .unwrap()
                            .as_basic_value_enum()
                    }
                    _ => value,
                };

                printf_args.push(casted_value.into());
            }

            let _ = builder.build_call(printf_func, &printf_args, "printf_call");
        }
        ASTNode::Statement(StatementNode::If {
                               condition,
                               body,
                               else_if_blocks,
                               else_block,
                           }) => {
            let current_fn = builder.get_insert_block().unwrap().get_parent().unwrap();

            let cond_value = generate_expression_ir(context, builder, condition, variables, module);

            let then_block = context.append_basic_block(current_fn, "then");
            let else_block_bb = context.append_basic_block(current_fn, "else");
            let merge_block = context.append_basic_block(current_fn, "merge");

            let _ = builder.build_conditional_branch(cond_value.try_into().unwrap(), then_block, else_block_bb);

            // then
            builder.position_at_end(then_block);
            for stmt in body {
                generate_statement_ir(context, builder, module, string_counter, stmt, variables, loop_exit_stack, loop_continue_stack);
            }
            let _ = builder.build_unconditional_branch(merge_block);

            // else
            builder.position_at_end(else_block_bb);

            if let Some(else_ifs) = else_if_blocks {
                for else_if in else_ifs.iter() {
                    generate_statement_ir(context, builder, module, string_counter, else_if, variables, loop_exit_stack, loop_continue_stack);
                }
            }

            if let Some(else_body) = else_block {
                for stmt in else_body.iter() {
                    generate_statement_ir(context, builder, module, string_counter, stmt, variables, loop_exit_stack, loop_continue_stack);
                }
            }

            let _ = builder.build_unconditional_branch(merge_block);
            builder.position_at_end(merge_block);
        }
        ASTNode::Statement(StatementNode::While { condition, body }) => {
            let current_fn = builder.get_insert_block().unwrap().get_parent().unwrap();

            let cond_block = context.append_basic_block(current_fn, "while.cond");
            let body_block = context.append_basic_block(current_fn, "while.body");
            let merge_block = context.append_basic_block(current_fn, "while.end");

            loop_exit_stack.push(merge_block);
            loop_continue_stack.push(cond_block);

            let _ = builder.build_unconditional_branch(cond_block);
            builder.position_at_end(cond_block);

            let cond_val = generate_expression_ir(context, builder, condition, variables, module);

            let cond_bool = match cond_val {
                BasicValueEnum::IntValue(val) => {
                    let zero = val.get_type().const_zero();
                    builder
                        .build_int_compare(inkwell::IntPredicate::NE, val, zero, "while_cond")
                        .unwrap()
                }
                BasicValueEnum::FloatValue(val) => {
                    let zero = val.get_type().const_float(0.0);
                    builder
                        .build_float_compare(FloatPredicate::ONE, val, zero, "while_cond")
                        .unwrap()
                }
                _ => panic!("Unsupported condition type in while loop"),
            };

            let _ = builder.build_conditional_branch(cond_bool, body_block, merge_block);

            builder.position_at_end(body_block);
            for stmt in body.iter() {
                generate_statement_ir(context, builder, module, string_counter, stmt, variables, loop_exit_stack, loop_continue_stack);
            }
            let _ = builder.build_unconditional_branch(cond_block);

            loop_exit_stack.pop();
            loop_continue_stack.pop();

            builder.position_at_end(merge_block);
        }
        ASTNode::Statement(StatementNode::Expression(expr)) => {
            let _ = generate_expression_ir(context, builder, expr, variables, module);
        }
        ASTNode::Statement(StatementNode::Assign { variable, value }) => {
            let val = generate_expression_ir(context, builder, value, variables, module);
            if let Some(ptr) = variables.get(variable) {
                let _ = builder.build_store(*ptr, val);
            } else {
                panic!("Variable {} not declared", variable);
            }
        }
        ASTNode::Statement(StatementNode::Break) => {
            if let Some(target_block) = loop_exit_stack.last() {
                let _ = builder.build_unconditional_branch(*target_block);
            } else {
                panic!("break used outside of loop!");
            }
        }
        ASTNode::Statement(StatementNode::Continue) => {
            if let Some(target_block) = loop_continue_stack.last() {
                let _ = builder.build_unconditional_branch(*target_block);
            } else {
                panic!("continue used outside of loop!");
            }
        }
        ASTNode::Statement(StatementNode::Return(expr_opt)) => {
            if let Some(expr) = expr_opt {
                let value = generate_expression_ir(context, builder, expr, variables, module);
                let _ = builder.build_return(Some(&value));
            } else {
                let _ = builder.build_return(None);
            }
        }
        _ => {}
    }
}

fn get_llvm_type<'a>(context: &'a Context, ty: &TokenType) -> BasicTypeEnum<'a> {
    match ty {
        TokenType::TypeInt(bits) => context.custom_width_int_type(*bits as u32).as_basic_type_enum(),
        TokenType::TypeUint(bits) => context.custom_width_int_type(*bits as u32).as_basic_type_enum(),
        TokenType::TypeFloat(bits) => match bits {
            32 => context.f32_type().as_basic_type_enum(),
            64 => context.f64_type().as_basic_type_enum(),
            128 => context.f128_type().as_basic_type_enum(),
            _ => panic!("Unsupported float size: {}", bits),
        },
        TokenType::TypeBool => context.bool_type().as_basic_type_enum(),
        TokenType::TypeChar => context.i8_type().as_basic_type_enum(),
        TokenType::TypeByte => context.i8_type().as_basic_type_enum(),
        TokenType::TypePointer(inner_type) => {
            let inner_llvm_type = get_llvm_type(context, &*inner_type); // Box 역참조
            inner_llvm_type.ptr_type(AddressSpace::default()).as_basic_type_enum()
        }
        TokenType::TypeArray(inner_type, size) => {
            let inner_llvm_type = get_llvm_type(context, &*inner_type); // Box 역참조
            inner_llvm_type.array_type(*size as u32).as_basic_type_enum()
        }
        TokenType::TypeString => context.i8_type().ptr_type(AddressSpace::default()).as_basic_type_enum(),
        _ => panic!("Unsupported type: {:?}", ty),
    }
}

unsafe fn create_alloc<'a>(
    context: &'a Context,
    builder: &'a inkwell::builder::Builder<'a>,
    function: FunctionValue<'a>,
    name: &'a str,
) -> PointerValue<'a> {
    let alloca = builder.build_alloca(context.i32_type(), name).unwrap();
    alloca
}