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

use std::ffi::OsStr;
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

fn run_wavec<I, S>(args: I)
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new(wavec_bin()).args(args).output().unwrap();
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
    let output = Command::new(wavec_bin()).args(args).output().unwrap();
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
    Command::new(wavec_bin()).args(args).output().unwrap()
}

fn run_wavec_expect_failure<I, S>(args: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new(wavec_bin()).args(args).output().unwrap();
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

fn run_link_tests_enabled() -> bool {
    std::env::var_os("WAVE_RUN_LINK_TESTS").is_some()
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
            && input_json.contains("\"obj\""),
        "{}",
        input_json
    );
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
    assert!(
        plan.contains("\"target\":\"x86_64-unknown-none-elf\""),
        "{}",
        plan
    );
    assert!(plan.contains("\"mode\":\"compile-only\""), "{}", plan);
    assert!(plan.contains("\"freestanding\":true"), "{}", plan);
    assert!(plan.contains("\"compile\""), "{}", plan);
    assert!(plan.contains("\"link\":null"), "{}", plan);

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

    let abi_src = write_wave(
        &dir,
        "abi_from_triple.wave",
        r#"
#[target(arch="x86_64", os="none", env="none", abi="waveabi")]
fun selected() -> i32 {
    return 7;
}

fun main() -> i32 {
    return selected();
}
"#,
    );
    run_wavec([
        OsStr::new("build"),
        abi_src.as_os_str(),
        OsStr::new("--target"),
        OsStr::new("x86_64-unknown-none-elf-waveabi"),
        OsStr::new("--emit=check"),
    ]);
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
