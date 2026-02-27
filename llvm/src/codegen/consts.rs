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

use inkwell::context::Context;
use inkwell::types::{BasicType, BasicTypeEnum, StringRadix, StructType};
use inkwell::values::{BasicValue, BasicValueEnum};

use parser::ast::{Expression, Literal, WaveType};
use std::collections::HashMap;
use std::fmt;

use super::types::{wave_type_to_llvm_type, TypeFlavor};

#[derive(Debug, Clone)]
pub enum ConstEvalError {
    UnknownIdentifier(String),
    TypeMismatch {
        expected: String,
        got: String,
        note: String,
    },
    InvalidLiteral(String),
    Unsupported(String),
}

impl fmt::Display for ConstEvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConstEvalError::UnknownIdentifier(n) => write!(f, "unknown const identifier `{}`", n),
            ConstEvalError::TypeMismatch { expected, got, note } => {
                write!(f, "type mismatch (expected {}, got {}): {}", expected, got, note)
            }
            ConstEvalError::InvalidLiteral(s) => write!(f, "invalid literal: {}", s),
            ConstEvalError::Unsupported(s) => write!(f, "unsupported const expression: {}", s),
        }
    }
}

fn type_name<'ctx>(t: BasicTypeEnum<'ctx>) -> String {
    format!("{:?}", t)
}

fn value_type_name<'ctx>(v: BasicValueEnum<'ctx>) -> String {
    format!("{:?}", v.get_type())
}

fn parse_signed_and_radix(s: &str) -> (bool, StringRadix, String) {
    let mut t = s.trim().replace('_', "");
    if t.is_empty() {
        return (false, StringRadix::Decimal, "".to_string());
    }

    let mut neg = false;
    if let Some(rest) = t.strip_prefix('-') {
        neg = true;
        t = rest.to_string();
    } else if let Some(rest) = t.strip_prefix('+') {
        t = rest.to_string();
    }

    let (radix, digits) = if let Some(rest) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        (StringRadix::Hexadecimal, rest)
    } else if let Some(rest) = t.strip_prefix("0b").or_else(|| t.strip_prefix("0B")) {
        (StringRadix::Binary, rest)
    } else if let Some(rest) = t.strip_prefix("0o").or_else(|| t.strip_prefix("0O")) {
        (StringRadix::Octal, rest)
    } else {
        (StringRadix::Decimal, t.as_str())
    };

    (neg, radix, digits.to_string())
}

fn is_zero_like(s: &str) -> bool {
    let s = s.trim().replace('_', "");
    let s = s.strip_prefix('+').unwrap_or(&s);
    let s = s.strip_prefix('-').unwrap_or(s);

    let s = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    let s = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")).unwrap_or(s);
    let s = s.strip_prefix("0o").or_else(|| s.strip_prefix("0O")).unwrap_or(s);

    !s.is_empty() && s.chars().all(|c| c == '0')
}

fn strip_struct_prefix(raw: &str) -> &str {
    raw.strip_prefix("struct.").unwrap_or(raw)
}

fn const_from_expected<'ctx>(
    context: &'ctx Context,
    expected: BasicTypeEnum<'ctx>,
    expr: &Expression,
    struct_types: &HashMap<String, StructType<'ctx>>,
    struct_field_indices: &HashMap<String, HashMap<String, u32>>,
    const_env: &HashMap<String, BasicValueEnum<'ctx>>,
) -> Result<BasicValueEnum<'ctx>, ConstEvalError> {
    match expr {
        Expression::Grouped(inner) => {
            return const_from_expected(context, expected, inner, struct_types, struct_field_indices, const_env);
        }

        Expression::Variable(name) => {
            let v = match const_env.get(name) {
                Some(v) => *v,
                None => return Err(ConstEvalError::UnknownIdentifier(name.clone())),
            };

            if v.get_type() != expected {
                return Err(ConstEvalError::TypeMismatch {
                    expected: type_name(expected),
                    got: value_type_name(v),
                    note: format!("identifier `{}` resolved to a const of different LLVM type", name),
                });
            }

            Ok(v)
        }

        Expression::Null => match expected {
            BasicTypeEnum::PointerType(pt) => Ok(pt.const_null().as_basic_value_enum()),
            _ => Err(ConstEvalError::TypeMismatch {
                expected: type_name(expected),
                got: "null".to_string(),
                note: "null can only be used where a pointer is expected".to_string(),
            }),
        },

        // --- ints ---
        Expression::Literal(Literal::Int(s)) => match expected {
            BasicTypeEnum::IntType(int_ty) => {
                let (neg, radix, digits) = parse_signed_and_radix(s);
                let mut iv = int_ty
                    .const_int_from_string(&digits, radix)
                    .ok_or_else(|| ConstEvalError::InvalidLiteral(s.clone()))?;

                if neg {
                    iv = iv.const_neg();
                }
                Ok(iv.as_basic_value_enum())
            }
            BasicTypeEnum::PointerType(ptr_ty) => {
                if is_zero_like(s) {
                    Ok(ptr_ty.const_null().as_basic_value_enum())
                } else {
                    Err(ConstEvalError::TypeMismatch {
                        expected: type_name(expected),
                        got: format!("int({})", s),
                        note: "only 0 can be used as a const null pointer literal".to_string(),
                    })
                }
            }
            _ => Err(ConstEvalError::TypeMismatch {
                expected: type_name(expected),
                got: format!("int({})", s),
                note: "const int literal not compatible with expected type".to_string(),
            }),
        },

        // --- floats ---
        Expression::Literal(Literal::Float(fv)) => match expected {
            BasicTypeEnum::FloatType(float_ty) => Ok(float_ty.const_float(*fv).as_basic_value_enum()),
            _ => Err(ConstEvalError::TypeMismatch {
                expected: type_name(expected),
                got: "float".to_string(),
                note: "const float literal not compatible with expected type".to_string(),
            }),
        },

        // --- struct literal ---
        Expression::StructLiteral { name, fields } => {
            let st = match expected {
                BasicTypeEnum::StructType(st) => st,
                _ => {
                    return Err(ConstEvalError::TypeMismatch {
                        expected: type_name(expected),
                        got: "struct-literal".to_string(),
                        note: format!("StructLiteral '{}' used where non-struct expected", name),
                    })
                }
            };

            let field_count = st.count_fields() as usize;

            let struct_name = if !name.is_empty() {
                name.as_str()
            } else {
                st.get_name()
                    .and_then(|c| c.to_str().ok())
                    .map(strip_struct_prefix)
                    .unwrap_or("")
            };

            let positional = fields.iter().all(|(n, _)| n.is_empty());
            let mut slots: Vec<Option<BasicValueEnum<'ctx>>> = vec![None; field_count];

            if positional {
                if fields.len() != field_count {
                    return Err(ConstEvalError::Unsupported(format!(
                        "StructLiteral '{}' positional init expects {} fields, got {}",
                        struct_name, field_count, fields.len()
                    )));
                }

                for (i, (_, vexpr)) in fields.iter().enumerate() {
                    let fty = st
                        .get_field_type_at_index(i as u32)
                        .ok_or_else(|| ConstEvalError::Unsupported(format!(
                            "Struct '{}' has no field index {}",
                            struct_name, i
                        )))?;

                    let cv = const_from_expected(context, fty, vexpr, struct_types, struct_field_indices, const_env)?;
                    slots[i] = Some(cv);
                }
            } else {
                let idx_map = struct_field_indices.get(struct_name).ok_or_else(|| {
                    ConstEvalError::Unsupported(format!("Struct '{}' field map not found", struct_name))
                })?;

                for (fname, vexpr) in fields {
                    let idx = *idx_map.get(fname).ok_or_else(|| {
                        ConstEvalError::Unsupported(format!("Field '{}' not found in struct '{}'", fname, struct_name))
                    })? as usize;

                    let fty = st.get_field_type_at_index(idx as u32).ok_or_else(|| {
                        ConstEvalError::Unsupported(format!("Struct '{}' has no field index {}", struct_name, idx))
                    })?;

                    let cv = const_from_expected(context, fty, vexpr, struct_types, struct_field_indices, const_env)?;
                    slots[idx] = Some(cv);
                }
            }

            let mut ordered: Vec<BasicValueEnum<'ctx>> = Vec::with_capacity(field_count);
            for i in 0..field_count {
                let fty = st.get_field_type_at_index(i as u32).unwrap();
                ordered.push(slots[i].unwrap_or_else(|| fty.const_zero()));
            }

            Ok(st.const_named_struct(&ordered).as_basic_value_enum())
        }

        // --- array literal ---
        Expression::ArrayLiteral(elems) => match expected {
            BasicTypeEnum::ArrayType(at) => {
                let len = at.len() as usize;
                if elems.len() != len {
                    return Err(ConstEvalError::Unsupported(format!(
                        "Array literal length mismatch: expected {}, got {}",
                        len, elems.len()
                    )));
                }

                let elem_ty = at.get_element_type();

                let elem_vals: Vec<BasicValueEnum<'ctx>> = elems
                    .iter()
                    .map(|e| const_from_expected(context, elem_ty, e, struct_types, struct_field_indices, const_env))
                    .collect::<Result<_, _>>()?;

                match elem_ty {
                    BasicTypeEnum::IntType(int_ty) => {
                        let mut vs = Vec::with_capacity(len);
                        for v in elem_vals {
                            match v {
                                BasicValueEnum::IntValue(iv) => vs.push(iv),
                                other => {
                                    return Err(ConstEvalError::TypeMismatch {
                                        expected: type_name(elem_ty),
                                        got: value_type_name(other),
                                        note: "array element expected int".to_string(),
                                    })
                                }
                            }
                        }
                        Ok(int_ty.const_array(&vs).as_basic_value_enum())
                    }

                    BasicTypeEnum::FloatType(float_ty) => {
                        let mut vs = Vec::with_capacity(len);
                        for v in elem_vals {
                            match v {
                                BasicValueEnum::FloatValue(fv) => vs.push(fv),
                                other => {
                                    return Err(ConstEvalError::TypeMismatch {
                                        expected: type_name(elem_ty),
                                        got: value_type_name(other),
                                        note: "array element expected float".to_string(),
                                    })
                                }
                            }
                        }
                        Ok(float_ty.const_array(&vs).as_basic_value_enum())
                    }

                    BasicTypeEnum::PointerType(ptr_ty) => {
                        let mut vs = Vec::with_capacity(len);
                        for v in elem_vals {
                            match v {
                                BasicValueEnum::PointerValue(pv) => vs.push(pv),
                                other => {
                                    return Err(ConstEvalError::TypeMismatch {
                                        expected: type_name(elem_ty),
                                        got: value_type_name(other),
                                        note: "array element expected pointer".to_string(),
                                    })
                                }
                            }
                        }
                        Ok(ptr_ty.const_array(&vs).as_basic_value_enum())
                    }

                    BasicTypeEnum::StructType(st_ty) => {
                        let mut vs = Vec::with_capacity(len);
                        for v in elem_vals {
                            match v {
                                BasicValueEnum::StructValue(sv) => vs.push(sv),
                                other => {
                                    return Err(ConstEvalError::TypeMismatch {
                                        expected: type_name(elem_ty),
                                        got: value_type_name(other),
                                        note: "array element expected struct".to_string(),
                                    })
                                }
                            }
                        }
                        Ok(st_ty.const_array(&vs).as_basic_value_enum())
                    }

                    BasicTypeEnum::ArrayType(inner_at) => {
                        let mut vs = Vec::with_capacity(len);
                        for v in elem_vals {
                            match v {
                                BasicValueEnum::ArrayValue(av) => vs.push(av),
                                other => {
                                    return Err(ConstEvalError::TypeMismatch {
                                        expected: type_name(elem_ty),
                                        got: value_type_name(other),
                                        note: "array element expected array".to_string(),
                                    })
                                }
                            }
                        }
                        Ok(inner_at.const_array(&vs).as_basic_value_enum())
                    }

                    other => Err(ConstEvalError::Unsupported(format!(
                        "Unsupported const array element type: {:?}",
                        other
                    ))),
                }
            }
            _ => Err(ConstEvalError::TypeMismatch {
                expected: type_name(expected),
                got: "array-literal".to_string(),
                note: "Array literal used where non-array expected".to_string(),
            }),
        },

        _ => Err(ConstEvalError::Unsupported(format!(
            "Constant expression must be a literal/struct/array/identifier, got {:?}",
            expr
        ))),
    }
}

pub(super) fn create_llvm_const_value<'ctx>(
    context: &'ctx Context,
    ty: &WaveType,
    expr: &Expression,
    struct_types: &HashMap<String, StructType<'ctx>>,
    struct_field_indices: &HashMap<String, HashMap<String, u32>>,
    const_env: &HashMap<String, BasicValueEnum<'ctx>>,
) -> Result<BasicValueEnum<'ctx>, ConstEvalError> {
    if matches!(expr, Expression::Null) && !matches!(ty, WaveType::Pointer(_)) {
        return Err(ConstEvalError::TypeMismatch {
            expected: format!("{:?}", ty),
            got: "null".to_string(),
            note: "null can only be assigned to ptr<T>".to_string(),
        });
    }

    let expected = wave_type_to_llvm_type(context, ty, struct_types, TypeFlavor::AbiC);
    const_from_expected(context, expected, expr, struct_types, struct_field_indices, const_env)
}
