use inkwell::types::{AnyTypeEnum, AsTypeRef, BasicType, BasicTypeEnum};
use super::ExprGenEnv;
use inkwell::values::{BasicMetadataValueEnum, BasicValue, BasicValueEnum, PointerValue};
use parser::ast::{Expression, WaveType};
use crate::llvm_temporary::llvm_codegen::abi_c::{ParamLowering, RetLowering};
use crate::llvm_temporary::statement::variable::{coerce_basic_value, CoercionMode};

fn pack_agg_to_int<'ctx, 'a>(
    env: &ExprGenEnv<'ctx, 'a>,
    agg: BasicValueEnum<'ctx>,
    dst: inkwell::types::IntType<'ctx>,
    tag: &str,
) -> BasicValueEnum<'ctx> {
    let agg_ty = agg.get_type();
    let tmp = env.builder.build_alloca(agg_ty, &format!("{}_agg_tmp", tag)).unwrap();
    env.builder.build_store(tmp, agg).unwrap();

    let int_ptr_ty = dst.ptr_type(tmp.get_type().get_address_space());
    let casted = env.builder
        .build_bit_cast(tmp.as_basic_value_enum(), int_ptr_ty.as_basic_type_enum(), &format!("{}_agg_i_ptr", tag))
        .unwrap()
        .into_pointer_value();

    env.builder
        .build_load(casted, &format!("{}_agg_i", tag))
        .unwrap()
        .as_basic_value_enum()
}

fn unpack_int_to_agg<'ctx, 'a>(
    env: &ExprGenEnv<'ctx, 'a>,
    iv: inkwell::values::IntValue<'ctx>,
    dst_agg_ty: BasicTypeEnum<'ctx>,
    tag: &str,
) -> BasicValueEnum<'ctx> {
    let tmp = env.builder.build_alloca(dst_agg_ty, &format!("{}_i2agg_tmp", tag)).unwrap();

    let int_ptr_ty = iv.get_type().ptr_type(tmp.get_type().get_address_space());
    let casted = env.builder
        .build_bit_cast(tmp.as_basic_value_enum(), int_ptr_ty.as_basic_type_enum(), &format!("{}_i_ptr", tag))
        .unwrap()
        .into_pointer_value();

    env.builder.build_store(casted, iv).unwrap();

    env.builder
        .build_load(tmp, &format!("{}_i2agg_load", tag))
        .unwrap()
        .as_basic_value_enum()
}

fn normalize_struct_name(raw: &str) -> &str {
    raw.strip_prefix("struct.").unwrap_or(raw).trim_start_matches('%')
}

fn resolve_struct_key<'ctx>(
    st: inkwell::types::StructType<'ctx>,
    struct_types: &std::collections::HashMap<String, inkwell::types::StructType<'ctx>>,
) -> String {
    if let Some(raw) = st.get_name().and_then(|n| n.to_str().ok()) {
        return normalize_struct_name(raw).to_string();
    }

    let st_ref = st.as_type_ref();
    for (name, ty) in struct_types {
        if ty.as_type_ref() == st_ref {
            return name.clone();
        }
    }

    panic!("LLVM struct type has no name and cannot be matched to struct_types");
}


pub(crate) fn gen_method_call<'ctx, 'a>(
    env: &mut ExprGenEnv<'ctx, 'a>,
    object: &Expression,
    name: &str,
    args: &[Expression],
) -> BasicValueEnum<'ctx> {
    // struct method sugar: obj.method(...)
    if let Expression::Variable(var_name) = object {
        if let Some(var_info) = env.variables.get(var_name) {
            if let WaveType::Struct(struct_name) = &var_info.ty {
                let fn_name = format!("{}_{}", struct_name, name);

                let function = env
                    .module
                    .get_function(&fn_name)
                    .unwrap_or_else(|| panic!("Function '{}' not found", fn_name));

                let fn_type = function.get_type();
                let param_types = fn_type.get_param_types();
                let expected_self = param_types.get(0).cloned();

                let obj_val = env.gen(object, expected_self);

                let mut call_args: Vec<BasicMetadataValueEnum> = Vec::new();
                call_args.push(obj_val.into());

                for (i, arg_expr) in args.iter().enumerate() {
                    let expected_ty = param_types.get(i + 1).cloned();
                    let mut arg_val = env.gen(arg_expr, expected_ty);
                    if let Some(et) = expected_ty {
                        arg_val = coerce_basic_value(
                            env.context, env.builder, arg_val, et, &format!("arg{}_cast", i),
                            CoercionMode::Implicit
                        );
                    }
                    call_args.push(arg_val.into());
                }

                let call_site = env
                    .builder
                    .build_call(function, &call_args, &format!("call_{}", fn_name))
                    .unwrap();

                if function.get_type().get_return_type().is_some() {
                    return call_site
                        .try_as_basic_value()
                        .left()
                        .expect("Expected a return value from struct method");
                } else {
                    return env.context.i32_type().const_zero().as_basic_value_enum();
                }
            }
        }
    }

    {
        let obj_preview = env.gen(object, None);
        let obj_ty = obj_preview.get_type();

        let struct_name_opt: Option<String> = match obj_ty {
            BasicTypeEnum::StructType(st) => Some(resolve_struct_key(st, env.struct_types)),
            BasicTypeEnum::PointerType(p) if p.get_element_type().is_struct_type() => {
                Some(resolve_struct_key(p.get_element_type().into_struct_type(), env.struct_types))
            }
            _ => None,
        };

        if let Some(struct_name) = struct_name_opt {
            let fn_name = format!("{}_{}", struct_name, name);

            if let Some(function) = env.module.get_function(&fn_name) {
                let fn_type = function.get_type();
                let param_types = fn_type.get_param_types();
                let expected_self = param_types.get(0).cloned();

                let mut obj_val = obj_preview;
                if let Some(et) = expected_self {
                    obj_val = coerce_basic_value(
                        env.context,
                        env.builder,
                        obj_val,
                        et,
                        "self_cast",
                        CoercionMode::Implicit,
                    );
                }

                let mut call_args: Vec<BasicMetadataValueEnum> = Vec::new();
                call_args.push(obj_val.into());

                for (i, arg_expr) in args.iter().enumerate() {
                    let expected_ty = param_types.get(i + 1).cloned();
                    let mut arg_val = env.gen(arg_expr, expected_ty);
                    if let Some(et) = expected_ty {
                        arg_val = coerce_basic_value(
                            env.context, env.builder, arg_val, et, &format!("arg{}_cast", i),
                            CoercionMode::Implicit
                        );
                    }
                    call_args.push(arg_val.into());
                }

                let call_site = env
                    .builder
                    .build_call(function, &call_args, &format!("call_{}", fn_name))
                    .unwrap();

                if function.get_type().get_return_type().is_some() {
                    return call_site.try_as_basic_value().left().unwrap();
                } else {
                    return env.context.i32_type().const_zero().as_basic_value_enum();
                }
            }
        }
    }

    // method-style call: fn(self, ...)
    let function = env
        .module
        .get_function(name)
        .unwrap_or_else(|| panic!("Function '{}' not found for method-style call", name));

    let fn_type = function.get_type();
    let param_types = fn_type.get_param_types();

    if param_types.is_empty() {
        panic!(
            "Method-style call {}() requires at least 1 parameter (self)",
            name
        );
    }

    let expected_self = param_types.get(0).cloned();
    let obj_val = env.gen(object, expected_self);

    let mut call_args: Vec<BasicMetadataValueEnum> = Vec::new();
    call_args.push(obj_val.into());

    for (i, arg_expr) in args.iter().enumerate() {
        let expected_ty = param_types.get(i + 1).cloned();
        let mut arg_val = env.gen(arg_expr, expected_ty);
        if let Some(et) = expected_ty {
            arg_val = coerce_basic_value(
                env.context, env.builder, arg_val, et, &format!("arg{}_cast", i),
                CoercionMode::Implicit
            );
        }
        call_args.push(arg_val.into());
    }

    let call_site = env
        .builder
        .build_call(function, &call_args, &format!("call_{}", name))
        .unwrap();

    if function.get_type().get_return_type().is_some() {
        call_site.try_as_basic_value().left().unwrap()
    } else {
        env.context.i32_type().const_zero().as_basic_value_enum()
    }
}

pub(crate) fn gen_function_call<'ctx, 'a>(
    env: &mut ExprGenEnv<'ctx, 'a>,
    name: &str,
    args: &[Expression],
    expected_type: Option<BasicTypeEnum<'ctx>>,
) -> BasicValueEnum<'ctx> {

    if let Some(info) = env.extern_c_info.get(name) {
        let function = env.module
            .get_function(name)
            .unwrap_or_else(|| panic!("Extern function '{}' not found in module (symbol alias?)", name));

        if args.len() != info.params.len() {
            panic!(
                "Extern `{}` expects {} arguments (wave-level), got {}",
                name, info.params.len(), args.len()
            );
        }

        let fn_type = function.get_type();
        let llvm_param_types = fn_type.get_param_types();

        let mut lowered_args: Vec<BasicMetadataValueEnum<'ctx>> = Vec::new();
        let mut llvm_pi: usize = 0;

        // 1) sret hidden param
        let mut sret_tmp: Option<PointerValue<'ctx>> = None;
        if let RetLowering::SRet { ty, .. } = &info.ret {
            let agg = any_agg_to_basic(*ty);
            let tmp = env.builder.build_alloca(agg, &format!("{}_sret_tmp", name)).unwrap();

            let expected_ptr = llvm_param_types[0]
                .into_pointer_type();
            let tmp2 = coerce_ptr_to(env, tmp, expected_ptr, &format!("{}_sret_ptrcast", name));

            lowered_args.push(tmp2.as_basic_value_enum().into());
            sret_tmp = Some(tmp);
            llvm_pi += 1;
        }

        // 2) wave params
        for (i, (arg_expr, p)) in args.iter().zip(info.params.iter()).enumerate() {
            match p {
                ParamLowering::Direct(t) => {
                    let mut v = env.gen(arg_expr, Some(*t));
                    v = coerce_to_expected(env, v, *t, name, i);
                    lowered_args.push(v.into());
                    llvm_pi += 1;
                }

                ParamLowering::ByVal { ty, .. } => {
                    let agg = any_agg_to_basic(*ty);
                    let v = env.gen(arg_expr, Some(agg));
                    let tmp = env.builder.build_alloca(agg, &format!("{}_byval_tmp_{}", name, i)).unwrap();
                    env.builder.build_store(tmp, v).unwrap();

                    let expected_ptr = llvm_param_types[llvm_pi].into_pointer_type();
                    let tmp2 = coerce_ptr_to(env, tmp, expected_ptr, &format!("{}_byval_ptrcast_{}", name, i));
                    lowered_args.push(tmp2.as_basic_value_enum().into());
                    llvm_pi += 1;
                }

                ParamLowering::Split(parts) => {
                    let mut agg_val = env.gen(arg_expr, None);

                    if let BasicValueEnum::PointerValue(pv) = agg_val {
                        let et = pv.get_type().get_element_type();
                        if et.is_struct_type() || et.is_array_type() {
                            agg_val = env.builder
                                .build_load(pv, &format!("{}_split_load_{}", name, i))
                                .unwrap()
                                .as_basic_value_enum();
                        }
                    }

                    let split_vals = split_hfa_from_agg(env, agg_val, parts, &format!("{}_split_{}", name, i));

                    for sv in split_vals {
                        let et = llvm_param_types[llvm_pi];
                        let vv = coerce_basic_value(env.context, env.builder, sv, et, "split_cast", CoercionMode::Implicit);
                        lowered_args.push(vv.into());
                        llvm_pi += 1;
                    }
                }

            }
        }

        let call_name = match info.ret {
            RetLowering::Void | RetLowering::SRet { .. } => String::new(),
            _ => format!("call_{}", name),
        };

        let call_site = env.builder
            .build_call(function, &lowered_args, &call_name)
            .unwrap();

        // 3) return
        match &info.ret {
            RetLowering::Void => {
                if expected_type.is_some() {
                    panic!("Extern '{}' returns void and cannot be used as a value", name);
                }
                return env.context.i32_type().const_zero().as_basic_value_enum();
            }

            RetLowering::SRet { ty, .. } => {
                let tmp = sret_tmp.expect("SRet lowering requires sret tmp");
                let agg = any_agg_to_basic(*ty);
                let v = env.builder.build_load(tmp, &format!("{}_sret_load", name)).unwrap();

                if let Some(et) = expected_type {
                    return coerce_lowered_ret_to_expected(env, v.as_basic_value_enum(), et, "sret_ret");
                }
                return v.as_basic_value_enum();
            }

            RetLowering::Direct(_t) => {
                let rv = call_site.try_as_basic_value().left().unwrap();

                if let Some(et) = expected_type {
                    return coerce_lowered_ret_to_expected(env, rv, et, "direct_ret");
                }
                return rv;
            }
        }
    }

    let function = env
        .module
        .get_function(name)
        .unwrap_or_else(|| panic!("Function '{}' not found", name));

    let fn_type = function.get_type();
    let param_types: Vec<BasicTypeEnum> = fn_type.get_param_types();
    let ret_type: Option<BasicTypeEnum> = fn_type.get_return_type();

    if args.len() != param_types.len() {
        panic!(
            "Function `{}` expects {} arguments, got {}",
            name,
            param_types.len(),
            args.len()
        );
    }

    let mut call_args: Vec<BasicMetadataValueEnum> = Vec::with_capacity(args.len());

    for (i, arg) in args.iter().enumerate() {
        let expected_param_ty = param_types[i];
        let mut val = env.gen(arg, Some(expected_param_ty));
        val = coerce_to_expected(env, val, expected_param_ty, name, i);
        call_args.push(val.into());
    }

    let call_name = if ret_type.is_some() { format!("call_{}", name) } else { String::new() };

    let call_site = env
        .builder
        .build_call(function, &call_args, &call_name)
        .unwrap();

    match ret_type {
        Some(_) => call_site.try_as_basic_value().left().unwrap(),
        None => {
            if expected_type.is_some() {
                panic!("Function '{}' returns void and cannot be used as a value", name);
            }
            env.context.i32_type().const_zero().as_basic_value_enum()
        }
    }
}

fn coerce_to_expected<'ctx, 'a>(
    env: &ExprGenEnv<'ctx, 'a>,
    val: BasicValueEnum<'ctx>,
    expected: BasicTypeEnum<'ctx>,
    name: &str,
    arg_index: usize,
) -> BasicValueEnum<'ctx> {
    let got = val.get_type();
    if got == expected {
        return val;
    }

    match (got, expected) {
        // 0) ptr -> int (ptrtoint)  (needed for syscall wrappers that take i64 registers)
        (BasicTypeEnum::PointerType(_), BasicTypeEnum::IntType(dst))
        if dst.get_bit_width() == 64 && name.starts_with("syscall") =>
            {
                let pv = val.into_pointer_value();
                env.builder
                    .build_ptr_to_int(pv, dst, &format!("arg{}_p2i", arg_index))
                    .unwrap()
                    .as_basic_value_enum()
            }

        // 0.1) int -> ptr (inttoptr) (useful when passing raw addresses)
        (BasicTypeEnum::IntType(src), BasicTypeEnum::PointerType(dst))
        if src.get_bit_width() == 64 && name.starts_with("syscall") =>
            {
                let iv = val.into_int_value();
                env.builder
                    .build_int_to_ptr(iv, dst, &format!("arg{}_i2p", arg_index))
                    .unwrap()
                    .as_basic_value_enum()
            }

        // 1) int -> int
        (BasicTypeEnum::IntType(src), BasicTypeEnum::IntType(dst)) => {
            let src_bw = src.get_bit_width();
            let dst_bw = dst.get_bit_width();
            let iv = val.into_int_value();

            if src_bw < dst_bw {
                env.builder
                    .build_int_s_extend(iv, dst, &format!("arg{}_sext", arg_index))
                    .unwrap()
                    .as_basic_value_enum()
            } else if src_bw > dst_bw {
                env.builder
                    .build_int_truncate(iv, dst, &format!("arg{}_trunc", arg_index))
                    .unwrap()
                    .as_basic_value_enum()
            } else {
                iv.as_basic_value_enum()
            }
        }

        // 2) ptr-to-array -> array value (load)
        (BasicTypeEnum::PointerType(p), BasicTypeEnum::ArrayType(a))
        if p.get_element_type().is_array_type()
            && p.get_element_type().into_array_type() == a =>
            {
                let ptr = val.into_pointer_value();
                env.builder
                    .build_load(ptr, &format!("arg{}_arr_load", arg_index))
                    .unwrap()
                    .as_basic_value_enum()
            }

        // 3) ptr-to-struct -> struct value (load)  (more robust: allow different named struct identities)
        (BasicTypeEnum::PointerType(_p), BasicTypeEnum::StructType(s)) => {
            let ptr = val.into_pointer_value();

            let expected_ptr_ty = s.ptr_type(ptr.get_type().get_address_space());
            let casted = env.builder
                .build_bit_cast(ptr.as_basic_value_enum(), expected_ptr_ty, &format!("arg{}_st_ptrcast", arg_index))
                .unwrap()
                .into_pointer_value();

            env.builder
                .build_load(casted, &format!("arg{}_st_load", arg_index))
                .unwrap()
                .as_basic_value_enum()
        }

        // 4) ptr -> ptr (bitcast)
        (BasicTypeEnum::PointerType(_), BasicTypeEnum::PointerType(dst)) => {
            env.builder
                .build_bit_cast(val, dst, &format!("arg{}_ptrcast", arg_index))
                .unwrap()
                .as_basic_value_enum()
        }

        // (Struct|Array) -> Int : store-size가 정확히 맞으면 bit-pack
        (got_ty @ (BasicTypeEnum::StructType(_) | BasicTypeEnum::ArrayType(_)), BasicTypeEnum::IntType(dst)) => {
            let sz = env.target_data.get_store_size(&got_ty) as u64;
            let bits = (sz * 8) as u32;
            if bits == dst.get_bit_width() {
                return pack_agg_to_int(env, val, dst, &format!("arg{}_pack", arg_index));
            }
            panic!(
                "Cannot pack aggregate to int: agg bits {} != dst bits {} (arg {} of {})",
                bits, dst.get_bit_width(), arg_index, name
            );
        }

        (BasicTypeEnum::IntType(src), dst_agg @ (BasicTypeEnum::StructType(_) | BasicTypeEnum::ArrayType(_))) => {
            let sz = env.target_data.get_store_size(&dst_agg) as u64;
            let bits = (sz * 8) as u32;
            if bits == src.get_bit_width() {
                let iv = val.into_int_value();
                return unpack_int_to_agg(env, iv, dst_agg, &format!("arg{}_unpack", arg_index));
            }
            panic!(
                "Cannot unpack int to aggregate: int bits {} != agg bits {} (arg {} of {})",
                src.get_bit_width(), bits, arg_index, name
            );
        }


        _ => {
            panic!(
                "Type mismatch for arg {} of '{}': expected {:?}, got {:?}",
                arg_index, name, expected, got
            );
        }
    }
}

fn any_agg_to_basic<'ctx>(ty: AnyTypeEnum<'ctx>) -> BasicTypeEnum<'ctx> {
    match ty {
        AnyTypeEnum::StructType(st) => st.as_basic_type_enum(),
        AnyTypeEnum::ArrayType(at) => at.as_basic_type_enum(),
        _ => panic!("Expected aggregate AnyTypeEnum (struct/array), got {:?}", ty),
    }
}

fn coerce_ptr_to<'ctx, 'a>(
    env: &ExprGenEnv<'ctx, 'a>,
    pv: PointerValue<'ctx>,
    expected_ptr_ty: inkwell::types::PointerType<'ctx>,
    tag: &str,
) -> PointerValue<'ctx> {
    if pv.get_type() == expected_ptr_ty {
        return pv;
    }
    env.builder
        .build_bit_cast(
            pv.as_basic_value_enum(),
            expected_ptr_ty.as_basic_type_enum(),
            tag,
        )
        .unwrap()
        .into_pointer_value()
}

fn split_hfa_from_agg<'ctx, 'a>(
    env: &ExprGenEnv<'ctx, 'a>,
    agg_val: BasicValueEnum<'ctx>,
    parts: &[BasicTypeEnum<'ctx>],
    tag: &str,
) -> Vec<BasicValueEnum<'ctx>> {
    let agg_ty = agg_val.get_type();
    let tmp = env.builder.build_alloca(agg_ty, &format!("{tag}_hfa_tmp")).unwrap();
    env.builder.build_store(tmp, agg_val).unwrap();

    let float_ty: inkwell::types::FloatType<'ctx> = match parts.first().unwrap() {
        BasicTypeEnum::FloatType(ft) => *ft,
        BasicTypeEnum::VectorType(vt) => match vt.get_element_type() {
            BasicTypeEnum::FloatType(ft) => ft,
            other => panic!("HFA vector elem is not float: {:?}", other),
        },
        other => panic!("HFA part is not float/vector: {:?}", other),
    };

    let fptr_ty = float_ty.ptr_type(tmp.get_type().get_address_space());
    let base_fptr = env.builder
        .build_bit_cast(tmp.as_basic_value_enum(), fptr_ty.as_basic_type_enum(), &format!("{tag}_hfa_fptr"))
        .unwrap()
        .into_pointer_value();

    let mut out = Vec::with_capacity(parts.len());
    let mut idx: u32 = 0;

    for (pi, part_ty) in parts.iter().enumerate() {
        match part_ty {
            BasicTypeEnum::FloatType(_) => {
                let i = env.context.i32_type().const_int(idx as u64, false);
                let ep = unsafe {
                    env.builder
                        .build_gep(base_fptr, &[i], &format!("{tag}_hfa_gep_{pi}"))
                        .unwrap()
                };
                let fv = env.builder.build_load(ep, &format!("{tag}_hfa_f_{pi}")).unwrap();
                out.push(fv.as_basic_value_enum());
                idx += 1;
            }
            BasicTypeEnum::VectorType(vt) => {
                let n = vt.get_size(); // element count
                let mut v = vt.get_undef();

                for j in 0..n {
                    let i = env.context.i32_type().const_int((idx + j) as u64, false);
                    let ep = unsafe {
                        env.builder
                            .build_gep(base_fptr, &[i], &format!("{tag}_hfa_gep_{pi}_{j}"))
                            .unwrap()
                    };
                    let fv = env.builder.build_load(ep, &format!("{tag}_hfa_f_{pi}_{j}")).unwrap();

                    let jv = env.context.i32_type().const_int(j as u64, false);
                    v = env.builder
                        .build_insert_element(v, fv, jv, &format!("{tag}_hfa_ins_{pi}_{j}"))
                        .unwrap();
                }

                out.push(v.as_basic_value_enum());
                idx += n;
            }
            other => panic!("Unsupported Split part type: {:?}", other),
        }
    }

    out
}

fn coerce_lowered_ret_to_expected<'ctx, 'a>(
    env: &ExprGenEnv<'ctx, 'a>,
    lowered_ret: BasicValueEnum<'ctx>,
    expected: BasicTypeEnum<'ctx>,
    tag: &str,
) -> BasicValueEnum<'ctx> {
    if lowered_ret.get_type() == expected {
        return lowered_ret;
    }

    match (lowered_ret.get_type(), expected) {
        (BasicTypeEnum::IntType(_), BasicTypeEnum::StructType(_) | BasicTypeEnum::ArrayType(_)) => {
            let iv = lowered_ret.into_int_value();
            unpack_int_to_agg(env, iv, expected, tag)
        }
        // vector(float) -> struct/array (HFA ret)
        (BasicTypeEnum::VectorType(vt), BasicTypeEnum::StructType(_) | BasicTypeEnum::ArrayType(_)) => {
            let tmp = env.builder.build_alloca(expected, &format!("{tag}_v2agg_tmp")).unwrap();

            let vptr_ty = vt.ptr_type(tmp.get_type().get_address_space());
            let vptr = env.builder
                .build_bit_cast(tmp.as_basic_value_enum(), vptr_ty.as_basic_type_enum(), &format!("{tag}_vptr"))
                .unwrap()
                .into_pointer_value();

            env.builder.build_store(vptr, lowered_ret).unwrap();

            env.builder
                .build_load(tmp, &format!("{tag}_v2agg_load"))
                .unwrap()
                .as_basic_value_enum()
        }
        (BasicTypeEnum::FloatType(ft), BasicTypeEnum::StructType(_) | BasicTypeEnum::ArrayType(_)) => {
            let tmp = env.builder.build_alloca(expected, &format!("{tag}_f2agg_tmp")).unwrap();
            let fptr_ty = ft.ptr_type(tmp.get_type().get_address_space());
            let fptr = env.builder
                .build_bit_cast(tmp.as_basic_value_enum(), fptr_ty.as_basic_type_enum(), &format!("{tag}_fptr"))
                .unwrap()
                .into_pointer_value();
            env.builder.build_store(fptr, lowered_ret).unwrap();
            env.builder.build_load(tmp, &format!("{tag}_f2agg_load")).unwrap().as_basic_value_enum()
        }
        _ => lowered_ret,
    }
}