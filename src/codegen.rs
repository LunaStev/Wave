use std::ffi::CString;
use crate::ast::ASTNode;
use llvm_sys::core::*;

pub fn generate(ast: &ASTNode) -> Result<(), String> {
    unsafe {
        let context = LLVMContextCreate();
        let module = LLVMModuleCreateWithName(b"main_module\0".as_ptr() as *const _);
        let builder = LLVMCreateBuilderInContext(context);

        match ast {
            ASTNode::Function { name, body, .. } => {
                let i32_type = LLVMInt32TypeInContext(context);
                let func_type = LLVMFunctionType(i32_type, std::ptr::null_mut(), 0, 0);
                let function = LLVMAddFunction(module, CString::new(name.clone()).unwrap().as_ptr(), func_type);
                let entry = LLVMAppendBasicBlockInContext(context, function, b"entry\0".as_ptr() as *const _);
                LLVMPositionBuilderAtEnd(builder, entry);

                for stmt in body {
                    match stmt {
                        ASTNode::Println(content) => {
                            // println 구현: LLVM에서 문자열 출력
                        }
                        _ => {}
                    }
                }

                LLVMBuildRet(builder, LLVMConstInt(i32_type, 0, 0));
                LLVMDumpModule(module);
            }
            _ => {}
        }

        LLVMDisposeBuilder(builder);
        LLVMDisposeModule(module);
        LLVMContextDispose(context);
    }
    Ok(())
}
