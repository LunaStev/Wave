use std::collections::HashMap;
use inkwell::AddressSpace;
use inkwell::context::Context;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{FunctionValue, PointerValue};
use lexer::TokenType;
use parser::ast::{Expression, Mutability, WaveType};

pub fn wave_format_to_c(format: &str, arg_types: &[BasicTypeEnum]) -> String {
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

pub fn wave_type_to_llvm_type<'ctx>(context: &'ctx Context, wave_type: &WaveType) -> BasicTypeEnum<'ctx> {
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

pub fn generate_address_ir<'ctx>(
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

#[derive(Clone)]
pub struct VariableInfo<'ctx> {
    pub ptr: PointerValue<'ctx>,
    pub mutability: Mutability,
}

pub fn get_llvm_type<'a>(context: &'a Context, ty: &TokenType) -> BasicTypeEnum<'a> {
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

pub unsafe fn create_alloc<'a>(
    context: &'a Context,
    builder: &'a inkwell::builder::Builder<'a>,
    function: FunctionValue<'a>,
    name: &'a str,
) -> PointerValue<'a> {
    let alloca = builder.build_alloca(context.i32_type(), name).unwrap();
    alloca
}