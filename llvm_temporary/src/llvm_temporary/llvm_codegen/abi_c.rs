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

// src/llvm_temporary/llvm_codegen/abi_c.rs
use std::collections::HashMap;
use inkwell::AddressSpace;
use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::context::Context;
use inkwell::targets::TargetData;
use inkwell::types::{AnyType, AnyTypeEnum, BasicMetadataTypeEnum, BasicType, BasicTypeEnum};
use inkwell::values::FunctionValue;

use parser::ast::{ExternFunctionNode, WaveType};

use super::types::{wave_type_to_llvm_type, TypeFlavor};

#[derive(Clone)]
pub enum ParamLowering<'ctx> {
    Direct(BasicTypeEnum<'ctx>),                 // pass as this llvm type
    Split(Vec<BasicTypeEnum<'ctx>>),             // pass as multiple params
    ByVal { ty: AnyTypeEnum<'ctx>, align: u32 }, // pass ptr + byval + align
}

#[derive(Clone)]
pub enum RetLowering<'ctx> {
    Void,
    Direct(BasicTypeEnum<'ctx>),
    SRet { ty: AnyTypeEnum<'ctx>, align: u32 }, // hidden first param
}

#[derive(Clone)]
pub struct ExternCInfo<'ctx> {
    pub wave_ret: WaveType,              // Wave-level return type (needed when sret => llvm void)
    pub ret: RetLowering<'ctx>,
    pub params: Vec<ParamLowering<'ctx>>, // per-wave param
    pub llvm_param_types: Vec<BasicMetadataTypeEnum<'ctx>>, // final lowered param list (including sret ptr, split, byval ptr)
}

pub struct LoweredExtern<'ctx> {
    pub info: ExternCInfo<'ctx>,
    pub llvm_name: String,
    pub fn_type: inkwell::types::FunctionType<'ctx>,
}

fn is_float_ty<'ctx>(td: &TargetData, t: BasicTypeEnum<'ctx>) -> Option<u32> {
    match t {
        BasicTypeEnum::FloatType(_) => Some(td.get_store_size(&t) as u32), // 4 or 8
        _ => None,
    }
}

fn any_ptr_basic<'ctx>(ty: AnyTypeEnum<'ctx>) -> BasicTypeEnum<'ctx> {
    let aspace = AddressSpace::default();
    match ty {
        AnyTypeEnum::ArrayType(t)  => t.ptr_type(aspace).as_basic_type_enum(),
        AnyTypeEnum::FloatType(t)  => t.ptr_type(aspace).as_basic_type_enum(),
        AnyTypeEnum::FunctionType(t)=> t.ptr_type(aspace).as_basic_type_enum(),
        AnyTypeEnum::IntType(t)    => t.ptr_type(aspace).as_basic_type_enum(),
        AnyTypeEnum::PointerType(t)=> t.ptr_type(aspace).as_basic_type_enum(),
        AnyTypeEnum::StructType(t) => t.ptr_type(aspace).as_basic_type_enum(),
        AnyTypeEnum::VectorType(t) => t.ptr_type(aspace).as_basic_type_enum(),
        _ => panic!("unsupported AnyTypeEnum for ptr"),
    }
}

fn flatten_leaf_types<'ctx>(t: BasicTypeEnum<'ctx>, out: &mut Vec<BasicTypeEnum<'ctx>>) {
    match t {
        BasicTypeEnum::StructType(st) => {
            for i in 0..st.count_fields() {
                let f = st.get_field_type_at_index(i).unwrap();
                flatten_leaf_types(f, out);
            }
        }
        BasicTypeEnum::ArrayType(at) => {
            let elem = at.get_element_type();
            for _ in 0..at.len() {
                flatten_leaf_types(elem, out);
            }
        }
        _ => out.push(t),
    }
}

fn classify_param<'ctx>(
    context: &'ctx Context,
    td: &TargetData,
    t: BasicTypeEnum<'ctx>,
) -> ParamLowering<'ctx> {
    let size = td.get_store_size(&t) as u64;

    // large aggregates => byval
    if matches!(t, BasicTypeEnum::StructType(_) | BasicTypeEnum::ArrayType(_)) && size > 16 {
        let align = td.get_abi_alignment(&t) as u32;
        return ParamLowering::ByVal { ty: t.as_any_type_enum(), align };
    }

    // small aggregates: try integer-only or homogeneous float
    if matches!(t, BasicTypeEnum::StructType(_) | BasicTypeEnum::ArrayType(_)) && size <= 16 {
        let mut leaves = vec![];
        flatten_leaf_types(t, &mut leaves);

        // homogeneous float aggregate
        let mut float_kind: Option<u32> = None;
        let mut all_float = true;
        for lt in &leaves {
            if let Some(sz) = is_float_ty(td, *lt) {
                float_kind.get_or_insert(sz);
                if float_kind != Some(sz) {
                    all_float = false;
                    break;
                }
            } else {
                all_float = false;
                break;
            }
        }

        if all_float {
            let count = leaves.len();
            let fsz = float_kind.unwrap_or(0);
            if fsz == 4 {
                let f = context.f32_type();
                return match count {
                    1 => ParamLowering::Direct(f.as_basic_type_enum()),
                    2 => ParamLowering::Direct(f.vec_type(2).as_basic_type_enum()),
                    3 => ParamLowering::Split(vec![
                        f.vec_type(2).as_basic_type_enum(),
                        f.as_basic_type_enum(),
                    ]),
                    4 => ParamLowering::Direct(f.vec_type(4).as_basic_type_enum()),
                    _ => {
                        let align = td.get_abi_alignment(&t) as u32;
                        ParamLowering::ByVal { ty: t.as_any_type_enum(), align }
                    }
                };
            } else if fsz == 8 {
                let f = context.f64_type();
                return match count {
                    1 => ParamLowering::Direct(f.as_basic_type_enum()),
                    2 => ParamLowering::Direct(f.vec_type(2).as_basic_type_enum()),
                    _ => {
                        let align = td.get_abi_alignment(&t) as u32;
                        ParamLowering::ByVal { ty: t.as_any_type_enum(), align }
                    }
                };
            }
        }

        // integer-only aggregate: coerce to i{size*8}
        let mut all_intlike = true;
        for lt in &leaves {
            match lt {
                BasicTypeEnum::IntType(_) | BasicTypeEnum::PointerType(_) => {}
                _ => { all_intlike = false; break; }
            }
        }
        if all_intlike {
            let bits = (size * 8) as u32;
            let it = context.custom_width_int_type(bits);
            return ParamLowering::Direct(it.as_basic_type_enum());
        }

        // mixed small aggregate: safest is byval (conservative but correct)
        let align = td.get_abi_alignment(&t) as u32;
        return ParamLowering::ByVal { ty: t.as_any_type_enum(), align };
    }

    // non-aggregate: direct
    ParamLowering::Direct(t)
}

fn classify_ret<'ctx>(
    context: &'ctx Context,
    td: &TargetData,
    t: Option<BasicTypeEnum<'ctx>>,
) -> RetLowering<'ctx> {
    let Some(t) = t else { return RetLowering::Void; };

    let size = td.get_store_size(&t) as u64;
    let is_agg = matches!(t, BasicTypeEnum::StructType(_) | BasicTypeEnum::ArrayType(_));

    if is_agg && size > 16 {
        let align = td.get_abi_alignment(&t) as u32;
        return RetLowering::SRet { ty: t.as_any_type_enum(), align };
    }

    if is_agg && size <= 16 {
        // integer-only ret => i{size*8}
        let mut leaves = vec![];
        flatten_leaf_types(t, &mut leaves);

        let mut all_intlike = true;
        for lt in &leaves {
            match lt {
                BasicTypeEnum::IntType(_) | BasicTypeEnum::PointerType(_) => {}
                _ => { all_intlike = false; break; }
            }
        }
        if all_intlike {
            let bits = (size * 8) as u32;
            let it = context.custom_width_int_type(bits);
            return RetLowering::Direct(it.as_basic_type_enum());
        }

        // homogeneous float ret (2 or 4 only to avoid multi-reg return complexity)
        let mut float_kind: Option<u32> = None;
        let mut all_float = true;
        for lt in &leaves {
            if let Some(sz) = is_float_ty(td, *lt) {
                float_kind.get_or_insert(sz);
                if float_kind != Some(sz) { all_float = false; break; }
            } else { all_float = false; break; }
        }
        if all_float {
            let count = leaves.len();
            if float_kind == Some(4) {
                let f = context.f32_type();
                return match count {
                    1 => RetLowering::Direct(f.as_basic_type_enum()),
                    2 => RetLowering::Direct(f.vec_type(2).as_basic_type_enum()),
                    4 => RetLowering::Direct(f.vec_type(4).as_basic_type_enum()),
                    _ => {
                        // 3-float ret 같은 건 일단 sret로 안전하게
                        let align = td.get_abi_alignment(&t) as u32;
                        RetLowering::SRet { ty: t.as_any_type_enum(), align }
                    }
                };
            }
            if float_kind == Some(8) {
                let f = context.f64_type();
                return match count {
                    1 => RetLowering::Direct(f.as_basic_type_enum()),
                    2 => RetLowering::Direct(f.vec_type(2).as_basic_type_enum()),
                    _ => {
                        let align = td.get_abi_alignment(&t) as u32;
                        RetLowering::SRet { ty: t.as_any_type_enum(), align }
                    }
                };
            }
        }

        // mixed small aggregate ret: safest sret
        let align = td.get_abi_alignment(&t) as u32;
        return RetLowering::SRet { ty: t.as_any_type_enum(), align };
    }

    RetLowering::Direct(t)
}

pub fn lower_extern_c<'ctx>(
    context: &'ctx Context,
    td: &TargetData,
    ext: &ExternFunctionNode,
    struct_types: &HashMap<String, inkwell::types::StructType<'ctx>>,
) -> LoweredExtern<'ctx> {
    let llvm_name = ext.symbol.as_deref().unwrap_or(ext.name.as_str()).to_string();

    // wave types -> layout types
    let wave_param_layout: Vec<BasicTypeEnum<'ctx>> = ext.params.iter()
        .map(|(_, ty)| wave_type_to_llvm_type(context, ty, struct_types, TypeFlavor::AbiC))
        .collect();

    let wave_ret_layout: Option<BasicTypeEnum<'ctx>> = match &ext.return_type {
        WaveType::Void => None,
        ty => Some(wave_type_to_llvm_type(context, ty, struct_types, TypeFlavor::AbiC)),
    };

    let ret = classify_ret(context, td, wave_ret_layout);
    let mut params: Vec<ParamLowering<'ctx>> = vec![];
    for p in wave_param_layout {
        params.push(classify_param(context, td, p));
    }

    // build lowered param list (sret first, then params possibly split)
    let mut llvm_param_types: Vec<BasicMetadataTypeEnum<'ctx>> = vec![];

    if let RetLowering::SRet { ty, .. } = &ret {
        // sret param is ptr to the return aggregate
        let ptr = any_ptr_basic(ty.clone());
        llvm_param_types.push(ptr.into());
    }

    for p in &params {
        match p {
            ParamLowering::Direct(t) => llvm_param_types.push((*t).into()),
            ParamLowering::Split(parts) => {
                for pt in parts {
                    llvm_param_types.push((*pt).into());
                }
            }
            ParamLowering::ByVal { ty, .. } => {
                let ptr = any_ptr_basic(ty.clone());
                llvm_param_types.push(ptr.into());
            }
        }
    }

    let fn_type = match &ret {
        RetLowering::Void | RetLowering::SRet { .. } => context.void_type().fn_type(&llvm_param_types, false),
        RetLowering::Direct(t) => t.fn_type(&llvm_param_types, false),
    };

    LoweredExtern {
        llvm_name,
        fn_type,
        info: ExternCInfo {
            wave_ret: ext.return_type.clone(),
            ret,
            params,
            llvm_param_types,
        },
    }
}

pub fn apply_extern_c_attrs<'ctx>(
    context: &'ctx Context,
    f: FunctionValue<'ctx>,
    info: &ExternCInfo<'ctx>,
) {
    let mut llvm_param_index: u32 = 0;

    // sret first param
    if let RetLowering::SRet { ty, align } = &info.ret {
        let sret_kind = Attribute::get_named_enum_kind_id("sret");
        let sret_attr = context.create_type_attribute(sret_kind, *ty);
        f.add_attribute(AttributeLoc::Param(0), sret_attr);

        let align_kind = Attribute::get_named_enum_kind_id("align");
        let align_attr = context.create_enum_attribute(align_kind, *align as u64);
        f.add_attribute(AttributeLoc::Param(0), align_attr);

        llvm_param_index += 1;
    }

    for p in &info.params {
        match p {
            ParamLowering::Direct(_) => {
                llvm_param_index += 1;
            }
            ParamLowering::Split(parts) => {
                llvm_param_index += parts.len() as u32;
            }
            ParamLowering::ByVal { ty, align } => {
                let byval_kind = Attribute::get_named_enum_kind_id("byval");
                let byval_attr = context.create_type_attribute(byval_kind, *ty);
                f.add_attribute(AttributeLoc::Param(llvm_param_index), byval_attr);

                let align_kind = Attribute::get_named_enum_kind_id("align");
                let align_attr = context.create_enum_attribute(align_kind, *align as u64);
                f.add_attribute(AttributeLoc::Param(llvm_param_index), align_attr);

                llvm_param_index += 1;
            }
        }
    }
}
