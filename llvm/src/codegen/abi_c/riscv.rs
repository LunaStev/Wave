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
// AI TRAINING NOTICE: Prohibited without prior written permission. No use for machine learning or generative AI training, fine-tuning, distillation, embedding, or dataset creation.

//! riscv C ABI classification; consumes shared transport records.
use super::shared::*;
use inkwell::context::Context;
use inkwell::targets::TargetData;
use inkwell::types::{AnyType, BasicType, BasicTypeEnum};

pub(super) fn classify_param_riscv64<'ctx>(
    context: &'ctx Context,
    td: &TargetData,
    t: BasicTypeEnum<'ctx>,
) -> ParamLowering<'ctx> {
    let size = td.get_store_size(&t) as u64;
    let is_agg = matches!(
        t,
        BasicTypeEnum::StructType(_) | BasicTypeEnum::ArrayType(_)
    );

    if is_agg && size > 16 {
        let align = td.get_abi_alignment(&t) as u32;
        return ParamLowering::ByVal {
            ty: t.as_any_type_enum(),
            align,
        };
    }

    if is_agg && size == 0 {
        return ParamLowering::Ignore;
    }

    if is_agg {
        let mut leaves = Vec::new();
        flatten_leaf_types(t, &mut leaves);
        let integer_only = leaves.iter().all(|leaf| {
            matches!(
                leaf,
                BasicTypeEnum::IntType(_) | BasicTypeEnum::PointerType(_)
            )
        });

        if integer_only {
            if size <= 8 {
                // The RV64 psABI uses an XLEN-sized transport slot for an
                // integer aggregate that fits in one argument register.
                return ParamLowering::Direct(context.i64_type().as_basic_type_enum());
            }

            return ParamLowering::Direct(context.i64_type().array_type(2).as_basic_type_enum());
        }
    }

    ParamLowering::Direct(t)
}

pub(super) fn classify_ret_riscv64<'ctx>(
    context: &'ctx Context,
    td: &TargetData,
    t: Option<BasicTypeEnum<'ctx>>,
) -> RetLowering<'ctx> {
    let Some(t) = t else {
        return RetLowering::Void;
    };
    let size = td.get_store_size(&t) as u64;
    let is_agg = matches!(
        t,
        BasicTypeEnum::StructType(_) | BasicTypeEnum::ArrayType(_)
    );

    if is_agg && size > 16 {
        let align = td.get_abi_alignment(&t) as u32;
        return RetLowering::SRet {
            ty: t.as_any_type_enum(),
            align,
        };
    }

    if is_agg && size == 0 {
        return RetLowering::Void;
    }

    if is_agg {
        let mut leaves = Vec::new();
        flatten_leaf_types(t, &mut leaves);
        let integer_only = leaves.iter().all(|leaf| {
            matches!(
                leaf,
                BasicTypeEnum::IntType(_) | BasicTypeEnum::PointerType(_)
            )
        });

        if integer_only {
            if size <= 8 {
                // Keep the aggregate object size separate from its XLEN-sized
                // ABI return transport representation.
                return RetLowering::Direct(context.i64_type().as_basic_type_enum());
            }

            return RetLowering::Direct(context.i64_type().array_type(2).as_basic_type_enum());
        }
    }

    RetLowering::Direct(t)
}
