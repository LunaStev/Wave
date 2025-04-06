use crate::parser::ast::{ASTNode, FunctionNode, StatementNode, Expression, VariableNode, Literal, Operator};
use inkwell::context::Context;
use inkwell::module::Linkage;
use inkwell::values::{PointerValue, FunctionValue};
use inkwell::AddressSpace;

use std::collections::HashMap;
use inkwell::types::{BasicType, BasicTypeEnum};
use crate::lexer::TokenType;
use crate::parser::parse_type;

pub unsafe fn generate_ir(ast: &ASTNode) -> String {
    let context = Context::create();

    let ir = {
        let module = Box::leak(Box::new(context.create_module("main")));
        let builder = Box::leak(Box::new(context.create_builder()));

        let mut variables: HashMap<String, PointerValue> = HashMap::new();
        let mut string_counter = 0;

        if let ASTNode::Function(FunctionNode { name, parameters: _, body }) = ast {
            // Create function type (void -> void)
            let fn_type = context.void_type().fn_type(&[], false);
            let function = module.add_function(name, fn_type, None);

            // Create entry block
            let entry_block = context.append_basic_block(function, "entry");
            builder.position_at_end(entry_block);

            let mut string_counter = 0;
            let mut loop_exit_stack = vec![];

            for stmt in body {
                generate_statement_ir(
                    &context,
                    &builder,
                    module,
                    &mut string_counter,
                    stmt,
                    &mut variables,
                    &mut loop_exit_stack,
                );
            }

            // Add void return
            let _ = builder.build_return(None);
        }
        module.print_to_string().to_string()
    };
    ir
}

fn wave_format_to_c(format: &str) -> String {
    let mut result = String::new();
    let mut chars = format.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '{' {
            if let Some('}') = chars.peek() {
                chars.next(); // consume '}'
                result.push_str("%d"); // Wave placeholder → C format
                continue;
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
) -> inkwell::values::IntValue<'ctx> {
    match expr {
        Expression::Literal(lit) => match lit {
            Literal::Number(value) => {
                context.i32_type().const_int(*value as u64, false).as_basic_value_enum().try_into().unwrap()
            }
            Literal::Float(value) => {
                context.f32_type().const_float(*value).as_basic_value_enum().try_into().unwrap()
            }
            _ => unimplemented!("Unsupported literal type"),
        },
        Expression::Variable(var_name) => {
            if let Some(alloca) = variables.get(var_name) {
                builder.build_load(*alloca, var_name).unwrap().into_int_value()
            } else {
                panic!("Variable {} not found", var_name);
            }
        }

        Expression::BinaryExpression { left, operator, right } => {
            let left_val = generate_expression_ir(context, builder, left, variables);
            let right_val = generate_expression_ir(context, builder, right, variables);

            match operator {
                Operator::Add => builder.build_int_add(left_val, right_val, "addtmp").unwrap(),
                Operator::Subtract => builder.build_int_sub(left_val, right_val, "subtmp").unwrap(),
                Operator::Multiply => builder.build_int_mul(left_val, right_val, "multmp").unwrap(),
                Operator::Divide => builder.build_int_signed_div(left_val, right_val, "divtmp").unwrap(),

                Operator::Greater => builder
                    .build_int_compare(inkwell::IntPredicate::SGT, left_val, right_val, "cmptmp")
                    .unwrap(),

                Operator::Less => builder
                    .build_int_compare(inkwell::IntPredicate::SLT, left_val, right_val, "cmptmp")
                    .unwrap(),

                Operator::Equal => builder
                    .build_int_compare(inkwell::IntPredicate::EQ, left_val, right_val, "cmptmp")
                    .unwrap(),

                Operator::NotEqual => builder
                    .build_int_compare(inkwell::IntPredicate::NE, left_val, right_val, "cmptmp")
                    .unwrap(),

                Operator::GreaterEqual => builder
                    .build_int_compare(inkwell::IntPredicate::SGE, left_val, right_val, "cmptmp")
                    .unwrap(),
                Operator::LessEqual => builder
                    .build_int_compare(inkwell::IntPredicate::SLE, left_val, right_val, "cmptmp")
                    .unwrap(),


                _ => panic!("Unsupported binary operator in generate_expression_ir"),
            }
        }

        _ => unimplemented!("Unsupported expression type"),
    }
}

fn generate_statement_ir<'ctx>(
    context: &'ctx Context,
    builder: &'ctx inkwell::builder::Builder<'ctx>,
    module: &'ctx inkwell::module::Module<'ctx>,
    string_counter: &mut usize,
    stmt: &ASTNode,
    variables: &mut HashMap<String, PointerValue<'ctx>>,
    loop_exit_stack: &mut Vec<inkwell::basic_block::BasicBlock<'ctx>>,
) {
    match stmt {
        ASTNode::Variable(VariableNode { name, type_name, initial_value }) => {
            // Parse the type
            let llvm_type = match parse_type(type_name) {
                Some(token_type) => get_llvm_type(&context, &token_type),
                None => panic!("Unsupported type: {}", type_name),
            };

            // Create alloca for the variable
            let alloca = builder.build_alloca(llvm_type, &name).unwrap();
            variables.insert(name.clone(), alloca);

            // Initialize the variable if an initial value is provided
            if let Some(Literal::Number(value)) = initial_value {
                let init_value = match llvm_type {
                    BasicTypeEnum::IntType(int_type) => {
                        int_type.const_int(*value as u64, false)
                    }
                    _ => panic!("Unsupported type for initialization"),
                };
                let _ = builder.build_store(alloca, init_value);
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
            let c_format_string = wave_format_to_c(&format);

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
                let value = generate_expression_ir(context, builder, arg, variables);
                printf_args.push(value.into()); // BasicValueEnum -> BasicMetadataValueEnum
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

            let cond_value = generate_expression_ir(context, builder, condition, variables);

            let then_block = context.append_basic_block(current_fn, "then");
            let else_block_bb = context.append_basic_block(current_fn, "else");
            let merge_block = context.append_basic_block(current_fn, "merge");

            let _ = builder.build_conditional_branch(cond_value, then_block, else_block_bb);

            // then
            builder.position_at_end(then_block);
            for stmt in body {
                generate_statement_ir(context, builder, module, string_counter, stmt, variables, loop_exit_stack);
            }
            let _ = builder.build_unconditional_branch(merge_block);

            // else
            builder.position_at_end(else_block_bb);

            if let Some(else_ifs) = else_if_blocks {
                for else_if in else_ifs.iter() {
                    generate_statement_ir(context, builder, module, string_counter, stmt, variables, loop_exit_stack);
                }
            }

            if let Some(else_body) = else_block {
                for stmt in else_body.iter() {
                    generate_statement_ir(context, builder, module, string_counter, stmt, variables, loop_exit_stack);
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

            let _ = builder.build_unconditional_branch(cond_block);
            builder.position_at_end(cond_block);

            let cond_val = generate_expression_ir(context, builder, condition, variables);
            let zero = cond_val.get_type().const_zero();
            let cond_bool = builder.build_int_compare(
                inkwell::IntPredicate::NE,
                cond_val,
                zero,
                "while_cond",
            ).unwrap();

            let _ = builder.build_conditional_branch(cond_bool, body_block, merge_block);

            builder.position_at_end(body_block);
            for stmt in body.iter() {
                generate_statement_ir(context, builder, module, string_counter, stmt, variables, loop_exit_stack);
            }
            let _ = builder.build_unconditional_branch(cond_block);

            loop_exit_stack.pop();

            builder.position_at_end(merge_block);
        }
        ASTNode::Statement(StatementNode::Assign { variable, value }) => {
            let val = generate_expression_ir(context, builder, value, variables);
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