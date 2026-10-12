// SPDX-License-Identifier: MPL-2.0
//! Execute the supported integer subset with the upstream bounded interpreter.
//! Float arithmetic and calls are explicit unsupported outcomes, not successes.
use super::{mask, module};
use whale_ir::*;

fn options() -> InterpreterOptions {
    InterpreterOptions {
        max_steps: 10_000,
        ir_limits: IrLimits { max_nodes: 20_000, ..IrLimits::default() },
        memory_limits: MemoryLimits {
            max_bytes: 4096,
            max_allocations: 128,
            max_pointer_fragments: 128,
            max_work: 100_000,
        },
    }
}

fn run(module: &Module, arguments: &[ConstValue]) -> Result<InterpreterResult, InterpreterError> {
    interpret_with_options(module, module.functions[0].id, arguments, options())
}

fn integer(raw: u128, bits: u32, signed: bool) -> ConstValue {
    let raw = raw & mask(bits);
    if signed {
        let shift = 128 - bits;
        ConstValue::I(((raw << shift) as i128) >> shift)
    } else {
        ConstValue::U(raw)
    }
}

#[test]
fn wrapping_arithmetic_executes_at_every_supported_integer_width() {
    for bits in [8, 16, 32, 64, 128] {
        for signed in [false, true] {
            let ty = format!("{}{bits}", if signed { "i" } else { "u" });
            let ir = module(&format!(
                "fun probe(a: {ty}, b: {ty}) -> {ty} {{ return (a + b) * b - b; }}"
            ));
            for (a, b) in
                [(0, 1), (mask(bits), 2), (1 << (bits - 1), 3), (mask(bits) - 1, mask(bits))]
            {
                let expected = a.wrapping_add(b).wrapping_mul(b).wrapping_sub(b);
                let result =
                    run(&ir, &[integer(a, bits, signed), integer(b, bits, signed)]).unwrap();
                assert_eq!(result.value, Some(integer(expected, bits, signed)), "{ty}: {a}, {b}");
                assert!(result.steps > 0 && result.steps < options().max_steps);
            }
        }
    }
}

#[test]
fn ordered_truncation_extension_and_equal_width_signedness_are_observable() {
    let ir = module("fun probe(n: i32) -> i32 { return (n as u8) as i32; }");
    for n in [-257, -1, 0, 255, 256, 257, i32::MAX] {
        assert_eq!(
            run(&ir, &[ConstValue::I(n as i128)]).unwrap().value,
            Some(ConstValue::I((n as u8) as i128))
        );
    }
    let ir = module("fun probe(n: u32) -> i32 { return n as i32; }");
    for n in [0, 1, 0x80000000, u32::MAX] {
        assert_eq!(
            run(&ir, &[ConstValue::U(n as u128)]).unwrap().value,
            Some(ConstValue::I((n as i32) as i128))
        );
    }
    let ir = module("fun probe(n: i8) -> u64 { return n as u64; }");
    assert_eq!(
        run(&ir, &[ConstValue::I(-1)]).unwrap().value,
        Some(ConstValue::U(u64::MAX as u128))
    );
}

#[test]
fn shifts_execute_or_trap_on_original_counts_before_narrowing() {
    for bits in [8, 16, 32, 64, 128] {
        for signed in [false, true] {
            let ty = format!("{}{bits}", if signed { "i" } else { "u" });
            let raw = mask(bits) - 1;
            for count_bits in [8, 16, 32, 64, 128] {
                for count_signed in [false, true] {
                    let rhs = format!("{}{count_bits}", if count_signed { "i" } else { "u" });
                    for op in ["<<", ">>"] {
                        let ir = module(&format!(
                            "fun probe(x: {ty}, n: {rhs}) -> {ty} {{ return x {op} n; }}"
                        ));
                        for count in
                            [0, 1, bits as u128 - 1, bits as u128, bits as u128 + 1, 256, u128::MAX]
                        {
                            let count = count & mask(count_bits);
                            let negative = count_signed && count & (1 << (count_bits - 1)) != 0;
                            let result = run(
                                &ir,
                                &[
                                    integer(raw, bits, signed),
                                    integer(count, count_bits, count_signed),
                                ],
                            );
                            if negative || count >= bits as u128 {
                                assert!(
                                    matches!(result, Err(InterpreterError::Trap(InterpreterTrap {
                                    reason: TrapReason::Explicit(ref reason), ..
                                })) if reason == "shift count out of range"),
                                    "{ty} {op} {rhs}({count}): {result:?}"
                                );
                            } else {
                                let shift = count as u32;
                                let expected = if op == "<<" {
                                    raw << shift
                                } else if signed {
                                    let ConstValue::I(n) = integer(raw, bits, true) else {
                                        unreachable!()
                                    };
                                    (n >> shift) as u128
                                } else {
                                    raw >> shift
                                };
                                assert_eq!(
                                    result.unwrap().value,
                                    Some(integer(expected, bits, signed)),
                                    "{ty} {op} {rhs}({count})"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn logical_conversions_short_circuit_traps_and_loop_storage_stays_initialized() {
    let ir =
        module("fun probe(flag: i32, n: u64) -> bool { return flag && (((1 as u8) << n) != 0); }");
    assert_eq!(
        run(&ir, &[ConstValue::I(0), ConstValue::U(8)]).unwrap().value,
        Some(ConstValue::Bool(false))
    );
    assert_eq!(
        run(&ir, &[ConstValue::I(-1), ConstValue::U(7)]).unwrap().value,
        Some(ConstValue::Bool(true))
    );
    assert!(matches!(
        run(&ir, &[ConstValue::I(1), ConstValue::U(8)]),
        Err(InterpreterError::Trap(_))
    ));
    let ir =
        module("fun probe(flag: i32, n: u64) -> bool { return flag || (((1 as u8) << n) != 0); }");
    assert_eq!(
        run(&ir, &[ConstValue::I(3), ConstValue::U(8)]).unwrap().value,
        Some(ConstValue::Bool(true))
    );
    let ir = module(
        r#"
fun probe(limit: i32) -> i32 {
    var n: i32 = 0;
    var sum: i32 = 0;
    while (n < limit) {
        n = n + 1;
        if (n == 2) { continue; }
        if (n == 5) { break; }
        var sum: i32 = sum + 10;
        if (sum < 10) { return -1; }
    }
    return sum + n;
}
"#,
    );
    for (limit, expected) in [(0, 0), (1, 1), (3, 3), (6, 5)] {
        assert_eq!(run(&ir, &[ConstValue::I(limit)]).unwrap().value, Some(ConstValue::I(expected)));
    }
}

#[test]
fn numeric_condition_truthiness_executes_without_reinferring_hir_types() {
    for ty in ["i8", "i32", "i64", "i128", "u8", "u64", "u128", "byte"] {
        let signed = ty.starts_with('i');
        let ir = module(&format!(
            "fun probe(n: {ty}) -> i32 {{ if ((n)) {{ return 7; }} else if (2) {{ return 3; }} else {{ return 9; }} }}"
        ));
        for n in [0, 1, 2] {
            let arg = if signed { ConstValue::I(n) } else { ConstValue::U(n as u128) };
            assert_eq!(
                run(&ir, &[arg]).unwrap().value,
                Some(ConstValue::I(if n == 0 { 3 } else { 7 }))
            );
        }
    }
    let ir = module("fun probe(n: i32) -> i32 { var x: i32 = n; while (x) { x = 0; } return x; }");
    assert_eq!(run(&ir, &[ConstValue::I(-3)]).unwrap().value, Some(ConstValue::I(0)));
}

#[test]
fn budgets_unsupported_operations_and_traps_have_distinct_outcomes() {
    let ir = module(
        "fun probe(n: i32) -> i32 { var x: i32 = n; while (x >= 0) { x = x + 1; } return x; }",
    );
    let limited = InterpreterOptions { max_steps: 40, ..options() };
    assert!(matches!(
        interpret_with_options(&ir, ir.functions[0].id, &[ConstValue::I(0)], limited),
        Err(InterpreterError::StepLimit { limit: 40, .. })
    ));
    let ir = module("fun probe(n: i64) -> i64 { var x: i64 = n; return x; }");
    let limited = InterpreterOptions {
        memory_limits: MemoryLimits { max_bytes: 1, ..options().memory_limits },
        ..options()
    };
    assert!(matches!(
        interpret_with_options(&ir, ir.functions[0].id, &[ConstValue::I(0)], limited),
        Err(InterpreterError::MemoryLimit { .. })
    ));
    let ir = module("fun probe(n: i32) -> i32 { return helper(n); } fun helper(n: i32) -> i32 { return n + 1; }");
    assert!(matches!(
        run(&ir, &[ConstValue::I(0)]),
        Err(InterpreterError::UnsupportedInstruction { .. })
    ));
    let ir = module("fun probe(n: i32) -> i32 { var f: f64 = n as f64; return f as i32; }");
    assert!(matches!(
        run(&ir, &[ConstValue::I(0)]),
        Err(InterpreterError::UnsupportedInstruction { .. })
    ));
}
