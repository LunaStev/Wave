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
use inkwell::types::BasicTypeEnum;
use parser::ast::WaveType;

/// Wave format string -> C printf format string
///
/// NOTE:
/// - Inkwell (opaque pointers) cannot extract the element type from PointerType.
/// - Therefore, determining "whether this pointer is a C string" cannot be done solely with LLVM types.
/// - The caller (io.rs) must also pass arg_is_cstr.
pub fn wave_format_to_c<'ctx>(
    context: &'ctx Context,
    format: &str,
    arg_types: &[BasicTypeEnum<'ctx>],
    arg_is_cstr: &[bool],
) -> String {
    assert!(
        arg_types.len() == arg_is_cstr.len(),
        "arg_types and arg_is_cstr length mismatch"
    );

    let mut result = String::new();
    let mut chars = format.chars().peekable();
    let mut arg_index = 0usize;

    while let Some(c) = chars.next() {
        if c == '{' {
            let mut spec = String::new();
            while let Some(&p) = chars.peek() {
                chars.next(); // consume
                if p == '}' {
                    break;
                }
                spec.push(p);
            }

            let spec = spec.trim();

            let ty = arg_types
                .get(arg_index)
                .unwrap_or_else(|| panic!("Missing argument for format at index {}", arg_index));

            let is_cstr = *arg_is_cstr
                .get(arg_index)
                .unwrap_or_else(|| panic!("Missing arg_is_cstr at index {}", arg_index));

            let fmt = if spec.is_empty() {
                match ty {
                    BasicTypeEnum::IntType(int_ty) => {
                        let bits = int_ty.get_bit_width();
                        match bits {
                            1 => "%d",
                            8 => "%hhd",
                            16 => "%hd",
                            32 => "%d",
                            64 => "%ld",
                            128 => "%lld",
                            _ => "%d",
                        }
                    }
                    BasicTypeEnum::FloatType(float_ty) => {
                        if *float_ty == context.f32_type() {
                            "%f"
                        } else {
                            "%lf"
                        }
                    }
                    BasicTypeEnum::PointerType(_) => {
                        if is_cstr {
                            "%s"
                        } else {
                            "%p"
                        }
                    }
                    BasicTypeEnum::ArrayType(_) => "%p",
                    BasicTypeEnum::StructType(_) => "%p",
                    BasicTypeEnum::VectorType(_) => "%p",
                    BasicTypeEnum::ScalableVectorType(_) => "%p",
                }
            } else {
                match spec {
                    "c" => "%c",
                    "x" => "%x",
                    "p" => "%p",
                    "s" => "%s",
                    "d" => "%d",
                    _ => panic!("Unknown format spec: {{{}}}", spec),
                }
            };

            result.push_str(fmt);
            arg_index += 1;
            continue;
        }

        result.push(c);
    }

    result
}

pub fn wave_format_to_scanf(format: &str, arg_types: &[WaveType]) -> String {
    let mut result = String::new();
    let mut chars = format.chars().peekable();
    let mut arg_index = 0usize;

    while let Some(c) = chars.next() {
        if c == '{' {
            if let Some('}') = chars.peek() {
                chars.next(); // consume '}'

                let ty = arg_types
                    .get(arg_index)
                    .unwrap_or_else(|| panic!("Missing argument for format at index {}", arg_index));

                let fmt = match ty {
                    WaveType::Bool => "%d",

                    WaveType::Char => "%c",
                    WaveType::Byte => "%hhu",

                    WaveType::Int(bits) => match *bits {
                        8 => "%hhd",
                        16 => "%hd",
                        32 => "%d",
                        64 => "%ld",
                        128 => "%lld",
                        _ => "%d",
                    },

                    WaveType::Uint(bits) => match *bits {
                        8 => "%hhu",
                        16 => "%hu",
                        32 => "%u",
                        64 => "%lu",
                        128 => "%llu",
                        _ => "%u",
                    },

                    WaveType::Float(bits) => match *bits {
                        32 => "%f",   // float*
                        64 => "%lf",  // double*
                        other => panic!("Unsupported float width in scanf: {}", other),
                    },

                    WaveType::Pointer(_) | WaveType::String => {
                        panic!("Cannot input into pointer/string type directly")
                    }

                    other => panic!("Unsupported type in scanf format: {:?}", other),
                };

                result.push_str(fmt);
                arg_index += 1;
                continue;
            }
        }

        result.push(c);
    }

    result
}
