use super::ExprGenEnv;
use inkwell::types::{AnyTypeEnum, BasicType, BasicTypeEnum};
use inkwell::values::{BasicValue, BasicValueEnum};
use parser::ast::Expression;
use crate::llvm_temporary::llvm_codegen::generate_address_ir;

fn push_deref_into_base(expr: &Expression) -> Expression {
    match expr {
        Expression::Grouped(inner) => {
            Expression::Grouped(Box::new(push_deref_into_base(inner)))
        }

        Expression::IndexAccess { target, index } => {
            Expression::IndexAccess {
                target: Box::new(push_deref_into_base(target)),
                index: index.clone(),
            }
        }

        Expression::FieldAccess { object, field } => {
            Expression::FieldAccess {
                object: Box::new(push_deref_into_base(object)),
                field: field.clone(),
            }
        }

        other => Expression::Deref(Box::new(other.clone())),
    }
}

pub(crate) fn gen_deref<'ctx, 'a>(
    env: &mut ExprGenEnv<'ctx, 'a>,
    inner_expr: &Expression,
) -> BasicValueEnum<'ctx> {
    match inner_expr {
        Expression::Grouped(inner) => {
            return gen_deref(env, inner);
        }

        Expression::IndexAccess { .. } | Expression::FieldAccess { .. } => {
            let addr = generate_address_ir(
                env.context,
                env.builder,
                inner_expr,
                env.variables,
                env.module,
                env.struct_types,
                env.struct_field_indices,
            );

            return env.builder
                .build_load(addr, "deref_load")
                .unwrap()
                .as_basic_value_enum();
        }

        _ => {}
    }

    let v = env.gen(inner_expr, None);
    if let BasicValueEnum::PointerValue(p) = v {
        return env.builder
            .build_load(p, "deref_load")
            .unwrap()
            .as_basic_value_enum();
    }

    panic!("deref expects pointer or lvalue (x[i], x.field), got: {:?}", inner_expr);
}

pub(crate) fn gen_addressof<'ctx, 'a>(
    env: &mut ExprGenEnv<'ctx, 'a>,
    inner_expr: &Expression,
    expected_type: Option<BasicTypeEnum<'ctx>>,
) -> BasicValueEnum<'ctx> {
    if let Expression::ArrayLiteral(elements) = inner_expr {
        let ptr_ty = match expected_type {
            Some(BasicTypeEnum::PointerType(p)) => p,
            _ => panic!("&[ ... ] needs an expected pointer type (e.g. ptr<i32>)"),
        };

        let elem_any = ptr_ty.get_element_type();
        let elem_basic: BasicTypeEnum<'ctx> = match elem_any {
            AnyTypeEnum::IntType(t) => t.into(),
            AnyTypeEnum::FloatType(t) => t.into(),
            AnyTypeEnum::PointerType(t) => t.into(),
            AnyTypeEnum::StructType(t) => t.into(),
            AnyTypeEnum::ArrayType(t) => t.into(),
            AnyTypeEnum::VectorType(t) => t.into(),
            other => panic!("&[ ... ] unsupported element type: {:?}", other),
        };

        let array_ty = elem_basic.array_type(elements.len() as u32);
        let arr_alloca = env.builder.build_alloca(array_ty, "tmp_array").unwrap();

        for (i, expr) in elements.iter().enumerate() {
            let val = env.gen(expr, Some(elem_basic));

            let gep = unsafe {
                env.builder
                    .build_in_bounds_gep(
                        arr_alloca,
                        &[
                            env.context.i32_type().const_zero(),
                            env.context.i32_type().const_int(i as u64, false),
                        ],
                        &format!("array_idx_{}", i),
                    )
                    .unwrap()
            };

            env.builder.build_store(gep, val).unwrap();
        }

        if elem_basic == ptr_ty.get_element_type().try_into().unwrap() {
            let first = unsafe {
                env.builder
                    .build_in_bounds_gep(
                        arr_alloca,
                        &[
                            env.context.i32_type().const_zero(),
                            env.context.i32_type().const_zero(),
                        ],
                        "array_first_ptr",
                    )
                    .unwrap()
            };

            if first.get_type() != ptr_ty {
                return env.builder
                    .build_bit_cast(first, ptr_ty, "addrof_array_cast")
                    .unwrap()
                    .as_basic_value_enum();
            }

            return first.as_basic_value_enum();
        }

        return arr_alloca.as_basic_value_enum();
    }

    let addr = generate_address_ir(
        env.context,
        env.builder,
        inner_expr,
        env.variables,
        env.module,
        env.struct_types,
        env.struct_field_indices,
    );

    if let Some(BasicTypeEnum::PointerType(ptr_ty)) = expected_type {
        if addr.get_type() != ptr_ty {
            return env.builder
                .build_bit_cast(addr, ptr_ty, "addrof_cast")
                .unwrap()
                .as_basic_value_enum();
        }
    }

    addr.as_basic_value_enum()
}
