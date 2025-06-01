use parser::ast::{ASTNode, FunctionNode, StatementNode, Expression, VariableNode, Literal, Operator, WaveType, Mutability, Value, AssignOperator};
use inkwell::context::Context;
use inkwell::module::Linkage;
use inkwell::values::{PointerValue, FunctionValue, BasicValue, BasicValueEnum, AnyValue};
use inkwell::{AddressSpace, FloatPredicate, IntPredicate};

use std::collections::HashMap;
use inkwell::basic_block::BasicBlock;
use inkwell::types::{AnyTypeEnum, BasicMetadataTypeEnum, BasicType, BasicTypeEnum};
use lexer::token::TokenType;

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
                        BasicTypeEnum::PointerType(ptr_ty) => {
                            if ptr_ty.get_element_type().is_int_type() && ptr_ty.get_element_type().into_int_type().get_bit_width() == 8 {
                                "%s"
                            } else {
                                "%ld"
                            }
                        },
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
    variables: &mut HashMap<String, VariableInfo<'ctx>>,
    module: &'ctx inkwell::module::Module<'ctx>,
    expected_type: Option<BasicTypeEnum<'ctx>>,
) -> BasicValueEnum<'ctx> {
    match expr {
        Expression::Literal(lit) => match lit {
            Literal::Number(v) => {
                match expected_type {
                    Some(BasicTypeEnum::IntType(int_ty)) => {
                        int_ty.const_int(*v as u64, false).as_basic_value_enum()
                    }
                    None => {
                        context.i64_type().const_int(*v as u64, false).as_basic_value_enum()
                    }
                    _ => panic!("Expected integer type for numeric literal, got {:?}", expected_type),
                }
            }
            Literal::Float(value) => {
                match expected_type {
                    Some(BasicTypeEnum::FloatType(float_ty)) => float_ty.const_float(*value).as_basic_value_enum(),
                    Some(BasicTypeEnum::IntType(int_ty)) => builder.build_float_to_signed_int(context.f32_type().const_float(*value), int_ty, "f32_to_int").unwrap().as_basic_value_enum(),
                    None => context.f32_type().const_float(*value).as_basic_value_enum(),
                    _ => panic!("Unsupported expected_type for float"),
                }
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
            if let Some(var_info) = variables.get(var_name) {
                builder.build_load(var_info.ptr, var_name).unwrap()
            } else if module.get_function(var_name).is_some() {
                panic!("Error: '{}' is a function name, not a variable", var_name);
            } else {
                panic!("variable '{}' not found in current scope", var_name);
            }
        }

        Expression::Deref(inner_expr) => {
            match &**inner_expr {
                Expression::Variable(var_name) => {
                    let ptr_to_value = variables.get(var_name).unwrap().ptr;
                    let actual_ptr = builder.build_load(ptr_to_value, "deref_target").unwrap().into_pointer_value();
                    builder.build_load(actual_ptr, "deref_load").unwrap().as_basic_value_enum()
                }
                _ => {
                    let ptr_val = generate_expression_ir(context, builder, inner_expr, variables, module, None);
                    let ptr = ptr_val.into_pointer_value();
                    builder.build_load(ptr, "deref_load").unwrap().as_basic_value_enum()
                }
            }
        }

        Expression::AddressOf(inner_expr) => {
            if let Some(BasicTypeEnum::PointerType(ptr_ty)) = expected_type {
                match &**inner_expr {
                    Expression::ArrayLiteral(elements) => unsafe {
                        let array_type = ptr_ty.get_element_type().into_array_type();
                        let elem_type = array_type.get_element_type();

                        let array_type = elem_type.array_type(elements.len() as u32);
                        let tmp_alloca = builder.build_alloca(array_type, "tmp_array").unwrap();

                        for (i, expr) in elements.iter().enumerate() {
                            let val = generate_expression_ir(
                                context,
                                builder,
                                expr,
                                variables,
                                module,
                                Some(elem_type),
                            );
                            let gep = builder.build_in_bounds_gep(
                                tmp_alloca,
                                &[
                                    context.i32_type().const_zero(),
                                    context.i32_type().const_int(i as u64, false),
                                ],
                                &format!("array_idx_{}", i),
                            ).unwrap();
                            builder.build_store(gep, val).unwrap();
                        }

                        let alloca = builder.build_alloca(tmp_alloca.get_type(), "tmp_array_ptr").unwrap();
                        builder.build_store(alloca, tmp_alloca).unwrap();
                        alloca.as_basic_value_enum()
                    }

                    Expression::Variable(var_name) => {
                        let ptr = variables.get(var_name)
                            .unwrap_or_else(|| panic!("Variable {} not found", var_name));
                        let alloca = builder.build_alloca(ptr.ptr.get_type(), "tmp_var_ptr").unwrap();
                        builder.build_store(alloca, ptr.ptr).unwrap();
                        alloca.as_basic_value_enum()
                    }

                    _ => panic!("& operator must be used on variable name or array literal"),
                }
            } else {
                panic!("Expected pointer type for AddressOf");
            }
        }

        Expression::FunctionCall { name, args } => {
            let function = module
                .get_function(name)
                .unwrap_or_else(|| panic!("Function '{}' not found", name));

            let function_type = function.get_type();
            let param_types: Vec<BasicTypeEnum> = function_type
                .get_param_types()
                .iter()
                .map(|t| t.clone().into())
                .collect();

            let mut compiled_args = vec![];
            for (i, arg) in args.iter().enumerate() {
                let expected = param_types.get(i).copied();
                let val = generate_expression_ir(context, builder, arg, variables, module, expected);
                compiled_args.push(val.into());
            }

            let call_site = builder.build_call(function, &compiled_args, "calltmp").unwrap();

            if function.get_type().get_return_type().is_some() {
                call_site.try_as_basic_value().left().unwrap()
            } else {
                context.i32_type().const_int(0, false).as_basic_value_enum()
            }
        }

        Expression::AssignOperation { target, operator, value } => {
            let ptr = generate_address_ir(context, builder, target, variables, module);

            let current_val = builder.build_load(ptr, "load_current").unwrap();

            let new_val = generate_expression_ir(context, builder, value, variables, module, Some(current_val.get_type()));

            let (current_val, new_val) = match (current_val, new_val) {
                (BasicValueEnum::FloatValue(lhs), BasicValueEnum::IntValue(rhs)) => {
                    let rhs_casted = builder.build_signed_int_to_float(rhs, lhs.get_type(), "int_to_float").unwrap();
                    (BasicValueEnum::FloatValue(lhs), BasicValueEnum::FloatValue(rhs_casted))
                }
                (BasicValueEnum::IntValue(lhs), BasicValueEnum::FloatValue(rhs)) => {
                    let lhs_casted = builder.build_signed_int_to_float(lhs, rhs.get_type(), "int_to_float").unwrap();
                    (BasicValueEnum::FloatValue(lhs_casted), BasicValueEnum::FloatValue(rhs))
                }
                other => other,
            };

            let result = match (current_val, new_val) {
                (BasicValueEnum::IntValue(lhs), BasicValueEnum::IntValue(rhs)) => {
                    match operator {
                        AssignOperator::AddAssign => builder.build_int_add(lhs, rhs, "add_assign").unwrap().as_basic_value_enum(),
                        AssignOperator::SubAssign => builder.build_int_sub(lhs, rhs, "sub_assign").unwrap().as_basic_value_enum(),
                        AssignOperator::MulAssign => builder.build_int_mul(lhs, rhs, "mul_assign").unwrap().as_basic_value_enum(),
                        AssignOperator::DivAssign => builder.build_int_signed_div(lhs, rhs, "div_assign").unwrap().as_basic_value_enum(),
                        AssignOperator::RemAssign => builder.build_int_signed_rem(lhs, rhs, "rem_assign").unwrap().as_basic_value_enum(),
                    }
                }
                (BasicValueEnum::FloatValue(lhs), BasicValueEnum::FloatValue(rhs)) => {
                    match operator {
                        AssignOperator::AddAssign => builder.build_float_add(lhs, rhs, "add_assign").unwrap().as_basic_value_enum(),
                        AssignOperator::SubAssign => builder.build_float_sub(lhs, rhs, "sub_assign").unwrap().as_basic_value_enum(),
                        AssignOperator::MulAssign => builder.build_float_mul(lhs, rhs, "mul_assign").unwrap().as_basic_value_enum(),
                        AssignOperator::DivAssign => builder.build_float_div(lhs, rhs, "div_assign").unwrap().as_basic_value_enum(),
                        AssignOperator::RemAssign => builder.build_float_rem(lhs, rhs, "rem_assign").unwrap().as_basic_value_enum(),
                    }
                }
                _ => panic!("Type mismatch or unsupported type in AssignOperation"),
            };

            let element_type = match ptr.get_type().get_element_type() {
                AnyTypeEnum::IntType(t) => BasicTypeEnum::IntType(t),
                AnyTypeEnum::FloatType(t) => BasicTypeEnum::FloatType(t),
                AnyTypeEnum::PointerType(t) => BasicTypeEnum::PointerType(t),
                AnyTypeEnum::ArrayType(t) => BasicTypeEnum::ArrayType(t),
                AnyTypeEnum::StructType(t) => BasicTypeEnum::StructType(t),
                AnyTypeEnum::VectorType(t) => BasicTypeEnum::VectorType(t),
                _ => panic!("Unsupported LLVM element type"),
            };

            let result_casted = match (result, element_type) {
                (BasicValueEnum::FloatValue(val), BasicTypeEnum::IntType(int_ty)) => {
                    builder.build_float_to_signed_int(val, int_ty, "float_to_int").unwrap().as_basic_value_enum()
                }
                (BasicValueEnum::IntValue(val), BasicTypeEnum::FloatType(float_ty)) => {
                    builder.build_signed_int_to_float(val, float_ty, "int_to_float").unwrap().as_basic_value_enum()
                }
                _ => result,
            };
            builder.build_store(ptr, result_casted).unwrap();

            builder.build_store(ptr, result_casted).unwrap();
            result
        }

        Expression::BinaryExpression { left, operator, right } => {
            let left_val = generate_expression_ir(context, builder, left, variables, module, None);
            let right_val = generate_expression_ir(context, builder, right, variables, module, None);

            // Branch after Type Examination
            match (left_val, right_val) {
                (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                    let l_type = l.get_type();
                    let r_type = r.get_type();

                    let (l_casted, r_casted) = if l_type != r_type {
                        if l_type.get_bit_width() < r_type.get_bit_width() {
                            let new_l = builder.build_int_z_extend(l, r_type, "zext_l").unwrap();
                            (new_l, r)
                        } else {
                            let new_r = builder.build_int_z_extend(r, l_type, "zext_r").unwrap();
                            (l, new_r)
                        }
                    } else {
                        (l, r)
                    };

                    let mut result = match operator {
                        Operator::Add => builder.build_int_add(l_casted, r_casted, "addtmp"),
                        Operator::Subtract => builder.build_int_sub(l_casted, r_casted, "subtmp"),
                        Operator::Multiply => builder.build_int_mul(l_casted, r_casted, "multmp"),
                        Operator::Divide => builder.build_int_signed_div(l_casted, r_casted, "divtmp"),
                        Operator::Remainder => builder.build_int_signed_rem(l_casted, r_casted, "modtmp"),
                        Operator::Greater => builder.build_int_compare(IntPredicate::SGT, l_casted, r_casted, "cmptmp"),
                        Operator::Less => builder.build_int_compare(IntPredicate::SLT, l_casted, r_casted, "cmptmp"),
                        Operator::Equal => builder.build_int_compare(IntPredicate::EQ, l_casted, r_casted, "cmptmp"),
                        Operator::NotEqual => builder.build_int_compare(IntPredicate::NE, l_casted, r_casted, "cmptmp"),
                        Operator::GreaterEqual => builder.build_int_compare(IntPredicate::SGE, l_casted, r_casted, "cmptmp"),
                        Operator::LessEqual => builder.build_int_compare(IntPredicate::SLE, l_casted, r_casted, "cmptmp"),
                        _ => panic!("Unsupported binary operator"),
                    }.unwrap();

                    if let Some(BasicTypeEnum::IntType(target_ty)) = expected_type {
                        let result_ty = result.get_type();

                        if result_ty != target_ty {
                            result = builder.build_int_cast(result, target_ty, "cast_result").unwrap();
                        }
                    }

                    result.as_basic_value_enum()
                }

                (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
                    match operator {
                        Operator::Greater => builder.build_float_compare(FloatPredicate::OGT, l, r, "fcmpgt").unwrap().as_basic_value_enum(),
                        Operator::Less => builder.build_float_compare(FloatPredicate::OLT, l, r, "fcmplt").unwrap().as_basic_value_enum(),
                        Operator::Equal => builder.build_float_compare(FloatPredicate::OEQ, l, r, "fcmpeq").unwrap().as_basic_value_enum(),
                        Operator::NotEqual => builder.build_float_compare(FloatPredicate::ONE, l, r, "fcmpne").unwrap().as_basic_value_enum(),
                        Operator::GreaterEqual => builder.build_float_compare(FloatPredicate::OGE, l, r, "fcmpge").unwrap().as_basic_value_enum(),
                        Operator::LessEqual => builder.build_float_compare(FloatPredicate::OLE, l, r, "fcmple").unwrap().as_basic_value_enum(),
                        Operator::Remainder => builder.build_float_rem(l, r, "modtmp").unwrap().as_basic_value_enum(),
                        _ => panic!("Unsupported float operator"),
                    }
                }

                (BasicValueEnum::IntValue(int_val), BasicValueEnum::FloatValue(float_val)) => {
                    let casted = builder.build_signed_int_to_float(int_val, float_val.get_type(), "cast_lhs").unwrap();
                    match operator {
                        Operator::Add => builder.build_float_add(casted, float_val, "addtmp").unwrap().as_basic_value_enum(),
                        Operator::Subtract => builder.build_float_sub(casted, float_val, "subtmp").unwrap().as_basic_value_enum(),
                        Operator::Multiply => builder.build_float_mul(casted, float_val, "multmp").unwrap().as_basic_value_enum(),
                        Operator::Divide => builder.build_float_div(casted, float_val, "divtmp").unwrap().as_basic_value_enum(),
                        Operator::Remainder => builder.build_float_rem(casted, float_val, "modtmp").unwrap().as_basic_value_enum(),
                        Operator::Greater => builder.build_float_compare(FloatPredicate::OGT, casted, float_val, "fcmpgt").unwrap().as_basic_value_enum(),
                        Operator::Less => builder.build_float_compare(FloatPredicate::OLT, casted, float_val, "fcmplt").unwrap().as_basic_value_enum(),
                        Operator::Equal => builder.build_float_compare(FloatPredicate::OEQ, casted, float_val, "fcmpeq").unwrap().as_basic_value_enum(),
                        Operator::NotEqual => builder.build_float_compare(FloatPredicate::ONE, casted, float_val, "fcmpne").unwrap().as_basic_value_enum(),
                        Operator::GreaterEqual => builder.build_float_compare(FloatPredicate::OGE, casted, float_val, "fcmpge").unwrap().as_basic_value_enum(),
                        Operator::LessEqual => builder.build_float_compare(FloatPredicate::OLE, casted, float_val, "fcmple").unwrap().as_basic_value_enum(),
                        _ => panic!("Unsupported mixed-type operator (int + float)"),
                    }
                }

                (BasicValueEnum::FloatValue(float_val), BasicValueEnum::IntValue(int_val)) => {
                    let casted = builder.build_signed_int_to_float(int_val, float_val.get_type(), "cast_rhs").unwrap();
                    match operator {
                        Operator::Add => builder.build_float_add(float_val, casted, "addtmp").unwrap().as_basic_value_enum(),
                        Operator::Subtract => builder.build_float_sub(float_val, casted, "subtmp").unwrap().as_basic_value_enum(),
                        Operator::Multiply => builder.build_float_mul(float_val, casted, "multmp").unwrap().as_basic_value_enum(),
                        Operator::Divide => builder.build_float_div(float_val, casted, "divtmp").unwrap().as_basic_value_enum(),
                        Operator::Remainder => builder.build_float_rem(float_val, casted, "modtmp").unwrap().as_basic_value_enum(),
                        Operator::Greater => builder.build_float_compare(FloatPredicate::OGT, float_val, casted, "fcmpgt").unwrap().as_basic_value_enum(),
                        Operator::Less => builder.build_float_compare(FloatPredicate::OLT, float_val, casted, "fcmplt").unwrap().as_basic_value_enum(),
                        Operator::Equal => builder.build_float_compare(FloatPredicate::OEQ, float_val, casted, "fcmpeq").unwrap().as_basic_value_enum(),
                        Operator::NotEqual => builder.build_float_compare(FloatPredicate::ONE, float_val, casted, "fcmpne").unwrap().as_basic_value_enum(),
                        Operator::GreaterEqual => builder.build_float_compare(FloatPredicate::OGE, float_val, casted, "fcmpge").unwrap().as_basic_value_enum(),
                        Operator::LessEqual => builder.build_float_compare(FloatPredicate::OLE, float_val, casted, "fcmple").unwrap().as_basic_value_enum(),
                        _ => panic!("Unsupported mixed-type operator (float + int)"),
                    }
                }

                _ => panic!("Type mismatch in binary expression"),
            }
        }

        Expression::IndexAccess { target, index } => unsafe {
            let target_val = generate_expression_ir(context, builder, target, variables, module, None);

            let index_val = generate_expression_ir(context, builder, index, variables, module, None);
            let index_int = match index_val {
                BasicValueEnum::IntValue(i) => i,
                _ => panic!("Index must be an integer"),
            };

            let zero = context.i32_type().const_zero();

            match target_val {
                BasicValueEnum::PointerValue(ptr_val) => {
                    let element_type = ptr_val.get_type().get_element_type();

                    if element_type.is_array_type() {
                        let gep = builder.build_in_bounds_gep(
                            ptr_val,
                            &[zero, index_int],
                            "array_index_gep",
                        ).unwrap();
                        builder.build_load(gep, "load_array_elem").unwrap().as_basic_value_enum()
                    }

                    else {
                        let gep = builder.build_in_bounds_gep(
                            ptr_val,
                            &[index_int],
                            "ptr_index_gep",
                        ).unwrap();
                        builder.build_load(gep, "load_ptr_elem").unwrap().as_basic_value_enum()
                    }
                }

                _ => panic!("Unsupported target in IndexAccess"),
            }
        }

        Expression::AsmBlock { instructions, inputs, outputs } => {
            use inkwell::InlineAsmDialect;
            use inkwell::values::{BasicMetadataValueEnum, CallableValue};
            use std::collections::HashSet;

            let asm_code: String = instructions.join("\n");

            let mut operand_vals: Vec<BasicMetadataValueEnum> = vec![];
            let mut constraint_parts: Vec<String> = vec![];

            let input_regs: HashSet<_> = inputs.iter().map(|(reg, _)| reg.to_string()).collect();
            let mut seen_regs: HashSet<String> = HashSet::new();

            for (reg, var) in outputs {
                if input_regs.contains(reg) {
                    panic!("Register '{}' used in both input and output in inline asm", reg);
                }

                if !seen_regs.insert(reg.to_string()) {
                    panic!("Register '{}' duplicated in outputs", reg);
                }

                let info = variables
                    .get(var)
                    .unwrap_or_else(|| panic!("Output variable '{}' not found", var));
                let dummy_val = builder.build_load(info.ptr, var).unwrap().into();
                operand_vals.push(dummy_val);
                constraint_parts.push(format!("={{{}}}", reg));
            }

            for (reg, var) in inputs {
                if !seen_regs.insert(reg.to_string()) {
                    panic!("Register '{}' duplicated in inputs", reg);
                }

                let val: BasicMetadataValueEnum = if let Ok(value) = var.parse::<i64>() {
                    context.i64_type().const_int(value as u64, false).into()
                } else {
                    let info = variables
                        .get(var)
                        .unwrap_or_else(|| panic!("Input variable '{}' not found", var));
                    builder.build_load(info.ptr, var).unwrap().into()
                };

                operand_vals.push(val);
                constraint_parts.push(format!("{{{}}}", reg));
            }

            let constraints_str = constraint_parts.join(",");

            let fn_type = if outputs.is_empty() {
                context.void_type().fn_type(&[], false)
            } else {
                context.i64_type().fn_type(&[], false)
            };

            let inline_asm_ptr = context.create_inline_asm(
                fn_type,
                asm_code,
                constraints_str,
                true,
                false,
                Some(InlineAsmDialect::Intel),
                false,
            );

            let inline_asm_fn = CallableValue::try_from(inline_asm_ptr)
                .expect("Failed to convert inline asm to CallableValue");

            let call = builder
                .build_call(inline_asm_fn, &operand_vals, "inline_asm_expr")
                .unwrap();

            call.try_as_basic_value().left().unwrap()
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

#[derive(Clone)]
pub struct VariableInfo<'ctx> {
    pub ptr: PointerValue<'ctx>,
    pub mutability: Mutability,
}

fn generate_statement_ir<'ctx>(
    context: &'ctx Context,
    builder: &'ctx inkwell::builder::Builder<'ctx>,
    module: &'ctx inkwell::module::Module<'ctx>,
    string_counter: &mut usize,
    stmt: &ASTNode,
    variables: &mut HashMap<String, VariableInfo<'ctx>>,
    loop_exit_stack: &mut Vec<BasicBlock<'ctx>>,
    loop_continue_stack: &mut Vec<BasicBlock<'ctx>>,
    current_function: FunctionValue<'ctx>,
) {
    match stmt {
        ASTNode::Variable(VariableNode {
                              name,
                              type_name,
                              initial_value,
                              mutability
                          }) => unsafe {
            let llvm_type = wave_type_to_llvm_type(&context, &type_name);
            let alloca = builder.build_alloca(llvm_type, &name).unwrap();

            if let (WaveType::Array(element_type, size), Some(Expression::ArrayLiteral(values))) = (&type_name, &initial_value) {
                if values.len() != *size as usize {
                    panic!(
                        "❌ Array length mismatch: expected {}, got {}",
                        size,
                        values.len()
                    );
                }

                let llvm_element_type = wave_type_to_llvm_type(context, element_type);

                for (i, value_expr) in values.iter().enumerate() {
                    let value = generate_expression_ir(context, builder, value_expr, variables, module, Some(llvm_element_type));

                    let gep = builder.build_in_bounds_gep(
                        alloca,
                        &[
                            context.i32_type().const_zero(),
                            context.i32_type().const_int(i as u64, false),
                        ],
                        &format!("array_idx_{}", i),
                    ).unwrap();

                    builder.build_store(gep, value).unwrap();
                }

                variables.insert(
                    name.clone(),
                    VariableInfo {
                        ptr: alloca,
                        mutability: mutability.clone(),
                    },
                );

                return;
            }

            variables.insert(
                name.clone(),
                VariableInfo {
                    ptr: alloca,
                    mutability: mutability.clone(),
                },
            );

            if let Some(init) = initial_value {
                match (init, llvm_type) {
                    (Expression::Literal(Literal::Number(value)), BasicTypeEnum::IntType(int_type)) => {
                        let init_value = int_type.const_int(*value as u64, false);
                        let _ = builder.build_store(alloca, init_value);
                    }
                    (Expression::Literal(Literal::Float(value)), BasicTypeEnum::FloatType(float_type)) => {
                        let init_value = float_type.const_float(*value);
                        builder.build_store(alloca, init_value).unwrap();
                    }
                    (Expression::Literal(Literal::Float(value)), _) => {
                        let float_value = context.f32_type().const_float(*value);

                        let casted_value = match llvm_type {
                            BasicTypeEnum::IntType(int_ty) => {
                                builder.build_float_to_signed_int(float_value, int_ty, "float_to_int").unwrap().as_basic_value_enum()
                            }
                            BasicTypeEnum::FloatType(_) => float_value.as_basic_value_enum(),
                            _ => panic!("Unsupported type for float literal initialization"),
                        };

                        builder.build_store(alloca, casted_value).unwrap();
                    }
                    (Expression::Literal(Literal::String(value)), BasicTypeEnum::PointerType(_)) => unsafe {
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
                        let gep = builder.build_gep(global.as_pointer_value(), &indices, "str_gep").unwrap();

                        let _ = builder.build_store(alloca, gep);
                    }
                    (Expression::AddressOf(inner_expr), BasicTypeEnum::PointerType(_)) => {
                        match &**inner_expr {
                            Expression::Variable(var_name) => {
                                let ptr = variables.get(var_name)
                                    .unwrap_or_else(|| panic!("Variable {} not found", var_name));
                                builder.build_store(alloca, ptr.ptr).unwrap();
                            }
                            Expression::ArrayLiteral(elements) => {
                                let elem_type = match llvm_type {
                                    BasicTypeEnum::PointerType(ptr_ty) => {
                                        match ptr_ty.get_element_type() {
                                            AnyTypeEnum::ArrayType(arr_ty) => arr_ty.get_element_type(),
                                            _ => panic!("Expected pointer to array type"),
                                        }
                                    }
                                    _ => panic!("Expected pointer-to-array type for array literal"),
                                };

                                let array_type = elem_type.array_type(elements.len() as u32);
                                let tmp_alloca = builder.build_alloca(array_type, "tmp_array").unwrap();

                                for (i, expr) in elements.iter().enumerate() {
                                    let val = generate_expression_ir(
                                        context,
                                        builder,
                                        expr,
                                        variables,
                                        module,
                                        Some(elem_type),
                                    );
                                    let gep = builder.build_in_bounds_gep(
                                        tmp_alloca,
                                        &[
                                            context.i32_type().const_zero(),
                                            context.i32_type().const_int(i as u64, false),
                                        ],
                                        &format!("array_idx_{}", i),
                                    ).unwrap();
                                    builder.build_store(gep, val).unwrap();
                                }

                                builder.build_store(alloca, tmp_alloca).unwrap();
                            }
                            _ => panic!("& operator must be used on variable name or array literal"),
                        }
                    }
                    (Expression::Deref(inner_expr), BasicTypeEnum::IntType(int_type)) => {
                        let target_ptr = match &**inner_expr {
                            Expression::Variable(var_name) => {
                                let ptr_to_value = variables.get(var_name).unwrap().ptr;
                                builder.build_load(ptr_to_value, "load_ptr").unwrap().into_pointer_value()
                            }
                            _ => panic!("Invalid deref in variable init"),
                        };

                        let val = builder.build_load(target_ptr, "deref_value").unwrap();
                        let _ = builder.build_store(alloca, val);
                    }
                    (Expression::IndexAccess { target, index }, _) => {
                        let val = generate_expression_ir(context, builder, init, variables, module, Some(llvm_type));
                        builder.build_store(alloca, val).unwrap();
                    }
                    (Expression::FunctionCall { .. }, _) => {
                        let val = generate_expression_ir(context, builder, init, variables, module, Some(llvm_type));
                        builder.build_store(alloca, val).unwrap();
                    }
                    (Expression::BinaryExpression { .. }, _) => {
                        let val = generate_expression_ir(context, builder, init, variables, module, Some(llvm_type));

                        let casted_val = match (val, llvm_type) {
                            (BasicValueEnum::FloatValue(v), BasicTypeEnum::IntType(t)) => {
                                builder.build_float_to_signed_int(v, t, "float_to_int").unwrap().as_basic_value_enum()
                            }
                            (BasicValueEnum::IntValue(v), BasicTypeEnum::FloatType(t)) => {
                                builder.build_signed_int_to_float(v, t, "int_to_float").unwrap().as_basic_value_enum()
                            }
                            _ => val,
                        };

                        builder.build_store(alloca, casted_val).unwrap();
                    }
                    (Expression::Variable(var_name), _) => {
                        let source_var = variables.get(var_name)
                            .unwrap_or_else(|| panic!("Variable {} not found", var_name));

                        let loaded_value = builder
                            .build_load(source_var.ptr, &format!("load_{}", var_name))
                            .unwrap();

                        let loaded_type = loaded_value.get_type();

                        let casted_value = match (loaded_type, llvm_type) {
                            (BasicTypeEnum::IntType(_), BasicTypeEnum::FloatType(float_ty)) => {
                                builder.build_signed_int_to_float(
                                    loaded_value.into_int_value(),
                                    float_ty,
                                    "int_to_float",
                                ).unwrap().as_basic_value_enum()
                            }
                            (BasicTypeEnum::FloatType(_), BasicTypeEnum::IntType(int_ty)) => {
                                builder.build_float_to_signed_int(
                                    loaded_value.into_float_value(),
                                    int_ty,
                                    "float_to_int",
                                ).unwrap().as_basic_value_enum()
                            }
                            _ => loaded_value,
                        };

                        builder.build_store(alloca, casted_value).unwrap();
                    }
                    (Expression::AsmBlock { instructions, inputs, outputs }, BasicTypeEnum::IntType(int_type)) => {
                        use inkwell::InlineAsmDialect;
                        use inkwell::values::{BasicMetadataValueEnum, CallableValue};

                        let asm_code: String = instructions.join("\n");
                        let mut operand_vals: Vec<BasicMetadataValueEnum> = vec![];
                        let mut constraint_parts = vec![];

                        for (reg, var) in inputs {
                            let val = if let Ok(num) = var.parse::<i64>() {
                                context.i64_type().const_int(num as u64, false).into()
                            } else if let Some(info) = variables.get(var) {
                                builder.build_load(info.ptr, var).unwrap().into()
                            } else {
                                panic!("Input variable '{}' not found", var);
                            };

                            operand_vals.push(val);
                            constraint_parts.push(format!("{{{}}}", reg));
                        }

                        for (reg, _) in outputs {
                            constraint_parts.insert(0, format!("={{{}}}", reg));
                        }

                        let constraint_str = constraint_parts.join(",");

                        let (fn_type, expects_return) = if outputs.is_empty() {
                            (context.void_type().fn_type(&[], false), false)
                        } else {
                            (context.i64_type().fn_type(&[], false), true)
                        };

                        let inline_asm_ptr = context.create_inline_asm(
                            fn_type,
                            asm_code,
                            constraint_str,
                            true,  // has_side_effects
                            false, // align_stack
                            Some(InlineAsmDialect::Intel),
                            false, // can_throw
                        );

                        let inline_asm_fn = CallableValue::try_from(inline_asm_ptr)
                            .expect("Failed to cast inline asm to CallableValue");

                        let call = builder
                            .build_call(inline_asm_fn, &operand_vals, "inline_asm")
                            .unwrap();

                        if expects_return {
                            let result = call.try_as_basic_value().left()
                                .expect("Expected return value from inline asm but got none");

                            builder.build_store(alloca, result).unwrap();
                        }
                    }
                    _ => {
                        panic!("Unsupported type/value combination for initialization: {:?}", init);
                    }
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
                let val = generate_expression_ir(context, builder, arg, variables, module, None);
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
                let value = generate_expression_ir(context, builder, arg, variables, module, None);

                let casted_value = match value {
                    BasicValueEnum::PointerValue(ptr_val) => {
                        let element_ty = ptr_val.get_type().get_element_type();
                        if element_ty.is_int_type() && element_ty.into_int_type().get_bit_width() == 8 {
                            ptr_val.as_basic_value_enum()
                        } else {
                            builder
                                .build_ptr_to_int(ptr_val, context.i64_type(), "ptr_as_int")
                                .unwrap()
                                .as_basic_value_enum()
                        }
                    }
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

            let cond_value = generate_expression_ir(context, builder, condition, variables, module, None);

            let then_block = context.append_basic_block(current_fn, "then");
            let else_block_bb = context.append_basic_block(current_fn, "else");
            let merge_block = context.append_basic_block(current_fn, "merge");

            let _ = builder.build_conditional_branch(cond_value.try_into().unwrap(), then_block, else_block_bb);

            // then
            builder.position_at_end(then_block);
            for stmt in body {
                generate_statement_ir(context, builder, module, string_counter, stmt, variables, loop_exit_stack, loop_continue_stack, current_function);
            }
            let _ = builder.build_unconditional_branch(merge_block);

            // else
            builder.position_at_end(else_block_bb);

            if let Some(else_ifs) = else_if_blocks {
                for else_if in else_ifs.iter() {
                    generate_statement_ir(context, builder, module, string_counter, else_if, variables, loop_exit_stack, loop_continue_stack, current_function);
                }
            }

            if let Some(else_body) = else_block {
                for stmt in else_body.iter() {
                    generate_statement_ir(context, builder, module, string_counter, stmt, variables, loop_exit_stack, loop_continue_stack, current_function);
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

            let cond_val = generate_expression_ir(context, builder, condition, variables, module, None);

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
                generate_statement_ir(context, builder, module, string_counter, stmt, variables, loop_exit_stack, loop_continue_stack, current_function);
            }
            let _ = builder.build_unconditional_branch(cond_block);

            loop_exit_stack.pop();
            loop_continue_stack.pop();

            builder.position_at_end(merge_block);
        }
        ASTNode::Statement(StatementNode::AsmBlock { instructions, inputs, outputs }) => {
            use inkwell::InlineAsmDialect;
            use inkwell::values::{BasicMetadataValueEnum, CallableValue};

            let asm_code: String = instructions.join("\n");
            let mut operand_vals: Vec<BasicMetadataValueEnum> = vec![];
            let mut constraint_parts: Vec<String> = vec![];

            for (reg, var) in outputs {
                if input_regs.contains(reg) {
                    panic!("Register '{}' used in both input and output in inline asm", reg);
                }

                if !seen_regs.insert(reg.to_string()) {
                    panic!("Register '{}' duplicated in outputs", reg);
                }

                let info = variables
                    .get(var)
                    .unwrap_or_else(|| panic!("Output variable '{}' not found", var));
                let dummy_val = builder.build_load(info.ptr, var).unwrap().into();
                operand_vals.push(dummy_val);
                constraint_parts.push(format!("={{{}}}", reg)); // e.g., ={rax}
            }

            for (reg, var) in inputs {
                if !seen_regs.insert(reg.to_string()) {
                    panic!("Register '{}' duplicated in inputs", reg);
                }

                let val: BasicMetadataValueEnum = if let Ok(value) = var.parse::<i64>() {
                    context.i64_type().const_int(value as u64, false).into()
                } else {
                    let info = variables
                        .get(var)
                        .unwrap_or_else(|| panic!("Input variable '{}' not found", var));
                    builder.build_load(info.ptr, var).unwrap().into()
                };

                operand_vals.push(val);
                constraint_parts.push(format!("{{{}}}", reg));
            }

            let constraints_str: String = constraint_parts.join(",");

            let fn_type = context.i64_type().fn_type(&[], false);

            let inline_asm_ptr = context.create_inline_asm(
                fn_type,
                asm_code,
                constraints_str,
                true,
                false,
                Some(InlineAsmDialect::Intel),
                false,
            );

            let inline_asm_fn = CallableValue::try_from(inline_asm_ptr)
                .expect("Failed to convert inline asm to CallableValue");

            let call = builder
                .build_call(inline_asm_fn, &operand_vals, "inline_asm")
                .unwrap();

            if let Some((_, out_var)) = outputs.first() {
                let ret_ptr = variables.get(out_var).unwrap().ptr;
                let ret_val = call.try_as_basic_value().left().unwrap();
                builder.build_store(ret_ptr, ret_val).unwrap();
            }
        }
        ASTNode::Statement(StatementNode::Expression(expr)) => {
            let _ = generate_expression_ir(context, builder, expr, variables, module, None);
        }
        ASTNode::Statement(StatementNode::Assign { variable, value }) => {
            if variable == "deref" {
                if let Expression::BinaryExpression { left, operator: _, right } = value {
                    if let Expression::Deref(inner_expr) = &**left {
                        let target_ptr = generate_address_ir(context, builder, inner_expr, variables, module);
                        let val = generate_expression_ir(context, builder, right, variables, module, None);
                        builder.build_store(target_ptr, val).unwrap();
                    }
                }
                return;
            }

            let var_info = variables.get(variable)
                .unwrap_or_else(|| panic!("Variable {} not declared", variable));

            if matches!(var_info.mutability, Mutability::Let) {
                panic!("Cannot assign to immutable variable '{}'", variable);
            }

            let element_type = var_info.ptr.get_type().get_element_type();

            let expected_type = match element_type {
                AnyTypeEnum::IntType(t) => t.as_basic_type_enum(),
                AnyTypeEnum::FloatType(t) => t.as_basic_type_enum(),
                AnyTypeEnum::PointerType(t) => t.as_basic_type_enum(),
                AnyTypeEnum::ArrayType(t) => t.as_basic_type_enum(),
                AnyTypeEnum::StructType(t) => t.as_basic_type_enum(),
                AnyTypeEnum::VectorType(t) => t.as_basic_type_enum(),
                _ => panic!("Unsupported LLVM type in assignment"),
            };

            let val = generate_expression_ir(context, builder, value, variables, module, Some(expected_type));

            if let Some(var_info) = variables.get(variable) {
                if matches!(var_info.mutability, Mutability::Let) {
                    panic!("Cannot assign to immutable variable '{}'", variable);
                }

                let element_type = match var_info.ptr.get_type().get_element_type() {
                    AnyTypeEnum::IntType(t) => BasicTypeEnum::IntType(t),
                    AnyTypeEnum::FloatType(t) => BasicTypeEnum::FloatType(t),
                    AnyTypeEnum::PointerType(t) => BasicTypeEnum::PointerType(t),
                    AnyTypeEnum::ArrayType(t) => BasicTypeEnum::ArrayType(t),
                    AnyTypeEnum::StructType(t) => BasicTypeEnum::StructType(t),
                    AnyTypeEnum::VectorType(t) => BasicTypeEnum::VectorType(t),
                    _ => panic!("Unsupported LLVM type in assignment"),
                };

                let casted_val = match (val, element_type) {
                    (BasicValueEnum::FloatValue(v), BasicTypeEnum::IntType(t)) => {
                        builder.build_float_to_signed_int(v, t, "float_to_int").unwrap().as_basic_value_enum()
                    }
                    (BasicValueEnum::IntValue(v), BasicTypeEnum::FloatType(t)) => {
                        builder.build_signed_int_to_float(v, t, "int_to_float").unwrap().as_basic_value_enum()
                    }
                    _ => val,
                };
                builder.build_store(var_info.ptr, casted_val).unwrap();
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
                let ret_type = current_function.get_type().get_return_type()
                    .expect("Function should have a return type");
                let expected_type = ret_type.try_into()
                    .expect("Failed to convert return type to BasicTypeEnum");

                let value = generate_expression_ir(
                    context,
                    builder,
                    expr,
                    variables,
                    module,
                    Some(expected_type),
                );

                let casted_value = match (value, expected_type) {
                    (BasicValueEnum::FloatValue(v), BasicTypeEnum::IntType(t)) => {
                        builder.build_float_to_signed_int(v, t, "float_to_int").unwrap().as_basic_value_enum()
                    }
                    (BasicValueEnum::IntValue(v), BasicTypeEnum::FloatType(t)) => {
                        builder.build_signed_int_to_float(v, t, "int_to_float").unwrap().as_basic_value_enum()
                    }
                    _ => value,
                };

                let _ = builder.build_return(Some(&casted_value));
            } else {
                let _ = builder.build_return(None);
            }
        }
        _ => {}
    }
}

fn generate_address_ir<'ctx>(
    context: &'ctx Context,
    builder: &'ctx inkwell::builder::Builder<'ctx>,
    expr: &Expression,
    variables: &mut HashMap<String, VariableInfo<'ctx>>,
    module: &'ctx inkwell::module::Module<'ctx>,
) -> PointerValue<'ctx> {
    match expr {
        Expression::Variable(name) => {
            let var_info = variables.get(name)
                .unwrap_or_else(|| panic!("Variable {} not found", name));

            var_info.ptr
        }

        Expression::Deref(inner_expr) => {
            match &**inner_expr {
                Expression::Variable(var_name) => {
                    let ptr_to_ptr = variables.get(var_name)
                        .unwrap_or_else(|| panic!("Variable {} not found", var_name))
                        .ptr;

                    let actual_ptr = builder.build_load(ptr_to_ptr, "deref_target").unwrap();
                    actual_ptr.into_pointer_value()
                }
                _ => panic!("Nested deref not supported"),
            }
        }

        _ => panic!("Cannot take address of this expression"),
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