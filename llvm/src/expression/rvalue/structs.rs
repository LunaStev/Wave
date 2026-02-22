// This file is part of the Wave language project.
// Copyright (c) 2024–2026 Wave Foundation
// Copyright (c) 2024–2026 LunaStev and contributors
//
// This Source Code Form is subject to the terms of the
// Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file,
// You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use super::ExprGenEnv;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValue, BasicValueEnum};
use parser::ast::Expression;
use crate::codegen::generate_address_and_type_ir;

pub(crate) fn gen_struct_literal<'ctx, 'a>(
    env: &mut ExprGenEnv<'ctx, 'a>,
    name: &str,
    fields: &[(String, Expression)],
) -> BasicValueEnum<'ctx> {
    let struct_ty = *env
        .struct_types
        .get(name)
        .unwrap_or_else(|| panic!("Struct type '{}' not found", name));

    let field_indices = env
        .struct_field_indices
        .get(name)
        .unwrap_or_else(|| panic!("Field index map for struct '{}' not found", name));

    let tmp_alloca = env
        .builder
        .build_alloca(struct_ty, &format!("tmp_{}_literal", name))
        .unwrap();

    for (field_name, field_expr) in fields {
        let idx = *field_indices
            .get(field_name)
            .unwrap_or_else(|| panic!("Field '{}' not found in struct '{}'", field_name, name));

        let expected_field_ty: BasicTypeEnum<'ctx> = struct_ty
            .get_field_type_at_index(idx)
            .unwrap_or_else(|| panic!("No field type at index {} for struct '{}'", idx, name));

        let mut field_val = env.gen(field_expr, Some(expected_field_ty));

        if field_val.get_type() != expected_field_ty {
            panic!(
                "Struct literal field type mismatch: {}.{} expected {:?}, got {:?}",
                name, field_name, expected_field_ty, field_val.get_type()
            );
        }

        let field_ptr = env
            .builder
            .build_struct_gep(struct_ty, tmp_alloca, idx, &format!("{}.{}", name, field_name))
            .unwrap();

        env.builder.build_store(field_ptr, field_val).unwrap();
    }

    env.builder
        .build_load(struct_ty.as_basic_type_enum(), tmp_alloca, &format!("{}_literal_val", name))
        .unwrap()
        .as_basic_value_enum()
}

pub(crate) fn gen_field_access<'ctx, 'a>(
    env: &mut ExprGenEnv<'ctx, 'a>,
    object: &Expression,
    field: &str,
) -> BasicValueEnum<'ctx> {
    let full = Expression::FieldAccess {
        object: Box::new(object.clone()),
        field: field.to_string(),
    };

    let (ptr, field_ty) = generate_address_and_type_ir(
        env.context,
        env.builder,
        &full,
        env.variables,
        env.module,
        env.struct_types,
        env.struct_field_indices,
    );

    env.builder
        .build_load(field_ty, ptr, &format!("load_field_{}", field))
        .unwrap()
        .as_basic_value_enum()
}
