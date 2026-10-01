# SPDX-License-Identifier: MPL-2.0
"""Frontend, backend, ABI and runtime stages selected by the shared target plan."""

import json
from pathlib import Path
import re
import struct
import sys

from tools.ci.common import ROOT, main
from tools.case_manifest import load_case_manifest
from tools.validation_reports import write_report


def compiler(r):
    return (
        ROOT / "target" / r.target.triple / "release/wavec.exe"
        if r.target.host_os == "windows"
        else ROOT / "target/release/wavec"
    )


def require(pattern, text):
    if not re.search(pattern, text, re.I):
        raise ValueError(f"missing expected output {pattern!r}: {text[:2000]}")


def crt_check(r, root, architecture):
    abis = {
        "riscv64": [
            ("lp64", "0x1", "RVC"),
            ("lp64f", "0x3", "single-float ABI"),
            ("lp64d", "0x5", "double-float ABI"),
        ],
        "loongarch64": [
            ("lp64s", "0x41", ""),
            ("lp64f", "0x42", ""),
            ("lp64d", "0x43", ""),
        ],
    }[architecture]
    machine = "RISC-V" if architecture == "riscv64" else "LoongArch"
    for abi, flag, desc in abis:
        for name in ("crt1.o", "Scrt1.o", "rcrt1.o", "crti.o", "crtn.o"):
            path = root / f"{architecture}-unknown-linux-gnu" / abi / name
            if not path.is_file():
                raise FileNotFoundError(path)
            text = r.run(["llvm-readelf", "-h", path])
            require(r"Machine:\s+" + machine, text)
            require(r"Flags:\s+" + flag, text)
            if desc:
                require(desc, text)


def crt_root():
    candidates = list((ROOT / "target/release/build").glob("*/out/crt"))
    if not candidates:
        raise FileNotFoundError("no compiler CRT output")
    return max(candidates, key=lambda p: p.stat().st_mtime_ns)


def crt_matrix(r, _):
    root = crt_root()
    for target in ("x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"):
        for name in ("crt1.o", "Scrt1.o", "rcrt1.o", "crti.o", "crtn.o"):
            if not (root / target / name).is_file():
                raise FileNotFoundError(root / target / name)
    crt_check(r, root, "riscv64")


def loongarch_crt(r, _):
    crt_check(r, crt_root(), "loongarch64")
    r.run(
        [
            "cargo",
            "test",
            "--test",
            "codegen_regressions",
            "loongarch64",
            "--",
            "--nocapture",
        ]
    )


def sysroot(r):
    value = Path(
        r.run([compiler(r), "print", "sysroot", "--target", r.target.triple]).strip()
    )
    if not value.is_dir():
        raise FileNotFoundError(f"compiler sysroot does not exist: {value}")
    if r.target.id == "linux-loong64" and str(value) != r.env.get(
        "WAVE_LOONGARCH64_SYSROOT"
    ):
        raise ValueError("LoongArch sysroot differs from pinned toolchain")
    return value


def smoke(r, source, expected):
    root = sysroot(r)
    directory = r.temp / "smoke"
    directory.mkdir(exist_ok=True)
    source = ROOT / source
    r.run(
        [
            compiler(r),
            "build",
            source,
            "--std-root",
            ROOT / "std",
            "--target",
            r.target.triple,
            "--sysroot",
            root,
            "--out-dir",
            directory,
        ]
    )
    binary = directory / source.stem
    loong = r.target.id == "linux-loong64"
    inspector = (
        "loongarch64-unknown-linux-gnu-readelf"
        if loong
        else "riscv64-linux-gnu-readelf"
    )
    header = r.run([inspector, "-h", binary])
    segments = r.run([inspector, "-l", binary])
    require(r"Machine:\s+" + ("LoongArch" if loong else "RISC-V"), header)
    require(
        (
            r"Flags:\s+0x43.*DOUBLE-FLOAT.*OBJ-v1"
            if loong
            else r"Flags:\s+0x5.*double-float ABI"
        ),
        header,
    )
    require(
        re.escape(
            "/lib64/ld-linux-loongarch-lp64d.so.1"
            if loong
            else "/lib/ld-linux-riscv64-lp64d.so.1"
        ),
        segments,
    )
    actual = r.run(
        ["qemu-loongarch64" if loong else "qemu-riscv64", "-L", root, binary],
        timeout=30,
    ).rstrip("\r\n")
    if actual != expected:
        raise ValueError(f"smoke output {actual!r} != {expected!r}")


def qemu_cases(r, _):
    target = load_case_manifest().target(r.target.id)
    if not target.smoke_case or target.smoke_stdout is None:
        raise ValueError("missing manifest smoke contract")
    smoke(r, "tests/cases/" + target.smoke_case, target.smoke_stdout)


def loongarch_smoke(r, _):
    qemu_cases(r, {})


def loongarch_release_smoke(r, _):
    smoke(r, "tests/cases/shared/test2.wave", "Hello World")


def riscv_release_smoke(r, _):
    smoke(r, "tests/cases/shared/test2.wave", "Hello World")


def selected_runtime(r, _):
    # Selection remains owned by case_manifest, including known host-import failures.
    sources = r.run(
        ["@python", "tools/case_manifest.py", "runtime-sources", r.target.id]
    ).splitlines()
    if not sources:
        raise ValueError("runtime case selection is empty")
    args = [
        "@python",
        "tools/run_runtime_cases.py",
        "--wavec",
        compiler(r),
        "--target-id",
        r.target.id,
        "--report-json",
        f"wave-cases-{r.target.id}-runtime.json",
        "--sources",
        *sources,
    ]
    if r.target.executor == "qemu":
        args += [
            "--qemu",
            "qemu-loongarch64" if r.target.id == "linux-loong64" else "qemu-riscv64",
            "--sysroot",
            sysroot(r),
        ]
    r.run(args, timeout=18000)


def loongarch_std(r, _):
    for name in (
        "memory",
        "time",
        "environment",
        "filesystem",
        "io",
        "buffers",
        "datetime_roundtrip",
        "process",
    ):
        r.run(
            [
                compiler(r),
                "run",
                ROOT / f"examples/std/{name}.wave",
                "--std-root",
                ROOT / "std",
                "--target",
                r.target.triple,
            ],
            cwd=r.temp,
            timeout=30,
        )


def riscv_net(r, _):
    root = sysroot(r)
    for name in (
        "net_tcp",
        "net_udp",
        "net_ipv6",
        "net_dns",
        "net_interfaces",
        "net_unix",
        "net_vectored",
        "net_event",
    ):
        directory = r.temp / name
        directory.mkdir(exist_ok=True)
        r.run(
            [
                compiler(r),
                "build",
                f"examples/std/{name}.wave",
                "--std-root",
                ROOT / "std",
                "--target",
                r.target.triple,
                "--sysroot",
                root,
                "--out-dir",
                directory,
            ]
        )
        r.run(["qemu-riscv64", "-L", root, directory / name], timeout=30)


def wasm_smoke(r, _):
    directory = r.temp / "wasm"
    directory.mkdir(exist_ok=True)
    for name, target, host in [
        ("wasm_module", "wasm32-unknown-unknown", "tools/run_wasm_smoke.mjs"),
        ("wasm_wasi", "wasm32-wasip1", "tools/run_wasi_smoke.mjs"),
    ]:
        r.run(
            [
                compiler(r),
                "build",
                f"examples/{name}.wave",
                "--std-root",
                ROOT / "std",
                "--target",
                target,
                "--out-dir",
                directory,
            ]
        )
        require(
            "Format: WASM",
            r.run(["llvm-readobj", "--file-headers", directory / (name + ".wasm")]),
        )
        r.run(
            [
                "node",
                "--no-warnings",
                host,
                directory / (name + ".wasm"),
                *([ROOT] if "wasi" in name else []),
            ]
        )
        r.run(
            [
                compiler(r),
                "run",
                f'examples/{"wasm_run" if name=="wasm_module" else name}.wave',
                "--std-root",
                ROOT / "std",
                "--target",
                target,
            ]
        )


def windows_default(r, _):
    from tools.windows_package import pe_machine, MACHINES

    if pe_machine(compiler(r)) != MACHINES[r.target.triple]:
        raise ValueError("compiler PE machine mismatch")
    r.run([compiler(r), "-V"])
    if r.run([compiler(r), "print", "default-target"]).strip() != r.target.triple:
        raise ValueError("compiler default target is not native MSVC")
    windows_object(r, {"explicit": False})


def windows_object(r, operation):
    output = r.temp / (
        "explicit-object" if operation.get("explicit") else "default-object"
    )
    args = [
        compiler(r),
        "build",
        "tests/cases/windows/arm64/test1.wave",
        "--emit=obj",
        "--out-dir",
        output,
    ]
    if operation.get("explicit"):
        args += ["--target", r.target.triple]
    r.run(args)
    data = (output / "test1.o").read_bytes()
    if len(data) < 2 or struct.unpack_from("<H", data)[0] != 0xAA64:
        raise ValueError("expected ARM64 COFF object")


def matrix(r, _):
    r.run(
        [
            "@python",
            "-m",
            "py_compile",
            "tools/case_manifest.py",
            "tools/populate_case_matrix.py",
            "tools/check_freebsd_sys.py",
        ]
    )
    r.run(["@python", "tools/case_manifest.py", "validate"])
    output = r.run(["@python", "tools/case_manifest.py", "github-output"])
    with Path(r.env["GITHUB_OUTPUT"]).open("a", encoding="utf-8") as stream:
        stream.write(output)


def freebsd_report(r, _):
    output = Path(r.env["RUNNER_TEMP"]) / "freebsd-results/job.json"
    write_report(
        output,
        {
            "schema_version": 1,
            "target": r.target.triple,
            "steps": {
                row["id"]: {
                    "outcome": r.context.get(
                        "steps." + row["id"] + ".outcome", "skipped"
                    )
                }
                for row in r.data["stages"]
                if row is not r.current
            },
        },
    )


OPERATIONS = {
    name: globals()[name]
    for name in (
        "crt_matrix",
        "loongarch_crt",
        "qemu_cases",
        "loongarch_smoke",
        "loongarch_release_smoke",
        "riscv_release_smoke",
        "selected_runtime",
        "loongarch_std",
        "riscv_net",
        "wasm_smoke",
        "windows_default",
        "windows_object",
        "matrix",
        "freebsd_report",
    )
}

if __name__ == "__main__":
    sys.exit(main("test"))
