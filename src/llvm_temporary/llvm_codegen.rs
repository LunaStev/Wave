use crate::parser::ast::{ASTNode, FunctionNode, StatementNode};

pub fn generate_ir(ast: &ASTNode) -> String {
    let mut ir = String::new();

    match ast {
        ASTNode::Function(FunctionNode { name, parameters, body }) => {
            ir.push_str(&format!("define void @{}() {{\n", name));
            ir.push_str("entry:\n");

        // Create entry block
        let entry_block = context.append_basic_block(function, "entry");
        builder.position_at_end(entry_block);

        let mut string_counter = 0;

        for stmt in body {
            if let ASTNode::Statement(StatementNode::Println(message)) = stmt {
                // Generate unique global name
                let global_name = format!("str_{}_{}", name, string_counter);
                string_counter += 1;

                // Create null-terminated string
                let mut bytes = message.as_bytes().to_vec();
                bytes.push(0);
                let const_str = context.const_string(&bytes, false);

                // Create global variable
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
                let printf_func = module.add_function("printf", printf_type, None);

                // Create GEP to get i8* pointer
                let zero = context.i32_type().const_zero();
                let indices = [zero, zero];
                let gep = builder.build_gep(
                    global.as_pointer_value(),
                    &indices,
                    "gep",
                ).unwrap();

                // Call printf
                builder.build_call(printf_func, &[gep.into()], "printf_call");
            }
        }

        // Add void return
        builder.build_return(None);
    }

    module.print_to_string().to_string()
}
