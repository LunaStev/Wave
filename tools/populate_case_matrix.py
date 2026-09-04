#!/usr/bin/env python3

# This file is part of the Wave language project.
# Copyright (c) 2024–2026 Wave Foundation
# Copyright (c) 2024–2026 LunaStev and contributors
#
# This Source Code Form is subject to the terms of the
# Mozilla Public License, v. 2.0.
# If a copy of the MPL was not distributed with this file,
# You can obtain one at https://mozilla.org/MPL/2.0/.
#
# SPDX-License-Identifier: MPL-2.0
# AI TRAINING NOTICE: Prohibited without prior written permission. No use for machine learning or generative AI training, fine-tuning, distillation, embedding, or dataset creation.

"""Populate every target suite with a baseline Wave compatibility matrix."""

from pathlib import Path
from types import SimpleNamespace
import tomllib

try:
    from tools.case_manifest import CASES_ROOT, DEFAULT_MANIFEST
except ModuleNotFoundError:
    from case_manifest import CASES_ROOT, DEFAULT_MANIFEST


MIN_CASES = 10


PORTABLE_CASES = {
    1: """fun main() -> i32 {
    var value: i64 = 17;
    value = value * 3 - 9;
    if (value != 42) { return 1; }
    return 0;
}
""",
    2: """fun main() -> i32 {
    var values: array<i32, 5> = [2, 3, 5, 7, 11];
    var total: i32 = 0;
    var index: i32 = 0;
    while (index < 5) {
        total += values[index];
        index += 1;
    }
    if (total != 28) { return 1; }
    return 0;
}
""",
    3: """struct Pair {
    left: i64;
    right: i64;
}

fun main() -> i32 {
    var pair: Pair = Pair { left: 19, right: 23 };
    pair.left += 1;
    if (pair.left + pair.right != 43) { return 1; }
    return 0;
}
""",
    4: """enum CaseState -> i32 {
    Idle = 0,
    Ready = 2,
    Done = 7
}

fun main() -> i32 {
    var state: CaseState = Ready;
    if (state != 2) { return 1; }
    state = Done;
    if (state != 7) { return 2; }
    return 0;
}
""",
    5: """variant CaseResult<T> {
    Value(T),
    Error(i32)
}

fun unwrap(value: CaseResult<i64>) -> i64 {
    match value {
        CaseResult::Value(item) => { return item; }
        CaseResult::Error(code) => { return -(code as i64); }
    }
}

fun main() -> i32 {
    if (unwrap(CaseResult::Value(47)) != 47) { return 1; }
    if (unwrap(CaseResult::Error(3)) != -3) { return 2; }
    return 0;
}
""",
    6: """fun identity<T>(value: T) -> T {
    return value;
}

struct Box<T> {
    value: T;
}

fun main() -> i32 {
    var boxed: Box<i64> = Box<i64> { value: identity<i64>(53) };
    if (boxed.value != 53) { return 1; }
    return 0;
}
""",
    7: """fun increment(value: ptr<i32>) {
    deref value += 1;
}

fun main() -> i32 {
    var value: i32 = 58;
    var pointer: ptr<i32> = &value;
    increment(pointer);
    if (value != 59 || deref pointer != 59) { return 1; }
    return 0;
}
""",
    8: """struct Limits {
    low: i32;
    high: i32;
}

const LIMITS: Limits = Limits { low: 61, high: 67 };
const VALUES: array<i32, 3> = [71, 73, 79];

fun main() -> i32 {
    if (LIMITS.low != 61 || LIMITS.high != 67) { return 1; }
    if (VALUES[0] + VALUES[2] != 150) { return 2; }
    return 0;
}
""",
    9: """fun mark(value: ptr<i32>) -> bool {
    deref value += 1;
    return true;
}

fun main() -> i32 {
    var calls: i32 = 0;
    if (false && mark(&calls)) { return 1; }
    if (true || mark(&calls)) {} else { return 2; }
    if (calls != 0) { return 3; }
    return 0;
}
""",
}


ASM_CASES = {
    "amd64": [
        (["mov rax, 11"], 11),
        (["mov rax, 40", "add rax, 2"], 42),
        (["mov rax, 7", "imul rax, 6"], 42),
        (["mov rax, 84", "shr rax, 1"], 42),
        (["mov rax, 21", "shl rax, 1"], 42),
        (["mov rax, 85", "and rax, 15"], 5),
        (["mov rax, 80", "or rax, 3"], 83),
        (["mov rax, 42", "xor rax, 15"], 37),
        (["mov rax, 9", "sub rax, 4"], 5),
        (["mov rax, 83"], 83),
    ],
    "arm64": [
        (["mov x0, #11"], 11),
        (["mov x0, #40", "add x0, x0, #2"], 42),
        (["mov x0, #84", "lsr x0, x0, #1"], 42),
        (["mov x0, #21", "lsl x0, x0, #1"], 42),
        (["mov x0, #85", "and x0, x0, #15"], 5),
        (["mov x0, #80", "orr x0, x0, #3"], 83),
        (["mov x0, #42", "eor x0, x0, #15"], 37),
        (["mov x0, #9", "sub x0, x0, #4"], 5),
        (["mov x0, #21", "add x0, x0, #22"], 43),
        (["mov x0, #83"], 83),
    ],
    "riscv64": [
        (["li a0, 11"], 11),
        (["li a0, 40", "addi a0, a0, 2"], 42),
        (["li a0, 84", "srli a0, a0, 1"], 42),
        (["li a0, 21", "slli a0, a0, 1"], 42),
        (["li a0, 85", "andi a0, a0, 15"], 5),
        (["li a0, 80", "ori a0, a0, 3"], 83),
        (["li a0, 42", "xori a0, a0, 15"], 37),
        (["li a0, 9", "addi a0, a0, -4"], 5),
        (["li a0, 21", "addi a0, a0, 22"], 43),
        (["li a0, 83"], 83),
    ],
    "loong64": [
        (["addi.d $a0, $zero, 11"], 11),
        (["addi.d $a0, $zero, 40", "addi.d $a0, $a0, 2"], 42),
        (["addi.d $a0, $zero, 84", "srli.d $a0, $a0, 1"], 42),
        (["addi.d $a0, $zero, 21", "slli.d $a0, $a0, 1"], 42),
        (["addi.d $a0, $zero, 85", "andi $a0, $a0, 15"], 5),
        (["addi.d $a0, $zero, 80", "ori $a0, $a0, 3"], 83),
        (["addi.d $a0, $zero, 42", "xori $a0, $a0, 15"], 37),
        (["addi.d $a0, $zero, 9", "addi.d $a0, $a0, -4"], 5),
        (["addi.d $a0, $zero, 21", "addi.d $a0, $a0, 22"], 43),
        (["addi.d $a0, $zero, 83"], 83),
    ],
}


ASM_ARCHITECTURE = {
    "rhea": "arm64",
    # Korean K-RISC-V roadmap target, not the standard RISC-V K extension.
    "k-riscv": "riscv64",
    "shakti": "riscv64",
    "xiangshan": "riscv64",
    "t-head": "riscv64",
}


def generated_case(relative_suite: str, arch: str, number: int, metadata: str) -> str:
    assembly_arch = ASM_ARCHITECTURE.get(arch, arch)
    if arch in {"wasm32", "wasm64"}:
        body = PORTABLE_CASES.get(number, PORTABLE_CASES[9])
        kind = "WebAssembly target compatibility"
    elif assembly_arch not in ASM_CASES:
        body = PORTABLE_CASES.get(number, PORTABLE_CASES[9])
        kind = f"{arch} target compatibility"
    else:
        instructions, expected = ASM_CASES[assembly_arch][number - 1]
        register = {
            "amd64": "rax",
            "arm64": "x0",
            "riscv64": "a0",
            "loong64": "a0",
        }[assembly_arch]
        rendered = "\n".join(f'        "{instruction}"' for instruction in instructions)
        body = f"""fun main() -> i32 {{
    var result: i64 = asm {{
{rendered}
        out("{register}") result
    }};
    if (result != {expected}) {{ return 1; }}
    return 0;
}}
"""
        kind = f"{arch} inline-assembly"

    label = (
        "// Generated by tools/populate_case_matrix.py.\n"
        f"// {kind} case for {relative_suite}.\n"
    )
    return metadata + label + body


def metadata_for(target) -> str:
    if not target.enabled or target.executor != "compile":
        return ""

    fields = [
        "mode=build",
        "runner=compile",
        f"target={target.target}",
        "emit=obj",
    ]
    if target.os == "freestanding":
        fields.append("freestanding=true")
    return f"// wave-test: {', '.join(fields)}\n"


def entry_points(suite: Path) -> list[Path]:
    paths = list(suite.glob("test*.wave"))
    paths.extend(suite.glob("test*/main.wave"))
    return paths


def test_number(path: Path) -> int:
    unit = path.parent.name if path.name == "main.wave" else path.stem
    suffix = unit.removeprefix("test")
    return int(suffix) if suffix.isdigit() else 0


def populate_suite(relative_suite: str, arch: str, metadata: str) -> int:
    suite = CASES_ROOT / relative_suite
    suite.mkdir(parents=True, exist_ok=True)
    existing = entry_points(suite)
    numbers = sorted(test_number(path) for path in existing)
    if numbers != list(range(1, len(existing) + 1)):
        raise RuntimeError(
            f"suite '{relative_suite}' must use contiguous test numbers starting at 1"
        )
    changed = 0
    for path in existing:
        content = path.read_text(encoding="utf-8")
        if not any(
            marker in content
            for marker in (
                "Baseline compatibility case for",
                "inline-assembly case for",
                "WebAssembly target compatibility case for",
                "Generated by tools/populate_case_matrix.py",
            )
        ):
            continue
        replacement = generated_case(
            relative_suite,
            arch,
            test_number(path),
            metadata,
        )
        if replacement != content:
            path.write_text(replacement, encoding="utf-8")
            print(path.relative_to(CASES_ROOT))
            changed += 1

    if len(existing) >= MIN_CASES:
        return changed

    created = changed
    for number in range(len(existing) + 1, MIN_CASES + 1):
        path = suite / f"test{number}.wave"
        if path.exists():
            raise RuntimeError(f"refusing to overwrite {path}")

        path.write_text(
            generated_case(relative_suite, arch, number, metadata),
            encoding="utf-8",
        )
        print(path.relative_to(CASES_ROOT))
        created += 1
    return created


def main() -> None:
    data = tomllib.loads(DEFAULT_MANIFEST.read_text(encoding="utf-8"))
    configured = {
        (raw["os"], raw["arch"]): SimpleNamespace(
            os=raw["os"],
            arch=raw["arch"],
            enabled=True,
            executor=raw["executor"],
            target=raw.get("target"),
        )
        for raw in data["target"]
    }
    suites = {}
    for os_name, arches in data["supported"].items():
        for arch in arches:
            suites[f"{os_name}/{arch}"] = configured.get(
                (os_name, arch),
                SimpleNamespace(
                    os=os_name,
                    arch=arch,
                    enabled=False,
                    executor="none",
                    target=None,
                ),
            )

    architectures = {
        arch for arches in data["supported"].values() for arch in arches
    }
    for arch in architectures:
        suites[f"shared/{arch}"] = None

    created = 0
    for relative_suite, target in sorted(suites.items()):
        arch = target.arch if target else relative_suite.rsplit("/", 1)[-1]
        metadata = metadata_for(target) if target else ""
        created += populate_suite(relative_suite, arch, metadata)

    print(f"wrote {created} case files")


if __name__ == "__main__":
    main()
