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
                        // 문자열을 상수로 정의하고 출력
                        let string_label = format!("@str_{}", name); // 고유한 문자열 레이블 생성
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

            ir.push_str("    ret void\n");
            ir.push_str("}\n");
        }
        _ => {}
    }

    ir
}
