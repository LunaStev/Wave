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

        for stmt in body {
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
                    // When only the string is printed, it is treated like printf("%s", message)

                    let global_name = format!("str_{}_{}", name, string_counter);
                    string_counter += 1;

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
                        true
                    );
                    let printf_func = match module.get_function("printf") {
                        Some(func) => func,
                        None => module.add_function("printf", printf_type, None),
                    };

                    let zero = context.i32_type().const_zero();
                    let indices = [zero, zero];
                    let gep = builder.build_gep(global.as_pointer_value(), &indices, "gep").unwrap();

                    let _ = builder.build_call(printf_func, &[gep.into()], "printf_call");
                }
                ASTNode::Statement(StatementNode::PrintlnFormat { format, args })|
                ASTNode::Statement(StatementNode::PrintFormat { format, args }) => {
                    // Determine the format string based on the type of the first argument
                    let format = wave_format_to_c(&format);

                    // Generate unique global name for the format string
                    let global_name = format!("str_{}_{}", name, string_counter);
                    string_counter += 1;

                    // Create null-terminated string
                    let mut bytes = format.as_bytes().to_vec();
                    bytes.push(0);
                    let const_str = context.const_string(&bytes, false);

                    // Create global variable for the format string
                    let global = module.add_global(
                        context.i8_type().array_type(bytes.len() as u32),
                        None,
                        &global_name,
                    );
                    global.set_initializer(&const_str);
                    global.set_linkage(Linkage::Private);
                    global.set_constant(true);

                    // Get printf function
                    let printf_type = context.i32_type().fn_type(
                        &[context.i8_type().ptr_type(AddressSpace::default()).into()],
                        true
                    );
                    let printf_func = match module.get_function("printf") {
                        Some(func) => func,
                        None => module.add_function("printf", printf_type, None),
                    };

                    // Create GEP to get i8* pointer to the format string
                    let zero = context.i32_type().const_zero();
                    let indices = [zero, zero];
                    let gep = builder.build_gep(
                        global.as_pointer_value(),
                        &indices,
                        "gep",
                    ).unwrap();

                    // Prepare arguments for printf
                    let mut printf_args = vec![gep.into()];

                    // Add additional arguments
                    for arg in args {
                        let value = generate_expression_ir(&context, &builder, arg, &mut variables);
                        printf_args.push(value.into());
                    }

                    // Call printf
                    let _ = builder.build_call(printf_func, &printf_args, "printf_call");
                }
                ASTNode::Statement(StatementNode::If {
                                       condition,
                                       body,
                                       else_if_blocks,
                                       else_block,
                                   }) => {
                    let mut blocks = Vec::new();

                    // if 본문
                    let condition_value = generate_expression_ir(&context, &builder, condition, &mut variables);
                    let zero = condition_value.get_type().const_zero();
                    let condition_bool = builder.build_int_compare(
                        inkwell::IntPredicate::NE,
                        condition_value,
                        zero,
                        "condition_cmp"
                    ).unwrap();
                    let then_block = context.append_basic_block(function, "if_then");
                    blocks.push((condition_bool, then_block, body));

                    // else if Blocks (Option<Box<Vec<ASTNode>>>)
                    if let Some(else_ifs) = else_if_blocks {
                        for else_if in else_ifs.iter() {
                            if let ASTNode::Statement(StatementNode::If { condition, body, .. }) = else_if {
                                let cond_val_raw = generate_expression_ir(&context, &builder, condition, &mut variables);
                                let zero = cond_val_raw.get_type().const_zero();
                                let cond_val = builder.build_int_compare(
                                    inkwell::IntPredicate::NE,
                                    cond_val_raw,
                                    zero,
                                    "else_if_cmp"
                                ).unwrap();

                                let block = context.append_basic_block(function, "else_if_then");
                                blocks.push((cond_val, block, body));
                            }
                        }
                    }

                    // else 블록
                    let else_block_ir = else_block.as_ref().map(|body| {
                        let block = context.append_basic_block(function, "else_block");
                        (block, body)
                    });

                    // merge block
                    let merge_block = context.append_basic_block(function, "if_merge");

                    // Create a branch
                    for (i, (cond_value, then_block, body)) in blocks.iter().enumerate() {
                        // Create a conditional branch at the current location
                        let next_cond_block = if i + 1 < blocks.len() {
                            context.append_basic_block(function, &format!("cond_{}", i + 1))
                        } else if let Some((else_block, _)) = &else_block_ir {
                            *else_block
                        } else {
                            merge_block
                        };

                        builder.build_conditional_branch(*cond_value, *then_block, next_cond_block);

                        // Run then block
                        builder.position_at_end(*then_block);
                        for stmt in *body {
                            generate_statement_ir(&context, &builder, stmt, &mut variables);
                        }
                        builder.build_unconditional_branch(merge_block);
                    }

                    // else block
                    if let Some((else_block, else_body)) = else_block_ir {
                        builder.position_at_end(else_block);
                        for stmt in else_body.iter() {
                            generate_statement_ir(&context, &builder, stmt, &mut variables);
                        }
                        builder.build_unconditional_branch(merge_block);
                    }

                    // Move position to merge block at the end
                    builder.position_at_end(merge_block);
                }
                ASTNode::Statement(StatementNode::While { condition, body }) => {
                    // Generate IR for while loop
                    let condition_block = context.append_basic_block(function, "while.cond");
                    let body_block = context.append_basic_block(function, "while.body");
                    let merge_block = context.append_basic_block(function, "while.merge");

                    let _ = builder.build_unconditional_branch(condition_block);

                    // Generate condition block
                    builder.position_at_end(condition_block);
                    let condition_value = generate_expression_ir(&context, &builder, condition, &mut variables);
                    let _ = builder.build_conditional_branch(condition_value, body_block, merge_block);

                    // Generate body block
                    builder.position_at_end(body_block);
                    for stmt in body {
                        generate_statement_ir(&context, &builder, stmt, &mut variables);
                    }
                    let _ = builder.build_unconditional_branch(condition_block);

                    // Position builder at merge block
                    builder.position_at_end(merge_block);
                }
                /*
                ASTNode::Statement(StatementNode::For { initialization, condition, increment, body }) => {
                    // Generate IR for for loop
                    let init_block = context.append_basic_block(function, "for.init");
                    let condition_block = context.append_basic_block(function, "for.cond");
                    let body_block = context.append_basic_block(function, "for.body");
                    let increment_block = context.append_basic_block(function, "for.inc");
                    let merge_block = context.append_basic_block(function, "for.merge");

                    // Generate initialization block
                    builder.position_at_end(init_block);
                    generate_expression_ir(&context, &builder, initialization, &mut variables);
                    builder.build_unconditional_branch(condition_block);

                    // Generate condition block
                    builder.position_at_end(condition_block);
                    let condition_value = generate_expression_ir(&context, &builder, condition, &mut variables);
                    builder.build_conditional_branch(condition_value, body_block, merge_block);

                    // Generate body block
                    builder.position_at_end(body_block);
                    for stmt in body {
                        generate_statement_ir(&context, &builder, stmt, &mut variables);
                    }
                    let _ = builder.build_unconditional_branch(increment_block);

                    // Generate increment block
                    builder.position_at_end(increment_block);
                    generate_expression_ir(&context, &builder, increment, &mut variables);
                    let _ = builder.build_unconditional_branch(condition_block);

                    // Position builder at merge block
                    builder.position_at_end(merge_block);
                }
                 */
                _ => {}
            }
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
    variables: &mut HashMap<String, PointerValue<'a>>,
) -> inkwell::values::IntValue<'a> {
    match expr {
        Expression::Literal(Literal::Number(value)) => {
            context.i32_type().const_int(*value as u64, false)
        }
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

fn generate_statement_ir<'a>(
    context: &'a Context,
    builder: &'a inkwell::builder::Builder<'a>,
    stmt: &ASTNode,
    variables: &mut HashMap<String, PointerValue<'a>>,
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