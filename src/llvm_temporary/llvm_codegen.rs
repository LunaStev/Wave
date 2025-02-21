use crate::parser::ast::{ASTNode, FunctionNode, StatementNode};

use inkwell::context::Context;
use inkwell::module::Linkage;
use inkwell::values::PointerValue;
use inkwell::AddressSpace;

use std::collections::HashMap;

pub unsafe fn generate_ir(ast: &ASTNode) -> String {
    let context = Context::create();
    let module = context.create_module("main");
    let builder = context.create_builder();

    // HashMap to store variables
    let mut variables: HashMap<String, PointerValue> = HashMap::new();

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
                    // Create variable alloca
                    let alloca = builder.build_alloca(context.i32_type(), &name).unwrap();
                    variables.insert(name.clone(), alloca);

                    // Initializing Variables
                    if let Some(Literal::Number(value)) = initial_value {
                        let init_value = context.i32_type().const_int(*value as u64, false);
                        let _ = builder.build_store(alloca, init_value);
                    }
                }
                ASTNode::Statement(StatementNode::Println { format, args }) |
                ASTNode::Statement(StatementNode::Print { format, args })=> {
                    // Convert '{}' to '%d' in format string
                    let format = format.replace("{}", "%d");

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
                        let value = match arg {
                            Expression::Variable(var_name) => {
                                // Find the alloca of the variable and load the value
                                if let Some(alloca) = variables.get(var_name) {
                                    builder.build_load(*alloca, var_name).unwrap().into_int_value()
                                } else {
                                    panic!("Variable {} not found", var_name);
                                }
                            }
                            Expression::Literal(literal) => {
                                match literal {
                                    Literal::Number(value) => {
                                        context.i32_type().const_int(*value as u64, false)
                                    }
                                    _ => unimplemented!("Unsupported literal type"),
                                }
                            }
                            _ => unimplemented!("Unsupported expression type"),
                        };
                        printf_args.push(value.into());
                    }

                    // Call printf
                    let _ = builder.build_call(printf_func, &printf_args, "printf_call");
                }
                _ => {}
            }
        }

        // Add void return
        let _ = builder.build_return(None);
    }

    module.print_to_string().to_string()
}

unsafe fn create_alloca<'a>(
    context: &'a Context,
    builder: &'a inkwell::builder::Builder<'a>,
    function: FunctionValue<'a>,
    name: &'a str,
) -> PointerValue<'a> {
    let alloca = builder.build_alloca(context.i32_type(), name).unwrap();
    alloca
}