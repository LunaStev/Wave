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

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn wavec_bin() -> PathBuf {
    if let Some(path) = option_env!("CARGO_BIN_EXE_wavec") {
        return PathBuf::from(path);
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/wavec")
}

fn temp_case_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("wavec-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_wave(dir: &Path, name: &str, source: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, source).unwrap();
    path
}

fn wavec_command() -> Command {
    let mut command = Command::new(wavec_bin());
    command.env("NO_COLOR", "1");
    command
}

fn run_wavec<I, S>(args: I)
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = wavec_command().args(args).output().unwrap();
    assert!(
        output.status.success(),
        "wavec failed with status {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_wavec_capture<I, S>(args: I) -> (String, String)
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = wavec_command().args(args).output().unwrap();
    assert!(
        output.status.success(),
        "wavec failed with status {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

fn run_wavec_raw<I, S>(args: I) -> std::process::Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    wavec_command().args(args).output().unwrap()
}

fn run_wavec_expect_failure<I, S>(args: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = wavec_command().args(args).output().unwrap();
    assert!(
        !output.status.success(),
        "wavec unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn bytes_contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn riscv64_elf_flags(path: &Path) -> u32 {
    let object = fs::read(path).unwrap();
    assert!(
        object.len() >= 52,
        "truncated ELF object: {}",
        path.display()
    );
    assert_eq!(&object[..4], b"\x7fELF", "{}", path.display());
    assert_eq!(object[4], 2, "expected ELF64 object: {}", path.display());
    assert_eq!(u16::from_le_bytes([object[18], object[19]]), 243);
    u32::from_le_bytes([object[48], object[49], object[50], object[51]])
}

fn json_string_for_test(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn json_contains_path_components(json: &str, components: &[&str]) -> bool {
    json.contains(&components.join("/")) || json.contains(&components.join("\\\\"))
}

fn write_minimal_elf64_object(path: &Path, machine: u16) {
    let mut header = [0_u8; 64];
    header[..4].copy_from_slice(b"\x7fELF");
    header[4] = 2;
    header[5] = 1;
    header[6] = 1;
    header[16..18].copy_from_slice(&1_u16.to_le_bytes());
    header[18..20].copy_from_slice(&machine.to_le_bytes());
    fs::write(path, header).unwrap();
}

#[test]
fn json_path_matching_accepts_unix_and_escaped_windows_separators() {
    let components = ["crt", "riscv64-unknown-linux-gnu", "crt1.o"];
    assert!(json_contains_path_components(
        r#"{"args":["/opt/wave/crt/riscv64-unknown-linux-gnu/crt1.o"]}"#,
        &components
    ));
    assert!(json_contains_path_components(
        r#"{"args":["C:\\wave\\crt\\riscv64-unknown-linux-gnu\\crt1.o"]}"#,
        &components
    ));
}

fn run_link_tests_enabled() -> bool {
    std::env::var_os("WAVE_RUN_LINK_TESTS").is_some()
}

#[test]
fn semantic_validation_rejects_invalid_returns_calls_and_loop_control() {
    let dir = temp_case_dir("semantic-validation");
    let cases = [
        (
            "wrong_return_type.wave",
            r#"
fun value() -> i32 {
    return "not an integer";
}

"#,
            "type mismatch in return value of function `value`",
        ),
        (
            "value_from_void.wave",
            r#"
fun noop() {
    return 1;
}
"#,
            "void function `noop` cannot return a value",
        ),
        (
            "empty_non_void_return.wave",
            r#"
fun value() -> i32 {
    return;
}
"#,
            "non-void function `value` must return `i32`",
        ),
        (
            "missing_return_path.wave",
            r#"
fun value(flag: bool) -> i32 {
    if (flag) {
        return 1;
    }
}
"#,
            "non-void function `value` may exit without returning `i32`",
        ),
        (
            "break_outside_loop.wave",
            r#"
fun main() {
    break;
}
"#,
            "`break` can only be used inside a loop",
        ),
        (
            "continue_outside_loop.wave",
            r#"
fun main() {
    continue;
}
"#,
            "`continue` can only be used inside a loop",
        ),
        (
            "wrong_call_type.wave",
            r#"
fun identity(value: i32) -> i32 {
    return value;
}

fun main() {
    identity("text");
}
"#,
            "type mismatch in argument 1 of function `identity`",
        ),
    ];

    for (file_name, source, expected) in cases {
        let source = write_wave(&dir, file_name, source);
        for mode in ["check", "build"] {
            let output = if mode == "check" {
                run_wavec_raw([OsStr::new("check"), source.as_os_str()])
            } else {
                run_wavec_raw([
                    OsStr::new("build"),
                    source.as_os_str(),
                    OsStr::new("--emit=obj"),
                    OsStr::new("--out-dir"),
                    dir.as_os_str(),
                ])
            };
            assert!(
                !output.status.success(),
                "{} unexpectedly passed semantic validation in {} mode",
                file_name,
                mode
            );
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                stderr.contains("error[E3001]"),
                "{} ({}): {}",
                file_name,
                mode,
                stderr
            );
            assert!(
                stderr.contains(expected),
                "{} ({}): {}",
                file_name,
                mode,
                stderr
            );
            assert!(
                !stderr.contains("E9001") && !stderr.contains("compiler internal error"),
                "{} ({}) leaked a backend failure: {}",
                file_name,
                mode,
                stderr
            );
        }
    }
}

#[test]
fn semantic_validation_rejects_backend_only_type_failures_early() {
    let dir = temp_case_dir("semantic-backend-parity");
    let cases = [
        (
            "narrow_return.wave",
            "fun narrow(x: i64) -> i32 { return x; }\nfun main() {}\n",
            "expected `i32`, found `i64`",
        ),
        (
            "narrow_call.wave",
            "fun take(x: i32) {}\nfun main() { let wide: i64 = 1; take(wide); }\n",
            "argument 1 of function `take`",
        ),
        (
            "narrow_initializer.wave",
            "fun main() { let wide: i64 = 1; let narrow: i32 = wide; }\n",
            "initializer for `narrow`",
        ),
        (
            "narrow_assignment.wave",
            "fun main() { let wide: i64 = 1; var narrow: i32 = 0; narrow = wide; }\n",
            "assignment to `narrow`",
        ),
        (
            "unknown_function.wave",
            "fun main() { missing_function(); }\n",
            "call to unknown function `missing_function`",
        ),
        (
            "unknown_field.wave",
            "struct Point { x: i32; }\nfun read(p: Point) -> i32 { return p.missing; }\nfun main() {}\n",
            "struct `Point` has no field `missing`",
        ),
        (
            "unknown_method.wave",
            "struct Point { x: i32; }\nfun main() { let p: Point = Point { x: 1 }; p.missing(); }\n",
            "struct `Point` has no method `missing`",
        ),
        (
            "unknown_struct_literal_field.wave",
            "struct Point { x: i32; }\nfun make() -> Point { return Point { missing: 1 }; }\nfun main() {}\n",
            "struct `Point` has no field `missing`",
        ),
        (
            "missing_struct_literal_field.wave",
            "struct Point { x: i32; y: i32; }\nfun main() { let p: Point = Point { x: 1 }; }\n",
            "struct literal `Point` is missing field(s): y",
        ),
        (
            "array_return.wave",
            "fun values() -> i32 { return [1, 2]; }\nfun main() {}\n",
            "found `array literal`",
        ),
        (
            "array_element.wave",
            "fun main() { let values: array<i32, 1> = [\"text\"]; }\n",
            "element 0 of initializer for `values`",
        ),
        (
            "invalid_condition.wave",
            "struct Flag { value: i32; }\nfun main() { let flag: Flag = Flag { value: 1 }; if (flag) {} }\n",
            "if condition must be bool, numeric, pointer, or string",
        ),
        (
            "invalid_match.wave",
            "struct Value { x: i32; }\nfun main() { let v: Value = Value { x: 1 }; match (v) { _ => {} } }\n",
            "match value must be an integer or enum",
        ),
        (
            "invalid_deref.wave",
            "fun main() { let value: i32 = 1; println(\"{}\", deref value); }\n",
            "deref expects a pointer",
        ),
        (
            "invalid_index_target.wave",
            "fun main() { let value: i32 = 1; println(\"{}\", value[0]); }\n",
            "index access requires an array or pointer",
        ),
        (
            "invalid_index_type.wave",
            "fun main() { let values: array<i32, 1> = [1]; println(\"{}\", values[\"text\"]); }\n",
            "index expression must be an integer",
        ),
        (
            "invalid_unary.wave",
            "fun main() { println(\"{}\", -\"text\"); }\n",
            "unary operator `Neg` is not supported for `str`",
        ),
        (
            "invalid_increment.wave",
            "fun main() { var flag: bool = true; flag++; }\n",
            "++/-- requires a numeric or pointer lvalue",
        ),
        (
            "invalid_export.wave",
            "export(rust) fun exposed() -> i32 { return 1; }\nfun main() {}\n",
            "unsupported export ABI 'rust'",
        ),
    ];

    for (file_name, source, expected) in cases {
        let source = write_wave(&dir, file_name, source);
        for mode in ["check", "build"] {
            let output = if mode == "check" {
                run_wavec_raw([OsStr::new("check"), source.as_os_str()])
            } else {
                run_wavec_raw([
                    OsStr::new("build"),
                    source.as_os_str(),
                    OsStr::new("--emit=obj"),
                    OsStr::new("--out-dir"),
                    dir.as_os_str(),
                ])
            };
            assert!(
                !output.status.success(),
                "{} unexpectedly succeeded in {} mode",
                file_name,
                mode
            );
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(stderr.contains("error[E3001]"), "{}: {}", file_name, stderr);
            assert!(stderr.contains(expected), "{}: {}", file_name, stderr);
            assert!(
                !stderr.contains("E9001")
                    && !stderr.contains("compiler internal error")
                    && !stderr.contains("panic location"),
                "{} leaked a backend failure in {} mode: {}",
                file_name,
                mode,
                stderr
            );
        }
    }
}

#[test]
fn second_semantic_audit_rejects_unsafe_programs_before_codegen() {
    let dir = temp_case_dir("semantic-audit-two");
    let cases = [
        (
            "array_format.wave",
            "fun main() { println(\"{}\", [1, 2]); }\n",
            "format argument must be a scalar",
        ),
        (
            "void_value.wave",
            "fun noop() {}\nfun main() { println(\"{}\", noop()); }\n",
            "found `void`",
        ),
        (
            "struct_format.wave",
            "struct Pair { x: i32; }\nfun main() { let p: Pair = Pair { x: 1 }; println(\"{}\", p); }\n",
            "found `Pair`",
        ),
        (
            "compound_string.wave",
            "fun main() { var text: str = \"a\"; text += \"b\"; }\n",
            "compound assignment `AddAssign` requires numeric operands",
        ),
        (
            "compound_bool.wave",
            "fun main() { var flag: bool = true; flag += false; }\n",
            "compound assignment `AddAssign` requires numeric operands",
        ),
        (
            "input_literal.wave",
            "fun main() { input(\"{}\", 1); }\n",
            "input argument must be a mutable lvalue",
        ),
        (
            "input_immutable.wave",
            "fun main() { let value: i32 = 0; input(\"{}\", value); }\n",
            "cannot write input into immutable binding `value`",
        ),
        (
            "invalid_struct_cast.wave",
            "struct Pair { x: i32; }\nfun main() { let p: Pair = Pair { x: 1 }; let bits: i32 = p as i32; }\n",
            "invalid cast from `Pair` to `i32`",
        ),
        (
            "invalid_string_float_cast.wave",
            "fun main() { let value: f32 = \"text\" as f32; }\n",
            "invalid cast from `str` to `f32`",
        ),
        (
            "invalid_void_cast.wave",
            "fun noop() {}\nfun main() { let value: i32 = noop() as i32; }\n",
            "invalid cast from `void` to `i32`",
        ),
        (
            "unknown_cast_type.wave",
            "fun main() { let value: i32 = 1 as Missing; }\n",
            "unknown type `Missing` in cast target",
        ),
        (
            "unknown_variable_type.wave",
            "fun main() { var value: Missing; }\n",
            "unknown type `Missing` in variable `value`",
        ),
        (
            "unknown_return_type.wave",
            "fun make() -> Missing { var value: Missing; return value; }\nfun main() {}\n",
            "unknown type `Missing` in return type of function `make`",
        ),
        (
            "unknown_struct_field_type.wave",
            "struct Holder { value: Missing; }\nfun main() {}\n",
            "unknown type `Missing` in field `Holder.value`",
        ),
        (
            "unknown_alias_target.wave",
            "type Alias = Missing;\nfun main() {}\n",
            "unknown type `Missing` in type alias `Alias`",
        ),
        (
            "unknown_proto_target.wave",
            "proto Missing { fun read(self: Missing) -> i32 { return 1; } }\nfun main() {}\n",
            "proto implementation targets unknown type `Missing`",
        ),
        (
            "void_variable.wave",
            "fun main() { var value: void; }\n",
            "variable `value` cannot use the `void` type",
        ),
        (
            "void_struct_field.wave",
            "struct Invalid { value: void; }\nfun main() {}\n",
            "field `Invalid.value` cannot use the `void` type",
        ),
        (
            "cyclic_pointer_alias.wave",
            "type Loop = ptr<Loop>;\nfun main() {}\n",
            "cyclic type alias involving `Loop`",
        ),
        (
            "float_enum_repr.wave",
            "enum Invalid -> f32 { Value = 1 }\nfun main() {}\n",
            "enum `Invalid` representation must be an integer type",
        ),
        (
            "out_of_range_literal.wave",
            "fun main() { let value: i8 = 300; }\n",
            "initializer for `value`",
        ),
        (
            "negative_out_of_range_literal.wave",
            "fun main() { let value: i8 = -129; }\n",
            "initializer for `value`",
        ),
        (
            "negative_unsigned_literal.wave",
            "fun main() { let value: u8 = -1; }\n",
            "initializer for `value`",
        ),
        (
            "wrong_addressed_array_element.wave",
            "fun values() -> ptr<array<i32, 1>> { return &[\"text\"]; }\nfun main() {}\n",
            "element 0 of return value of function `values`",
        ),
        (
            "duplicate_local.wave",
            "fun main() { let value: i32 = 1; let value: i32 = 2; }\n",
            "duplicate variable declaration `value` in the same scope",
        ),
        (
            "duplicate_struct_field.wave",
            "struct Pair { x: i32; x: i64; }\nfun main() {}\n",
            "duplicate field `x` in struct `Pair`",
        ),
        (
            "duplicate_enum_variant.wave",
            "enum Mode -> i32 { Same = 1, Same = 2 }\nfun main() {}\n",
            "duplicate variant `Same` in enum `Mode`",
        ),
        (
            "duplicate_method.wave",
            "struct Pair { x: i32; }\nproto Pair { fun read(self: Pair) -> i32 { return self.x; } fun read(self: Pair) -> i32 { return self.x; } }\nfun main() {}\n",
            "duplicate method `Pair.read`",
        ),
        (
            "duplicate_match_value.wave",
            "enum Mode -> i32 { First = 1, Second = 1 }\nfun main() { let mode: Mode = First; match (mode) { First => {} Second => {} } }\n",
            "duplicate match case pattern `value:1`",
        ),
        (
            "mixed_float_widths.wave",
            "fun add(left: f32, right: f64) -> f32 { return left + right; }\nfun main() {}\n",
            "mixed float widths require an explicit cast",
        ),
    ];

    for (file_name, source, expected) in cases {
        let source = write_wave(&dir, file_name, source);
        for mode in ["check", "build"] {
            let output = if mode == "check" {
                run_wavec_raw([OsStr::new("check"), source.as_os_str()])
            } else {
                run_wavec_raw([
                    OsStr::new("build"),
                    source.as_os_str(),
                    OsStr::new("--emit=obj"),
                    OsStr::new("--out-dir"),
                    dir.as_os_str(),
                ])
            };
            assert!(
                !output.status.success(),
                "{} unexpectedly succeeded in {} mode",
                file_name,
                mode
            );
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(stderr.contains("error[E3001]"), "{}: {}", file_name, stderr);
            assert!(stderr.contains(expected), "{}: {}", file_name, stderr);
            assert!(
                !stderr.contains("E9001")
                    && !stderr.contains("compiler internal error")
                    && !stderr.contains("panic location"),
                "{} leaked a backend failure in {} mode: {}",
                file_name,
                mode,
                stderr
            );
        }
    }
}

#[test]
fn second_semantic_audit_preserves_explicit_and_contextual_valid_cases() {
    let dir = temp_case_dir("semantic-audit-two-valid");
    let source = write_wave(
        &dir,
        "valid.wave",
        r#"
enum Mode -> i32 {
    First = 1,
    Alias = 1
}

fun noop() {}

fun add(left: f32, right: f64) -> f32 {
    return left + (right as f32);
}

fun main() {
    noop();
    let minimum: i8 = -128;
    let bit_pattern: i8 = 0xFF;
    let unsigned_max: u128 = 340282366920938463463374607431768211455;
    let explicit: i8 = 300 as i8;
    let values: ptr<array<i32, 2>> = &[1, 2];
    var input_value: i32 = 0;
    if (true) {
        let minimum: i32 = 1;
        println("{}", minimum);
    }
    println("{} {} {} {} {}", minimum, bit_pattern, unsigned_max, explicit, values);
}
"#,
    );

    run_wavec([OsStr::new("check"), source.as_os_str()]);
    run_wavec([
        OsStr::new("build"),
        source.as_os_str(),
        OsStr::new("--emit=obj"),
        OsStr::new("--out-dir"),
        dir.as_os_str(),
    ]);
}

#[test]
fn semantic_diagnostics_point_at_the_relevant_source() {
    let dir = temp_case_dir("semantic-source-position");
    let source = write_wave(
        &dir,
        "wrong_return.wave",
        r#"
fun value() -> i32 {
    return "text";
}
"#,
    );
    let output = run_wavec_raw([OsStr::new("check"), source.as_os_str()]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("wrong_return.wave:3:5"), "{}", stderr);
    assert!(stderr.contains("return \"text\";"), "{}", stderr);

    let repeated_return = write_wave(
        &dir,
        "repeated_return.wave",
        r#"
fun value(flag: bool) -> i32 {
    if (flag) {
        return 1;
    }
    return "text";
}
"#,
    );
    let output = run_wavec_raw([OsStr::new("check"), repeated_return.as_os_str()]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("repeated_return.wave:6:5"), "{}", stderr);
    assert!(stderr.contains("return \"text\";"), "{}", stderr);

    let duplicate = write_wave(
        &dir,
        "duplicate.wave",
        r#"
fun other() {
    let value: i32 = 0;
}

fun main() {
    let value: i32 = 1;
    let value: i32 = 2;
}
"#,
    );
    let output = run_wavec_raw([OsStr::new("check"), duplicate.as_os_str()]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("duplicate.wave:8:9"), "{}", stderr);
    assert!(stderr.contains("let value: i32 = 2;"), "{}", stderr);

    let duplicate_type = write_wave(
        &dir,
        "duplicate_type.wave",
        "struct Item { first: i32; }\nstruct Item { second: i32; }\nfun main() {}\n",
    );
    let output = run_wavec_raw([OsStr::new("check"), duplicate_type.as_os_str()]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("duplicate_type.wave:2:8"), "{}", stderr);
    assert!(
        stderr.contains("struct Item { second: i32; }"),
        "{}",
        stderr
    );

    let imported = dir.join("broken.wave");
    fs::write(
        &imported,
        "fun broken() {\n    let value: Missing = 1;\n}\n",
    )
    .unwrap();
    let entry = write_wave(
        &dir,
        "import_main.wave",
        "import(\"broken\");\n\nfun main() {}\n",
    );
    let output = run_wavec_raw([OsStr::new("check"), entry.as_os_str()]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("broken.wave:2:9"), "{}", stderr);
    assert!(stderr.contains("let value: Missing = 1;"), "{}", stderr);
    assert!(!stderr.contains("import_main.wave:1:1"), "{}", stderr);

    let generic_import = dir.join("generic_broken.wave");
    fs::write(
        &generic_import,
        "struct Box<T> { value: T; }\nfun broken() {\n    let value: Box<i32, i64>;\n}\n",
    )
    .unwrap();
    let generic_entry = write_wave(
        &dir,
        "generic_import_main.wave",
        "import(\"generic_broken\");\n\nfun main() {}\n",
    );
    let output = run_wavec_raw([OsStr::new("check"), generic_entry.as_os_str()]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("generic_broken.wave:3:9"), "{}", stderr);
    assert!(
        stderr.contains("type `Box` expects 1 generic argument(s), found 2"),
        "{}",
        stderr
    );
    assert!(
        !stderr.contains("generic monomorphization failed"),
        "{}",
        stderr
    );
}

#[test]
fn semantic_validation_accepts_complete_returns_and_explicit_casts() {
    let dir = temp_case_dir("semantic-validation-valid");
    let source = write_wave(
        &dir,
        "valid.wave",
        r#"
struct Pair {
    x: i32;
    y: i32;
}

fun choose(flag: bool) -> i32 {
    if (flag) {
        return 1;
    } else {
        return 2;
    }
    println("unreachable");
}

fun dead_after_return() {
    return;
    println("unreachable");
}

fun widen(value: i32) -> i64 {
    return value;
}

fun narrow_explicitly(value: i64) -> i32 {
    return value as i32;
}

fun values() -> array<i32, 2> {
    return [1, 2];
}

fun pointer_bits() -> i64 {
    return "text" as i64;
}

fun infinite() -> i32 {
    while (true) {
        continue;
    }
}

fun main() -> i32 {
    let value: i32 = choose(true);
    let bits: i64 = pointer_bits();
    let pair: Pair = Pair { x: 1, y: 2 };
    let items: array<i32, 2> = values();
    let pointer: ptr<Pair> = &pair;
    if (pointer) {
        if (value == 1 && bits != 0 && items[0] == 1 && widen(pair.y) == 2) {
            return narrow_explicitly(0);
        }
    }
    return infinite();
}
"#,
    );
    let out_dir = dir.join("out");
    run_wavec([
        OsStr::new("build"),
        source.as_os_str(),
        OsStr::new("--emit=ir"),
        OsStr::new("--out-dir"),
        out_dir.as_os_str(),
    ]);
}

#[test]
fn vex_cli_print_json_contracts_are_machine_readable() {
    let (stdout, stderr) = run_wavec_capture([
        OsStr::new("print"),
        OsStr::new("target-spec"),
        OsStr::new("--target"),
        OsStr::new("x86_64-unknown-linux-gnu"),
        OsStr::new("--format=json"),
    ]);
    assert!(stderr.trim().is_empty(), "unexpected stderr:\n{}", stderr);
    let json = stdout.trim();
    assert!(json.starts_with('{') && json.ends_with('}'), "{}", json);
    assert!(
        json.contains("\"triple\":\"x86_64-unknown-linux-gnu\""),
        "{}",
        json
    );
    assert!(json.contains("\"arch\":\"x86_64\""), "{}", json);
    assert!(json.contains("\"os\":\"linux\""), "{}", json);
    assert!(json.contains("\"env\":\"gnu\""), "{}", json);
    assert!(json.contains("\"object_format\":\"elf\""), "{}", json);
    assert!(json.contains("\"default_linker\":"), "{}", json);

    let (stdout, _) = run_wavec_capture([
        OsStr::new("print"),
        OsStr::new("supported-emit-kinds"),
        OsStr::new("--format=json"),
    ]);
    let emit_json = stdout.trim();
    assert!(
        emit_json.contains("\"check\"")
            && emit_json.contains("\"ir\"")
            && emit_json.contains("\"obj\"")
            && emit_json.contains("\"bin\""),
        "{}",
        emit_json
    );

    let (stdout, _) = run_wavec_capture([
        OsStr::new("print"),
        OsStr::new("supported-input-types"),
        OsStr::new("--format=json"),
    ]);
    let input_json = stdout.trim();
    assert!(
        input_json.contains("\"wave\"")
            && input_json.contains("\"ir\"")
            && input_json.contains("\"bc\"")
            && input_json.contains("\"asm\"")
            && input_json.contains("\"obj\"")
            && input_json.contains("\"archive\""),
        "{}",
        input_json
    );

    let (stdout, _) = run_wavec_capture([
        OsStr::new("print"),
        OsStr::new("supported-print-items"),
        OsStr::new("--format=json"),
    ]);
    let print_items = stdout.trim();
    assert!(
        print_items.contains("\"cpu-list\"")
            && print_items.contains("\"target-features\"")
            && print_items.contains("\"default-linker\"")
            && print_items.contains("\"supported-input-types\"")
            && print_items.contains("\"supported-emit-kinds\""),
        "{}",
        print_items
    );

    let (stdout, _) = run_wavec_capture([
        OsStr::new("print"),
        OsStr::new("cpu-list"),
        OsStr::new("--target"),
        OsStr::new("x86_64-unknown-linux-gnu"),
        OsStr::new("--format=json"),
    ]);
    assert!(stdout.trim().starts_with('['), "{}", stdout);

    let (stdout, _) = run_wavec_capture([
        OsStr::new("print"),
        OsStr::new("target-features"),
        OsStr::new("--target"),
        OsStr::new("x86_64-unknown-linux-gnu"),
        OsStr::new("--format=json"),
    ]);
    assert!(stdout.contains("\"sse2\""), "{}", stdout);
}

#[test]
fn vex_cli_json_errors_and_dry_run_are_stable() {
    let bad = run_wavec_raw([
        OsStr::new("--error-format=json"),
        OsStr::new("build"),
        OsStr::new("--bad-option"),
    ]);
    assert!(
        !bad.status.success(),
        "invalid CLI should fail\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&bad.stdout),
        String::from_utf8_lossy(&bad.stderr)
    );
    assert_eq!(bad.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&bad.stderr);
    assert!(stderr.contains("\"error\""), "{}", stderr);
    assert!(stderr.contains("\"kind\":\"usage\""), "{}", stderr);
    assert!(stderr.contains("\"exit_code\":2"), "{}", stderr);

    let dir = temp_case_dir("vex-dry-run-json");
    let src = write_wave(
        &dir,
        "main.wave",
        r#"
fun main() -> i32 {
    return 0;
}
"#,
    );
    let out_dir = dir.join("out");
    let (stdout, stderr) = run_wavec_capture([
        OsStr::new("--error-format=json"),
        OsStr::new("build"),
        src.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("x86_64-unknown-none-elf"),
        OsStr::new("--freestanding"),
        OsStr::new("--emit=obj"),
        OsStr::new("--dry-run"),
        OsStr::new("--out-dir"),
        out_dir.as_os_str(),
    ]);
    assert!(stderr.trim().is_empty(), "unexpected stderr:\n{}", stderr);
    let plan = stdout.trim();
    assert!(plan.starts_with('{') && plan.ends_with('}'), "{}", plan);
    assert!(plan.contains("\"schema_version\":1"), "{}", plan);
    assert!(
        plan.contains("\"target\":\"x86_64-unknown-none-elf\""),
        "{}",
        plan
    );
    assert!(plan.contains("\"cpu\":\"generic\""), "{}", plan);
    assert!(plan.contains("\"features\":\"\""), "{}", plan);
    assert!(plan.contains("\"abi\":null"), "{}", plan);
    assert!(plan.contains("\"isa\":null"), "{}", plan);
    assert!(plan.contains("\"mode\":\"compile-only\""), "{}", plan);
    assert!(plan.contains("\"emit_kinds\":[\"obj\"]"), "{}", plan);
    assert!(plan.contains("\"control_mode\":null"), "{}", plan);
    assert!(plan.contains("\"freestanding\":true"), "{}", plan);
    assert!(plan.contains("\"compile\""), "{}", plan);
    assert!(plan.contains("\"link\":null"), "{}", plan);
    assert!(plan.contains("\"execute\":null"), "{}", plan);

    let syntax_src = write_wave(
        &dir,
        "syntax_error.wave",
        r#"
fun main( {
}
"#,
    );
    let syntax = run_wavec_raw([
        OsStr::new("build"),
        syntax_src.as_os_str(),
        OsStr::new("--emit=check"),
        OsStr::new("--error-format=json"),
    ]);
    assert!(
        !syntax.status.success(),
        "syntax error should fail\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&syntax.stdout),
        String::from_utf8_lossy(&syntax.stderr)
    );
    let stderr = String::from_utf8_lossy(&syntax.stderr);
    assert!(stderr.contains("\"error\""), "{}", stderr);
    assert!(stderr.contains("\"kind\":\"syntax-error\""), "{}", stderr);
    assert!(stderr.contains("\"line\":"), "{}", stderr);
    assert!(stderr.contains("\"column\":"), "{}", stderr);

    let ordered_format = run_wavec_raw([
        OsStr::new("--error-format=human"),
        OsStr::new("build"),
        OsStr::new("--bad-option"),
        OsStr::new("--error-format=json"),
    ]);
    assert_eq!(ordered_format.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&ordered_format.stderr);
    assert!(
        stderr.contains("\"kind\":\"usage\""),
        "later build-level json format must control parser errors:\n{}",
        stderr
    );
}

#[test]
fn vex_cli_mixed_inputs_and_mode_conflicts_are_stable() {
    let dir = temp_case_dir("vex-cli-mixed-inputs");
    let src = write_wave(
        &dir,
        "main.wave",
        r#"
fun main() -> i32 {
    return 0;
}
"#,
    );
    let obj = dir.join("native.o");
    let archive = dir.join("libnative.a");
    fs::write(&obj, b"not-a-real-object").unwrap();
    fs::write(&archive, b"not-a-real-archive").unwrap();

    let out_dir = dir.join("artifacts");
    let target_dir = dir.join("intermediate");
    let binary = dir.join("app");
    let (stdout, stderr) = run_wavec_capture([
        OsStr::new("--error-format=json"),
        OsStr::new("build"),
        src.as_os_str(),
        obj.as_os_str(),
        archive.as_os_str(),
        OsStr::new("--emit=obj,bin"),
        OsStr::new("--dry-run"),
        OsStr::new("--out-dir"),
        out_dir.as_os_str(),
        OsStr::new("--target-dir"),
        target_dir.as_os_str(),
        OsStr::new("-o"),
        binary.as_os_str(),
    ]);
    assert!(stderr.trim().is_empty(), "unexpected stderr:\n{}", stderr);
    let plan = stdout.trim();
    assert!(plan.contains("\"kind\":\"wave\""), "{}", plan);
    assert!(plan.contains("\"kind\":\"obj\""), "{}", plan);
    assert!(plan.contains("\"kind\":\"archive\""), "{}", plan);
    assert!(
        plan.contains("\"emit_kinds\":[\"obj\",\"bin\"]"),
        "{}",
        plan
    );
    assert!(
        plan.contains(&format!(
            "\"output\":{}",
            json_string_for_test(&binary.to_string_lossy())
        )),
        "-o must apply to final linked binary only:\n{}",
        plan
    );
    assert!(
        plan.contains(&format!(
            "\"output\":{}",
            json_string_for_test(&out_dir.join("main.o").to_string_lossy())
        )),
        "obj artifact must follow --out-dir, not -o:\n{}",
        plan
    );
    assert!(
        plan.contains(&json_string_for_test(&archive.to_string_lossy())),
        "archive input must be forwarded to the link plan:\n{}",
        plan
    );

    let run_plan = run_wavec_raw([
        OsStr::new("--error-format=json"),
        OsStr::new("build"),
        src.as_os_str(),
        OsStr::new("--run"),
        OsStr::new("--dry-run"),
        OsStr::new("--"),
        OsStr::new("arg one"),
        OsStr::new("--flag"),
    ]);
    assert!(
        run_plan.status.success(),
        "run dry-run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run_plan.stdout),
        String::from_utf8_lossy(&run_plan.stderr)
    );
    let stdout = String::from_utf8_lossy(&run_plan.stdout);
    assert!(stdout.contains("\"mode\":\"build+run\""), "{}", stdout);
    assert!(stdout.contains("\"execute\":{"), "{}", stdout);
    assert!(
        stdout.contains("\"args\":[\"arg one\",\"--flag\"]"),
        "{}",
        stdout
    );

    let cases: &[(&str, &[&str], &[&str])] = &[
        (
            "check cannot be combined",
            &["build", "main.wave", "--emit=check,ast"],
            &["--emit=check", "alone"],
        ),
        (
            "link-only obj emit conflict",
            &["build", "native.o", "--link-only", "--emit=obj"],
            &["--link-only", "--emit=bin"],
        ),
        (
            "shared static conflict",
            &["build", "main.wave", "--shared", "--static", "--dry-run"],
            &["--shared", "--static"],
        ),
        (
            "run shared conflict",
            &["build", "main.wave", "--shared", "--run", "--dry-run"],
            &["--run", "--shared"],
        ),
        (
            "no-pie relocation conflict",
            &[
                "-C",
                "relocation-model=pie",
                "build",
                "main.wave",
                "--no-pie",
                "--dry-run",
            ],
            &["--no-pie", "relocation-model=pie"],
        ),
        (
            "forced input conflict",
            &["build", "native.o", "--input-type=wave", "--dry-run"],
            &["--input-type=wave", "conflicts"],
        ),
        (
            "unknown input inference",
            &["build", "unknown.blob", "--dry-run"],
            &["cannot infer input type", "--input-type"],
        ),
    ];

    for (name, args, needles) in cases {
        let full_args = std::iter::once("--error-format=json")
            .chain(args.iter().copied())
            .map(OsStr::new);
        let out = run_wavec_raw(full_args);
        assert!(
            !out.status.success(),
            "{} unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
            name,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(out.status.code(), Some(2), "{}", name);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("\"kind\":\"usage\""),
            "{}: {}",
            name,
            stderr
        );
        for needle in *needles {
            assert!(
                stderr.contains(needle),
                "{} missing '{}': {}",
                name,
                needle,
                stderr
            );
        }
    }
}

#[test]
fn target_attribute_supports_arch_os_env_and_abi_conditions() {
    let dir = temp_case_dir("target-attr-conditions");
    let src = write_wave(
        &dir,
        "select.wave",
        r#"
#[target(arch="x86_64", os="none", env="none")]
fun selected() -> i32 {
    return 64;
}

#[target(arch="aarch64", os="none", env="none")]
fun selected() -> i32 {
    return 128;
}

#[target(arch="riscv64", os="none", env="none", abi="lp64d")]
fun selected() -> i32 {
    return 255;
}

fun main() -> i32 {
    return selected();
}
"#,
    );

    let x86_dir = dir.join("x86");
    run_wavec([
        OsStr::new("build"),
        src.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("x86_64-unknown-none-elf"),
        OsStr::new("--emit=ir"),
        OsStr::new("--out-dir"),
        x86_dir.as_os_str(),
    ]);
    let x86_ir = fs::read_to_string(x86_dir.join("select.ll")).unwrap();
    assert!(x86_ir.contains("ret i32 64"), "{}", x86_ir);
    assert!(!x86_ir.contains("ret i32 128"), "{}", x86_ir);

    let arm_dir = dir.join("arm");
    run_wavec([
        OsStr::new("build"),
        src.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("aarch64-unknown-none-elf"),
        OsStr::new("--emit=ir"),
        OsStr::new("--out-dir"),
        arm_dir.as_os_str(),
    ]);
    let arm_ir = fs::read_to_string(arm_dir.join("select.ll")).unwrap();
    assert!(arm_ir.contains("ret i32 128"), "{}", arm_ir);
    assert!(!arm_ir.contains("ret i32 64"), "{}", arm_ir);

    let riscv_dir = dir.join("riscv");
    run_wavec([
        OsStr::new("build"),
        src.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("riscv64-unknown-none-elf"),
        OsStr::new("--abi"),
        OsStr::new("lp64d"),
        OsStr::new("--emit=ir"),
        OsStr::new("--out-dir"),
        riscv_dir.as_os_str(),
    ]);
    let riscv_ir = fs::read_to_string(riscv_dir.join("select.ll")).unwrap();
    assert!(riscv_ir.contains("ret i32 255"), "{}", riscv_ir);
}

#[test]
fn target_configuration_is_rejected_before_frontend_or_backend_work() {
    let dir = temp_case_dir("target-validation");
    let source = write_wave(&dir, "main.wave", "fun main() -> i32 { return 0; }\n");

    let run_failure = |options: &[&str], expected: &str| {
        let mut args = vec![
            OsString::from("--error-format=json"),
            OsString::from("build"),
            source.as_os_str().to_os_string(),
        ];
        args.extend(options.iter().map(OsString::from));
        let output = run_wavec_raw(args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "target validation should fail with usage exit code 2\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.trim().is_empty(),
            "JSON usage errors must not write to stdout: {stdout}"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("\"kind\":\"usage\""), "{}", stderr);
        assert!(stderr.contains(expected), "{}", stderr);
        assert!(
            !stderr.contains("E9001")
                && !stderr.contains("compiler internal error")
                && !stderr.contains("panic location"),
            "target validation leaked a backend failure: {}",
            stderr
        );
    };

    for mode in [
        &["--target", "mips64-unknown-linux-gnu", "--emit=check"][..],
        &["--target", "mips64-unknown-linux-gnu", "--emit=obj"][..],
        &[
            "--target",
            "mips64-unknown-linux-gnu",
            "--emit=obj",
            "--dry-run",
        ][..],
    ] {
        run_failure(mode, "unsupported target 'mips64-unknown-linux-gnu'");
    }

    for (options, expected) in [
        (
            &["--target", "x86_64-garbage-linux-gnu", "--emit=check"][..],
            "unsupported target 'x86_64-garbage-linux-gnu'",
        ),
        (
            &["--target", "riscv64-unknown-linux-musl", "--emit=check"][..],
            "unsupported target 'riscv64-unknown-linux-musl'",
        ),
        (
            &[
                "--target",
                "x86_64-unknown-linux-gnu",
                "--cpu",
                "sifive-u74",
                "--emit=check",
            ][..],
            "unsupported CPU 'sifive-u74'",
        ),
        (
            &[
                "--target",
                "riscv64-unknown-none-elf",
                "--cpu",
                "rocket",
                "--emit=obj",
            ][..],
            "unsupported CPU 'rocket'",
        ),
        (
            &[
                "--target",
                "x86_64-unknown-linux-gnu",
                "--features",
                "+m",
                "--emit=check",
            ][..],
            "unsupported feature 'm'",
        ),
        (
            &[
                "--target",
                "aarch64-unknown-linux-gnu",
                "--features",
                "+fp",
                "--emit=obj",
            ][..],
            "unsupported feature 'fp'",
        ),
        (
            &[
                "--target",
                "x86_64-unknown-linux-gnu",
                "--abi",
                "lp64d",
                "--emit=check",
            ][..],
            "unsupported ABI 'lp64d'",
        ),
        (
            &[
                "--target",
                "x86_64-unknown-linux-gnu",
                "--features",
                "sse2",
                "--emit=check",
            ][..],
            "invalid target feature 'sse2'",
        ),
        (
            &[
                "--target",
                "x86_64-unknown-linux-gnu",
                "--features",
                "+sse2,-sse2",
                "--emit=check",
            ][..],
            "target feature 'sse2' is specified more than once",
        ),
        (
            &[
                "--target",
                "x86_64-unknown-linux-gnu",
                "--features",
                "+sse2,,+avx",
                "--emit=check",
            ][..],
            "invalid empty target feature",
        ),
        (
            &[
                "--target",
                "riscv64-unknown-none-elf",
                "--features",
                "+m,+a,-f,+d,+c",
                "--abi",
                "lp64d",
                "--emit=check",
            ][..],
            "feature 'd' requires feature 'f'",
        ),
        (
            &[
                "--target",
                "riscv64-unknown-none-elf",
                "--features",
                "+f,-d",
                "--abi",
                "lp64d",
                "--emit=check",
            ][..],
            "ABI 'lp64d' for target 'riscv64-unknown-none-elf' requires features 'f' and 'd'",
        ),
        (
            &[
                "--target",
                "riscv64-unknown-none-elf",
                "--features",
                "-f",
                "--abi",
                "lp64f",
                "--emit=check",
            ][..],
            "ABI 'lp64f' for target 'riscv64-unknown-none-elf' requires feature 'f'",
        ),
        (
            &[
                "--target",
                "riscv64-unknown-linux-gnu",
                "--features",
                "-zicsr",
                "--emit=check",
            ][..],
            "feature 'f' requires feature 'zicsr'",
        ),
    ] {
        run_failure(options, expected);
    }

    let x86_out = dir.join("valid-x86");
    run_wavec([
        OsStr::new("build"),
        source.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("x86_64-unknown-linux-gnu"),
        OsStr::new("--cpu"),
        OsStr::new("x86-64-v2"),
        OsStr::new("--features"),
        OsStr::new("+sse2,-avx"),
        OsStr::new("--emit=obj"),
        OsStr::new("--out-dir"),
        x86_out.as_os_str(),
    ]);
    assert!(x86_out.join("main.o").is_file());

    let riscv_out = dir.join("valid-riscv");
    run_wavec([
        OsStr::new("build"),
        source.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("riscv64-unknown-none-elf"),
        OsStr::new("--cpu"),
        OsStr::new("sifive-u74"),
        OsStr::new("--features"),
        OsStr::new("+m,+a,+f,+d,+c"),
        OsStr::new("--abi"),
        OsStr::new("lp64d"),
        OsStr::new("--emit=obj"),
        OsStr::new("--out-dir"),
        riscv_out.as_os_str(),
    ]);
    assert!(riscv_out.join("main.o").is_file());

    let linux_riscv_out = dir.join("valid-linux-riscv");
    run_wavec([
        OsStr::new("build"),
        source.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("riscv64-unknown-linux-gnu"),
        OsStr::new("--emit=obj"),
        OsStr::new("--out-dir"),
        linux_riscv_out.as_os_str(),
    ]);
    let linux_riscv_object = linux_riscv_out.join("main.o");
    assert!(linux_riscv_object.is_file());
    assert_eq!(riscv64_elf_flags(&linux_riscv_object) & 0x7, 0x5);
    let linux_riscv_bytes = fs::read(&linux_riscv_object).unwrap();
    assert!(bytes_contains(&linux_riscv_bytes, b"zicsr"));
    assert!(bytes_contains(&linux_riscv_bytes, b"zifencei"));

    let (stdout, stderr) = run_wavec_capture([
        OsStr::new("print"),
        OsStr::new("target-spec"),
        OsStr::new("--target"),
        OsStr::new("riscv64-unknown-none-elf"),
        OsStr::new("--format=json"),
    ]);
    assert!(stderr.trim().is_empty(), "{}", stderr);
    assert!(stdout.contains("\"hosted\":false"), "{}", stdout);
    assert!(stdout.contains("\"freestanding\":true"), "{}", stdout);
    assert!(stdout.contains("\"cpu\":\"generic-rv64\""), "{}", stdout);
    assert!(
        stdout.contains("\"features\":\"+m,+a,-f,-d,+c,-zicsr,-zifencei\""),
        "{}",
        stdout
    );
    assert!(stdout.contains("\"abi\":\"lp64\""), "{}", stdout);
    assert!(stdout.contains("\"isa\":\"rv64imac\""), "{}", stdout);

    let (stdout, stderr) = run_wavec_capture([
        OsStr::new("print"),
        OsStr::new("target-spec"),
        OsStr::new("--target"),
        OsStr::new("riscv64-unknown-linux-gnu"),
        OsStr::new("--format=json"),
    ]);
    assert!(stderr.trim().is_empty(), "{}", stderr);
    assert!(stdout.contains("\"hosted\":true"), "{}", stdout);
    assert!(stdout.contains("\"freestanding\":false"), "{}", stdout);
    assert!(
        stdout.contains("\"features\":\"+m,+a,+f,+d,+c,+zicsr,+zifencei\""),
        "{}",
        stdout
    );
    assert!(stdout.contains("\"abi\":\"lp64d\""), "{}", stdout);
    assert!(stdout.contains("\"isa\":\"rv64gc\""), "{}", stdout);

    let (stdout, stderr) = run_wavec_capture([
        OsStr::new("--error-format=json"),
        OsStr::new("build"),
        source.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("riscv64-unknown-linux-gnu"),
        OsStr::new("--emit=obj"),
        OsStr::new("--dry-run"),
    ]);
    assert!(stderr.trim().is_empty(), "{}", stderr);
    assert!(stdout.contains("\"cpu\":\"generic-rv64\""), "{}", stdout);
    assert!(
        stdout.contains("\"features\":\"+m,+a,+f,+d,+c,+zicsr,+zifencei\""),
        "{}",
        stdout
    );
    assert!(stdout.contains("\"abi\":\"lp64d\""), "{}", stdout);
    assert!(stdout.contains("\"isa\":\"rv64gc\""), "{}", stdout);

    let (stdout, stderr) = run_wavec_capture([
        OsStr::new("--error-format=json"),
        OsStr::new("build"),
        source.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("riscv64-unknown-linux-gnu"),
        OsStr::new("--abi=lp64"),
        OsStr::new("--emit=bin"),
        OsStr::new("--dry-run"),
    ]);
    assert!(stderr.trim().is_empty(), "{}", stderr);
    assert!(
        stdout.contains("--dynamic-linker=/lib/ld-linux-riscv64-lp64.so.1"),
        "{}",
        stdout
    );
    assert!(
        !stdout.contains("ld-linux-riscv64-lp64d.so.1"),
        "{}",
        stdout
    );
    for host_path in [
        "/usr/lib64/crt1.o",
        "/usr/lib64/crti.o",
        "/usr/lib64/crtn.o",
        "-L/usr/lib64",
        "-L/lib64",
        "-L/usr/lib ",
        "-L/lib ",
    ] {
        assert!(
            !stdout.contains(host_path),
            "cross-target link plan consumed host runtime path '{}':\n{}",
            host_path,
            stdout
        );
    }

    let (stdout, stderr) = run_wavec_capture([
        OsStr::new("--error-format=json"),
        OsStr::new("build"),
        source.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("riscv64-unknown-linux-gnu"),
        OsStr::new("--static"),
        OsStr::new("--emit=bin"),
        OsStr::new("--dry-run"),
    ]);
    assert!(stderr.trim().is_empty(), "{}", stderr);
    assert!(stdout.contains("-static"), "{}", stdout);
    assert!(
        !stdout.contains("--dynamic-linker="),
        "static link plan must not select a dynamic loader:\n{}",
        stdout
    );
    assert!(
        json_contains_path_components(&stdout, &["crt", "riscv64-unknown-linux-gnu", "crt1.o"]),
        "static link plan must retain a CRT entry point:\n{}",
        stdout
    );

    let (stdout, stderr) = run_wavec_capture([
        OsStr::new("--error-format=json"),
        OsStr::new("build"),
        source.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("riscv64-unknown-linux-gnu"),
        OsStr::new("--static"),
        OsStr::new("--pie"),
        OsStr::new("--emit=bin"),
        OsStr::new("--dry-run"),
    ]);
    assert!(stderr.trim().is_empty(), "{}", stderr);
    assert!(
        stdout.contains("rcrt1.o"),
        "static PIE link plan must use the relocatable CRT entry point:\n{}",
        stdout
    );
}

#[test]
fn riscv64_debian_cross_prefix_does_not_double_apply_linker_sysroot() {
    let dir = temp_case_dir("riscv64-debian-cross-prefix");
    let source = write_wave(&dir, "main.wave", "fun main() -> i32 { return 0; }\n");
    let sysroot = dir.join("usr").join("riscv64-linux-gnu");
    let runtime = sysroot.join("lib");
    fs::create_dir_all(&runtime).unwrap();

    for crt in ["crt1.o", "crti.o", "crtn.o"] {
        write_minimal_elf64_object(&runtime.join(crt), 243);
    }
    let runtime_prefix = runtime.to_string_lossy();
    fs::write(
        runtime.join("libc.so"),
        format!(
            "GROUP ( {runtime_prefix}/libc.so.6 {runtime_prefix}/libc_nonshared.a \
             AS_NEEDED ( {runtime_prefix}/ld-linux-riscv64-lp64d.so.1 ) )\n"
        ),
    )
    .unwrap();
    fs::write(
        runtime.join("libm.so"),
        format!("GROUP ( {runtime_prefix}/libm.so.6 )\n"),
    )
    .unwrap();
    for runtime_file in [
        "libc.so.6",
        "libc_nonshared.a",
        "libm.so.6",
        "ld-linux-riscv64-lp64d.so.1",
    ] {
        fs::write(runtime.join(runtime_file), []).unwrap();
    }

    let (stdout, stderr) = run_wavec_capture([
        OsStr::new("--error-format=json"),
        OsStr::new("build"),
        source.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("riscv64-unknown-linux-gnu"),
        OsStr::new("--sysroot"),
        sysroot.as_os_str(),
        OsStr::new("--emit=bin"),
        OsStr::new("--dry-run"),
    ]);
    assert!(stderr.trim().is_empty(), "{}", stderr);
    assert!(stdout.contains("--sysroot=/"), "{}", stdout);
    assert!(
        !stdout.contains(&format!("--sysroot={}", sysroot.display())),
        "cross prefix must not be applied twice by ld.lld:\n{}",
        stdout
    );
    assert!(
        stdout.contains(&format!("-L{}", runtime.display())),
        "target runtime search path must remain isolated to the cross prefix:\n{}",
        stdout
    );
}

#[test]
fn advertised_target_options_reach_object_codegen_without_backend_diagnostics() {
    let dir = temp_case_dir("target-option-matrix");
    let source = write_wave(&dir, "matrix.wave", "fun main() -> i32 { return 0; }\n");

    let build_object = |target: &str, label: &str, options: &[OsString]| -> PathBuf {
        let out_dir = dir.join(label);
        let mut args = vec![
            OsString::from("build"),
            source.as_os_str().to_os_string(),
            OsString::from("--target"),
            OsString::from(target),
        ];
        args.extend_from_slice(options);
        args.extend([
            OsString::from("--emit=obj"),
            OsString::from("--out-dir"),
            out_dir.as_os_str().to_os_string(),
        ]);

        let (stdout, stderr) = run_wavec_capture(args);
        assert!(stdout.trim().is_empty(), "{}", stdout);
        assert!(
            stderr.trim().is_empty(),
            "advertised target option emitted a backend diagnostic for {target}: {stderr}"
        );
        let object = out_dir.join("matrix.o");
        assert!(object.is_file(), "{target} {label}");
        object
    };

    let (targets, stderr) = run_wavec_capture([OsStr::new("print"), OsStr::new("target-list")]);
    assert!(stderr.trim().is_empty(), "{}", stderr);

    for target in targets.lines().filter(|line| !line.is_empty()) {
        let target_label = target.replace('-', "_");
        let object = build_object(target, &format!("{target_label}_default"), &[]);

        let (target_spec, stderr) = run_wavec_capture([
            OsStr::new("print"),
            OsStr::new("target-spec"),
            OsStr::new("--target"),
            OsStr::new(target),
            OsStr::new("--format=json"),
        ]);
        assert!(stderr.trim().is_empty(), "{}", stderr);
        let object = fs::read(object).unwrap();
        if target_spec.contains("\"object_format\":\"elf\"") {
            assert!(object.starts_with(b"\x7fELF"), "{target}: {target_spec}");
        } else if target_spec.contains("\"object_format\":\"macho\"") {
            assert!(
                object.starts_with(&[0xcf, 0xfa, 0xed, 0xfe]),
                "{target}: {target_spec}"
            );
        } else if target_spec.contains("\"object_format\":\"coff\"") {
            assert!(object.starts_with(&[0x64, 0x86]), "{target}: {target_spec}");
        } else {
            panic!("target spec has an unknown object format: {target_spec}");
        }

        let (cpus, stderr) = run_wavec_capture([
            OsStr::new("print"),
            OsStr::new("cpu-list"),
            OsStr::new("--target"),
            OsStr::new(target),
        ]);
        assert!(stderr.trim().is_empty(), "{}", stderr);
        for cpu in cpus.lines().filter(|line| !line.is_empty()) {
            let cpu_label = cpu.replace('-', "_");
            build_object(
                target,
                &format!("{target_label}_cpu_{cpu_label}"),
                &[OsString::from("--cpu"), OsString::from(cpu)],
            );
        }

        let (features, stderr) = run_wavec_capture([
            OsStr::new("print"),
            OsStr::new("target-features"),
            OsStr::new("--target"),
            OsStr::new(target),
        ]);
        assert!(stderr.trim().is_empty(), "{}", stderr);
        for feature in features.lines().filter(|line| !line.is_empty()) {
            let feature_label = feature.replace(['-', '.'], "_");
            for (sign, action) in [("+", "enable"), ("-", "disable")] {
                let setting = match (target.starts_with("riscv64-"), feature, sign) {
                    (true, "f", "-") => "-f,-d".to_string(),
                    (true, "d", "+") => "+f,+d".to_string(),
                    (true, "zicsr", "-") => "-f,-d,-zicsr".to_string(),
                    _ => format!("{sign}{feature}"),
                };
                build_object(
                    target,
                    &format!("{target_label}_feature_{action}_{feature_label}"),
                    &[OsString::from("--features"), OsString::from(setting)],
                );
            }
        }
    }

    #[cfg(any(feature = "llvm-target-all", feature = "llvm-target-riscv"))]
    {
        for (abi, features, expected_flags) in [
            ("lp64", "+m,+a,-f,-d,+c", 0x1),
            ("lp64f", "+m,+a,+f,-d,+c", 0x3),
            ("lp64d", "+m,+a,+f,+d,+c", 0x5),
        ] {
            for target in ["riscv64-unknown-linux-gnu", "riscv64-unknown-none-elf"] {
                let object = build_object(
                    target,
                    &format!("{}_abi_{abi}", target.replace('-', "_")),
                    &[
                        OsString::from("--features"),
                        OsString::from(features),
                        OsString::from("--abi"),
                        OsString::from(abi),
                    ],
                );
                assert_eq!(riscv64_elf_flags(&object) & 0x7, expected_flags);
            }
        }
    }
}

#[test]
fn lvalue_store_updates_deref_index_and_struct_fields() {
    let dir = temp_case_dir("lvalue-store");
    let src = write_wave(
        &dir,
        "lvalue_store.wave",
        r#"
struct Pair {
    a: i32;
    b: i32;
}

struct PointerBox {
    data: ptr<i32>;
}

fun write_deref(p: ptr<i32>, v: i32) {
    deref p = v;
}

fun write_index(p: ptr<i32>, v: i32) {
    p[1] = v;
}

fun write_field(p: ptr<Pair>, v: i32) {
    p.b = v;
}

fun write_array_pointer(p: ptr<array<i32, 3>>, v: i32) {
    deref p[1] = v;
}

fun write_pointer_field(p: ptr<PointerBox>, value: ptr<i32>) {
    deref p.data = value;
}

fun id_ptr(p: ptr<i32>) -> ptr<i32> {
    return p;
}

fun main() -> i32 {
    let mut x: i32 = 1;
    write_deref(&x, 41);
    if (x != 41) {
        return 1;
    }

    let mut arr: array<i32, 3> = [1, 2, 3];
    write_index(&arr[0], 9);
    if (arr[1] != 9) {
        return 2;
    }

    let mut pair: Pair = Pair { a: 7, b: 8 };
    write_field(&pair, 99);
    if (pair.b != 99) {
        return 3;
    }

    pair.a = 12;
    if (pair.a != 12) {
        return 4;
    }

    deref id_ptr(&x) = 77;
    if (x != 77) {
        return 5;
    }

    let mut array_ptr_target: array<i32, 3> = [4, 5, 6];
    write_array_pointer(&array_ptr_target, 23);
    if (array_ptr_target[1] != 23) {
        return 6;
    }

    let mut y: i32 = 88;
    let mut pointer_box: PointerBox = PointerBox { data: &x };
    write_pointer_field(&pointer_box, &y);
    if (pointer_box.data != &y) {
        return 7;
    }

    return 0;
}
"#,
    );

    let ir_dir = dir.join("ir");
    run_wavec([
        OsStr::new("build"),
        src.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("x86_64-unknown-none-elf"),
        OsStr::new("--freestanding"),
        OsStr::new("--emit=ir"),
        OsStr::new("--out-dir"),
        ir_dir.as_os_str(),
    ]);

    let ir = fs::read_to_string(ir_dir.join("lvalue_store.ll")).unwrap();
    assert!(
        ir.contains("store i32") && ir.contains("getelementptr"),
        "lvalue store regression should generate store/GEP operations without requiring a host linker:\n{}",
        ir
    );

    if run_link_tests_enabled() {
        let target_dir = dir.join("target");
        run_wavec([
            OsStr::new("build"),
            src.as_os_str(),
            OsStr::new("--run"),
            OsStr::new("--target-dir"),
            target_dir.as_os_str(),
        ]);
    }
}

#[test]
fn freestanding_codegen_marks_functions_no_red_zone() {
    let dir = temp_case_dir("freestanding-noredzone");
    let src = write_wave(
        &dir,
        "leaf.wave",
        r#"
fun leaf(a: i64, b: i64, c: i64, d: i64, e: i64) -> i64 {
    let x: i64 = a + b;
    let y: i64 = c + d;
    return x + y + e;
}
"#,
    );

    let explicit_dir = dir.join("explicit");
    run_wavec([
        OsStr::new("build"),
        src.as_os_str(),
        OsStr::new("--freestanding"),
        OsStr::new("--emit=ir"),
        OsStr::new("--out-dir"),
        explicit_dir.as_os_str(),
    ]);
    let explicit_out = explicit_dir.join("leaf.ll");
    let explicit_ir = fs::read_to_string(&explicit_out).unwrap();
    assert!(
        explicit_ir.contains("noredzone"),
        "--freestanding IR must carry the LLVM noredzone function attribute:\n{}",
        explicit_ir
    );
    assert!(
        explicit_ir.contains("nounwind"),
        "--freestanding IR must mark Wave functions nounwind:\n{}",
        explicit_ir
    );

    let bare_dir = dir.join("bare");
    run_wavec([
        OsStr::new("build"),
        src.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("x86_64-unknown-none-elf"),
        OsStr::new("--emit=ir"),
        OsStr::new("--out-dir"),
        bare_dir.as_os_str(),
    ]);
    let bare_out = bare_dir.join("leaf.ll");
    let bare_ir = fs::read_to_string(&bare_out).unwrap();
    assert!(
        bare_ir.contains("noredzone"),
        "bare-metal target IR must carry the LLVM noredzone function attribute:\n{}",
        bare_ir
    );
    assert!(
        bare_ir.contains("nounwind"),
        "bare-metal target IR must mark Wave functions nounwind:\n{}",
        bare_ir
    );
}

#[test]
fn inline_asm_requires_explicit_stack_contract() {
    let dir = temp_case_dir("asm-stack-contract");
    let bad_src = write_wave(
        &dir,
        "bad_stack.wave",
        r#"
fun main() {
    asm {
        "sub rsp, 8"
        "add rsp, 8"
    }
}
"#,
    );

    let bad_dir = dir.join("bad");
    let err = run_wavec_expect_failure([
        OsStr::new("build"),
        bad_src.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("x86_64-unknown-linux-gnu"),
        OsStr::new("--emit=ir"),
        OsStr::new("--out-dir"),
        bad_dir.as_os_str(),
    ]);
    assert!(
        err.contains("clobber(\\\"stack\\\")") || err.contains("clobber(\"stack\")"),
        "stack contract diagnostic should mention clobber(\"stack\"):\n{}",
        err
    );

    let good_src = write_wave(
        &dir,
        "good_stack.wave",
        r#"
fun main() {
    asm {
        "sub rsp, 8"
        "add rsp, 8"
        clobber("stack")
    }
}
"#,
    );
    let good_dir = dir.join("good");
    run_wavec([
        OsStr::new("build"),
        good_src.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("x86_64-unknown-linux-gnu"),
        OsStr::new("--emit=ir"),
        OsStr::new("--out-dir"),
        good_dir.as_os_str(),
    ]);
    let ir = fs::read_to_string(good_dir.join("good_stack.ll")).unwrap();
    assert!(
        ir.contains("asm sideeffect alignstack"),
        "stack-declared inline asm should be volatile and alignstack:\n{}",
        ir
    );

    let unbalanced_src = write_wave(
        &dir,
        "unbalanced_stack.wave",
        r#"
fun main() {
    asm {
        "sub rsp, 8"
        clobber("stack")
    }
}
"#,
    );
    let unbalanced_dir = dir.join("unbalanced");
    let err = run_wavec_expect_failure([
        OsStr::new("build"),
        unbalanced_src.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("x86_64-unknown-linux-gnu"),
        OsStr::new("--emit=ir"),
        OsStr::new("--out-dir"),
        unbalanced_dir.as_os_str(),
    ]);
    assert!(
        err.contains("stack delta is not balanced"),
        "unbalanced stack asm should be rejected:\n{}",
        err
    );

    let missing_noreturn_src = write_wave(
        &dir,
        "missing_noreturn.wave",
        r#"
fun main() {
    asm {
        "jmp rax"
        in("rax") 0
    }
}
"#,
    );
    let missing_noreturn_dir = dir.join("missing-noreturn");
    let err = run_wavec_expect_failure([
        OsStr::new("build"),
        missing_noreturn_src.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("x86_64-unknown-linux-gnu"),
        OsStr::new("--emit=ir"),
        OsStr::new("--out-dir"),
        missing_noreturn_dir.as_os_str(),
    ]);
    assert!(
        err.contains("clobber(\\\"noreturn\\\")") || err.contains("clobber(\"noreturn\")"),
        "non-returning asm should require clobber(\"noreturn\"):\n{}",
        err
    );

    let noreturn_src = write_wave(
        &dir,
        "noreturn.wave",
        r#"
fun jump_out(addr: u64) {
    asm {
        "jmp rax"
        in("rax") addr
        clobber("noreturn")
    }
}
"#,
    );
    let noreturn_dir = dir.join("noreturn");
    run_wavec([
        OsStr::new("build"),
        noreturn_src.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("x86_64-unknown-linux-gnu"),
        OsStr::new("--emit=ir"),
        OsStr::new("--out-dir"),
        noreturn_dir.as_os_str(),
    ]);
    let ir = fs::read_to_string(noreturn_dir.join("noreturn.ll")).unwrap();
    assert!(
        ir.contains("unreachable"),
        "noreturn inline asm should terminate the current IR block:\n{}",
        ir
    );
}

#[test]
fn inline_asm_rejects_invalid_contracts_and_allows_local_jumps() {
    let dir = temp_case_dir("asm-contract-extra");

    let local_jump_src = write_wave(
        &dir,
        "local_jump.wave",
        r#"
fun main() {
    asm {
        "jmp 1f"
        "1:"
    }
}

"#,
    );
    let local_jump_dir = dir.join("local-jump");
    run_wavec([
        OsStr::new("build"),
        local_jump_src.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("x86_64-unknown-linux-gnu"),
        OsStr::new("--emit=ir"),
        OsStr::new("--out-dir"),
        local_jump_dir.as_os_str(),
    ]);

    let conflict_src = write_wave(
        &dir,
        "conflicting_stack.wave",
        r#"
fun main() {
    asm {
        "nop"
        clobber("stack")
        clobber("nostack")
    }
}
"#,
    );
    let conflict_dir = dir.join("conflict");
    let err = run_wavec_expect_failure([
        OsStr::new("build"),
        conflict_src.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("x86_64-unknown-linux-gnu"),
        OsStr::new("--emit=ir"),
        OsStr::new("--out-dir"),
        conflict_dir.as_os_str(),
    ]);
    assert!(
        err.contains("cannot declare both"),
        "stack/nostack conflict should be rejected:\n{}",
        err
    );

    let expr_noreturn_src = write_wave(
        &dir,
        "expr_noreturn.wave",
        r#"
fun main() -> i64 {
    let x: i64 = asm {
        "jmp rax"
        in("rax") 0
        clobber("noreturn")
    };
    return x;
}
"#,
    );
    let expr_noreturn_dir = dir.join("expr-noreturn");
    let err = run_wavec_expect_failure([
        OsStr::new("build"),
        expr_noreturn_src.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("x86_64-unknown-linux-gnu"),
        OsStr::new("--emit=ir"),
        OsStr::new("--out-dir"),
        expr_noreturn_dir.as_os_str(),
    ]);
    assert!(
        err.contains("asm expression cannot declare"),
        "asm expressions must reject noreturn:\n{}",
        err
    );

    let clobber_operand_conflict_src = write_wave(
        &dir,
        "clobber_operand_conflict.wave",
        r#"
fun main() {
    let x: i64 = 1;
    asm {
        "mov rax, rax"
        in("rax") x
        clobber("rax")
    }
}
"#,
    );
    let clobber_operand_conflict_dir = dir.join("clobber-operand-conflict");
    let err = run_wavec_expect_failure([
        OsStr::new("build"),
        clobber_operand_conflict_src.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("x86_64-unknown-linux-gnu"),
        OsStr::new("--emit=ir"),
        OsStr::new("--out-dir"),
        clobber_operand_conflict_dir.as_os_str(),
    ]);
    assert!(
        err.contains("conflicts with an input/output operand register"),
        "clobber/operand register conflict should be rejected:\n{}",
        err
    );
}

#[test]
fn riscv64_inline_asm_enforces_reserved_registers_fprs_and_indirect_jumps() {
    let dir = temp_case_dir("riscv64-asm-contract");

    for reserved in ["sp", "zero"] {
        let source = write_wave(
            &dir,
            &format!("reserved_{}.wave", reserved),
            &format!(
                r#"
fun bind(value: u64) {{
    asm {{
        "nop"
        in("{reserved}") value
    }}
}}

fun main() {{}}
"#
            ),
        );
        let reserved_out = dir.join(format!("reserved-{}", reserved));
        let error = run_wavec_expect_failure([
            OsStr::new("--error-format=json"),
            OsStr::new("build"),
            source.as_os_str(),
            OsStr::new("--target"),
            OsStr::new("riscv64-unknown-linux-gnu"),
            OsStr::new("--emit=obj"),
            OsStr::new("--out-dir"),
            reserved_out.as_os_str(),
        ]);
        assert!(error.contains("\"code\":\"E3401\""), "{}", error);
        assert!(!error.contains("compiler internal error"), "{}", error);
    }

    let float_source = write_wave(
        &dir,
        "float_register.wave",
        r#"
fun consume(value: f64) {
    asm {
        "fmv.d fa0, fa0"
        in("fa0") value
    }
}

fun main() {}
"#,
    );
    let float_out = dir.join("float-register");
    run_wavec([
        OsStr::new("build"),
        float_source.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("riscv64-unknown-linux-gnu"),
        OsStr::new("--abi=lp64d"),
        OsStr::new("--emit=obj"),
        OsStr::new("--out-dir"),
        float_out.as_os_str(),
    ]);
    assert!(float_out.join("float_register.o").is_file());

    let jump_source = write_wave(
        &dir,
        "jalr.wave",
        r#"
fun jump(addr: u64) {
    asm {
        "jalr x0, 0(a0)"
        in("a0") addr
        clobber("stack")
    }
}

fun main() {}
"#,
    );
    let jump_out = dir.join("jalr-missing-noreturn");
    let error = run_wavec_expect_failure([
        OsStr::new("build"),
        jump_source.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("riscv64-unknown-none-elf"),
        OsStr::new("--emit=ir"),
        OsStr::new("--out-dir"),
        jump_out.as_os_str(),
    ]);
    assert!(error.contains("clobber(\"noreturn\")"), "{}", error);

    let noreturn_source = write_wave(
        &dir,
        "jalr_noreturn.wave",
        r#"
fun jump(addr: u64) {
    asm {
        "jalr x0, 0(a0)"
        in("a0") addr
        clobber("stack")
        clobber("noreturn")
    }
}

fun main() {}
"#,
    );
    let noreturn_out = dir.join("jalr-noreturn");
    run_wavec([
        OsStr::new("build"),
        noreturn_source.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("riscv64-unknown-none-elf"),
        OsStr::new("--emit=ir"),
        OsStr::new("--out-dir"),
        noreturn_out.as_os_str(),
    ]);
    let ir = fs::read_to_string(noreturn_out.join("jalr_noreturn.ll")).unwrap();
    assert!(ir.contains("unreachable"), "{}", ir);
}

#[test]
fn riscv64_ir_and_bitcode_preserve_target_contract_when_recompiled() {
    let dir = temp_case_dir("riscv64-artifact-contract");
    let source = write_wave(&dir, "main.wave", "fun main() -> i32 { return 0; }\n");
    let original = dir.join("original");
    run_wavec([
        OsStr::new("build"),
        source.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("riscv64-unknown-linux-gnu"),
        OsStr::new("--abi=lp64f"),
        OsStr::new("--emit=ir,bc,obj"),
        OsStr::new("--out-dir"),
        original.as_os_str(),
    ]);

    let ir = fs::read_to_string(original.join("main.ll")).unwrap();
    assert!(ir.contains("!\"target-abi\", !\"lp64f\""), "{}", ir);
    assert!(ir.contains("!\"riscv-isa\""), "{}", ir);
    assert!(ir.contains("\"target-cpu\"=\"generic-rv64\""), "{}", ir);
    assert!(
        ir.contains("\"target-features\"=\"+m,+a,+f,-d,+c,+zicsr,+zifencei\""),
        "{}",
        ir
    );

    let from_ir = dir.join("from-ir");
    let ir_input = original.join("main.ll");
    run_wavec([
        OsStr::new("build"),
        ir_input.as_os_str(),
        OsStr::new("--emit=obj"),
        OsStr::new("--out-dir"),
        from_ir.as_os_str(),
    ]);
    let from_bc = dir.join("from-bc");
    let bitcode_input = original.join("main.bc");
    run_wavec([
        OsStr::new("build"),
        bitcode_input.as_os_str(),
        OsStr::new("--emit=obj"),
        OsStr::new("--out-dir"),
        from_bc.as_os_str(),
    ]);

    for object in [
        original.join("main.o"),
        from_ir.join("main.o"),
        from_bc.join("main.o"),
    ] {
        assert_eq!(
            riscv64_elf_flags(&object) & 0x7,
            0x3,
            "{}",
            object.display()
        );
        let bytes = fs::read(&object).unwrap();
        assert!(bytes_contains(&bytes, b"zicsr"), "{}", object.display());
        assert!(bytes_contains(&bytes, b"zifencei"), "{}", object.display());
    }
}

#[test]
fn riscv64_export_c_uses_indirect_aggregate_parameters_and_sret() {
    let dir = temp_case_dir("riscv64-export-c-aggregate");
    let source = write_wave(
        &dir,
        "aggregate.wave",
        r#"
struct Triple {
    a: u64;
    b: u64;
    c: u64;
}

export(c) fun wave_take(value: Triple) -> u64 {
    return value.c;
}

export(c) fun wave_make(a: u64, b: u64, c: u64) -> Triple {
    return Triple { a: a, b: b, c: c };
}

fun main() -> i32 {
    let value: Triple = wave_make(1, 2, 3);
    if (wave_take(value) != 3) {
        return 1;
    }
    return 0;
}
"#,
    );
    let out = dir.join("out");
    run_wavec([
        OsStr::new("build"),
        source.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("riscv64-unknown-linux-gnu"),
        OsStr::new("--emit=ir"),
        OsStr::new("--out-dir"),
        out.as_os_str(),
    ]);
    let ir = fs::read_to_string(out.join("aggregate.ll")).unwrap();
    assert!(
        ir.contains("define i64 @wave_take(ptr byval(%Triple) align 8"),
        "{}",
        ir
    );
    assert!(
        ir.contains("define void @wave_make(ptr sret(%Triple) align 8"),
        "{}",
        ir
    );
    assert!(ir.contains("@__wave_export_impl_wave_take"), "{}", ir);
    assert!(ir.contains("@__wave_export_impl_wave_make"), "{}", ir);
}

#[test]
fn waveos_boot_smoke_builds_windows_freestanding_coff_object() {
    let dir = temp_case_dir("waveos-boot-smoke-coff");
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test/test108.wave");
    let object = dir.join("waveos_boot_smoke.obj");

    run_wavec([
        OsStr::new("build"),
        source.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("x86_64-pc-windows-gnu"),
        OsStr::new("--freestanding"),
        OsStr::new("--emit=obj"),
        OsStr::new("-o"),
        object.as_os_str(),
    ]);

    let bytes = fs::read(&object).unwrap();
    assert!(
        bytes_contains(&bytes, &[0xB0, 0x45, 0xE6, 0xE9]),
        "COFF object must keep the embedded kernel byte array in .data"
    );
    assert!(
        bytes_contains(&bytes, b"embedded_kernel"),
        "COFF object must keep a relocatable embedded_kernel symbol"
    );
    assert!(
        bytes_contains(&bytes, &[0x41, 0xFF, 0xE3]),
        "jump_to_kernel must lower to an indirect jmp through r11"
    );
    assert!(
        !bytes_contains(&bytes, &[0x49, 0xC7, 0xC3, 0x00, 0x00, 0x20, 0x00]),
        "jump_to_kernel must not hard-code mov r11, 0x200000"
    );
}
