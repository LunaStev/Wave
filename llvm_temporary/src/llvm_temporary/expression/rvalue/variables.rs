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

use inkwell::AddressSpace;
use super::ExprGenEnv;
use inkwell::types::{AnyTypeEnum, BasicTypeEnum};
use inkwell::values::{BasicValue, BasicValueEnum};

pub(crate) fn gen<'ctx, 'a>(
    env: &mut ExprGenEnv<'ctx, 'a>,
    var_name: &str,
    expected_type: Option<BasicTypeEnum<'ctx>>,
) -> BasicValueEnum<'ctx> {
    if var_name == "true" {
        return env.context.bool_type().const_int(1, false).as_basic_value_enum();
    } else if var_name == "false" {
        return env.context.bool_type().const_int(0, false).as_basic_value_enum();
    } else if var_name == "null" {
        return env.context
            .i8_type()
            .ptr_type(AddressSpace::from(0))
            .const_null()
            .as_basic_value_enum();
    }

    if let Some(const_val) = env.global_consts.get(var_name) {
        return *const_val;
    }

    if let Some(var_info) = env.variables.get(var_name) {
        let ptr = var_info.ptr;

        if let Some(et) = expected_type {
            if et.is_pointer_type() {
                // ptr: i8**
                let loaded = env
                    .builder
                    .build_load(ptr, &format!("load_{}", var_name))
                    .unwrap();

                let expected_ptr = et.into_pointer_type();
                if loaded.get_type() != BasicTypeEnum::from(expected_ptr) {
                    return env
                        .builder
                        .build_bit_cast(
                            loaded,
                            expected_ptr,
                            &format!("{}_as_ptr", var_name),
                        )
                        .unwrap()
                        .as_basic_value_enum();
                }

                return loaded;
            }
        }

        let elem_ty = ptr.get_type().get_element_type();
        match elem_ty {
            AnyTypeEnum::ArrayType(_) => ptr.as_basic_value_enum(),
            _ => env
                .builder
                .build_load(ptr, &format!("load_{}", var_name))
                .unwrap()
                .as_basic_value_enum(),
        }
    } else if env.module.get_function(var_name).is_some() {
        panic!("Error: '{}' is a function name, not a variable", var_name);
    } else {
        panic!("variable '{}' not found in current scope", var_name);
    }
}