use crate::parser::ast::{ASTNode, FunctionNode, StatementNode};

pub fn generate_ir(ast: &ASTNode) -> String {
    let mut ir = String::new();

    match ast {
        ASTNode::Function(FunctionNode { name, parameters, body }) => {
            ir.push_str(&format!("define void @{}() {{\n", name));
            ir.push_str("entry:\n");

            for statement in body {
                match statement {
                    ASTNode::Statement(StatementNode::Println(message)) => {
                        let string_label = format!("@str_{}", name);
                        ir.push_str(&format!(
                            "    {} = private constant [{} x i8] c\"{}\\00\"\n",
                            string_label,
                            message.len() + 1,
                            message
                        ));

                        ir.push_str(&format!(
                            "    call void @printf(i8* getelementptr inbounds ([{} x i8], [{} x i8]* {}, i32 0, i32 0))\n",
                            message.len() + 1,
                            message.len() + 1,
                            string_label
                        ));
                    }
                    _ => {}
                }
            }

        // Add void return
        builder.build_return(None);
    }

    module.print_to_string().to_string()
}
