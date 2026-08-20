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

import subprocess
import time
import argparse
import json
from pathlib import Path
import threading
import socket
import sys
import platform
import shutil
import tempfile
import errno

try:
    from tools.test_contracts import (
        normalize_arch,
        parse_test_metadata as parse_test_metadata_file,
        validate_compiled_artifact,
    )
except ModuleNotFoundError:
    from test_contracts import (
        normalize_arch,
        parse_test_metadata as parse_test_metadata_file,
        validate_compiled_artifact,
    )

ROOT = Path(__file__).resolve().parent.parent
TEST_DIR = ROOT / "tests" / "cases"

TIMEOUT_SEC = 5

GREEN = "\033[92m"
RED = "\033[91m"
YELLOW = "\033[93m"
BLUE = "\033[94m"
CYAN = "\033[96m"
MAGENTA = "\033[95m"
RESET = "\033[0m"

KNOWN_TIMEOUT = {
    # "test22.wave",
}

FAIL_PATTERNS = [
    "WaveError",
    "WaveErrorKind",
    "SyntaxError",
    "error[E",
    "failed to parse",
    "Failed to run",
    "llc failed:",
    "compiler internal error during code generation",
    "thread 'main' panicked",
    "panicked at",
    "LLVM ERROR",
    "Segmentation fault",
    "stack overflow",
]

def resolve_wavec() -> Path:
    candidates = [
        ROOT / "target" / "release" / "wavec.exe",
        ROOT / "target" / "release" / "wavec",
        ROOT / "target" / "debug" / "wavec.exe",
        ROOT / "target" / "debug" / "wavec",
        ROOT / "target" / "x86_64-pc-windows-gnu" / "release" / "wavec.exe",
        ROOT / "target" / "x86_64-pc-windows-gnu" / "debug" / "wavec.exe",
    ]
    for candidate in candidates:
        if candidate.exists():
            return candidate

    print("wavec not found. Run `cargo build --release` or `cargo build` first.")
    sys.exit(1)


WAVEC = resolve_wavec()

results = []

HOST_OS = platform.system().lower()
HOST_ARCH = normalize_arch(platform.machine())
TEST_OUTPUT_DIR = Path(tempfile.mkdtemp(prefix="wave-test-output-"))


def parse_args():
    parser = argparse.ArgumentParser(description="Run Wave end-to-end tests")
    parser.add_argument(
        "--only",
        action="append",
        default=[],
        metavar="NAME",
        help="run only the named test; may be repeated",
    )
    parser.add_argument(
        "--skip",
        action="append",
        default=[],
        metavar="NAME",
        help="skip the named test; may be repeated",
    )
    parser.add_argument(
        "--report-json",
        type=Path,
        metavar="PATH",
        help="write a machine-readable result report",
    )
    return parser.parse_args()


ARGS = parse_args()


def iter_test_entries():
    for path in sorted(TEST_DIR.glob("test*.wave")):
        if (not ARGS.only or path.name in ARGS.only) and path.name not in ARGS.skip:
            yield path.name, path.relative_to(ROOT).as_posix()

    for main_wave in sorted(TEST_DIR.glob("test*/main.wave")):
        name = f"{main_wave.parent.name} (dir)"
        if (not ARGS.only or name in ARGS.only) and name not in ARGS.skip:
            yield name, main_wave.relative_to(ROOT).as_posix()


def parse_test_metadata(rel_path: str):
    return parse_test_metadata_file(ROOT / rel_path, rel_path)


def skip_reason_for_metadata(name: str, rel_path: str):
    meta = parse_test_metadata(rel_path)
    host_os = meta.host_os
    host_arch = meta.host_arch

    if host_os and host_os != HOST_OS:
        return f"{name} requires host OS {host_os}, current host is {HOST_OS}"

    if host_arch and host_arch != HOST_ARCH:
        return f"{name} requires host arch {host_arch}, current host is {HOST_ARCH}"

    return None


def command_for_test(name: str, rel_path: str):
    meta = parse_test_metadata(rel_path)
    mode = meta.mode

    if mode == "run":
        return [str(WAVEC), "run", rel_path]

    if mode == "check":
        return [str(WAVEC), "check", rel_path]

    if mode == "build":
        output_dir = TEST_OUTPUT_DIR / name.replace(" ", "-")
        output_dir.mkdir(parents=True, exist_ok=True)

        cmd = [
            str(WAVEC),
            "build",
            rel_path,
            f"--emit={meta.emit}",
            "--out-dir",
            str(output_dir),
        ]
        if meta.target:
            cmd.extend(["--target", meta.target])
        if meta.freestanding:
            cmd.append("--freestanding")
        return cmd

    raise ValueError(f"unsupported wave-test mode '{mode}' in {rel_path}")


def send_udp_test_input():
    try:
        with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as sock:
            # `wavec run` compiles before starting the receiver. Repeat the
            # datagram during that startup window instead of racing a single
            # send against compilation on slower CI hosts.
            for _ in range(50):
                time.sleep(0.1)
                sock.sendto(b"hello from python\n", ("127.0.0.1", 8080))
    except OSError:
        # Some CI/sandbox environments block local sockets.
        pass

def run_test56_server(cmd):
    print(f"{BLUE}RUN test56.wave (server test){RESET}")

    proc = subprocess.Popen(
        cmd,
        cwd=str(ROOT),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )

    try:
        time.sleep(1.0)  # server boot wait

        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.settimeout(2)
        s.connect(("127.0.0.1", 8080))
        s.sendall(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")

        data = s.recv(4096)
        s.close()

        if b"Welcome to the Wave HTTP Server!" in data:
            print(f"{GREEN}→ PASS (server responded){RESET}\n")
            return 1, None
        else:
            print(f"{RED}→ FAIL (unexpected response){RESET}")
            print(data)
            return 0, None

    except OSError as e:
        if e.errno in {errno.EPERM, errno.EACCES}:
            print(f"{CYAN}→ SKIP (local TCP sockets blocked by environment){RESET}\n")
            return 2, "local TCP sockets blocked by environment"

        print(f"{RED}→ FAIL (server not responding){RESET}")
        print(e)
        return 0, None

    except Exception as e:
        print(f"{RED}→ FAIL (server not responding){RESET}")
        print(e)
        return 0, None

    finally:
        proc.terminate()
        try:
            proc.wait(timeout=1)
        except subprocess.TimeoutExpired:
            proc.kill()


def looks_like_fail(stderr: str) -> bool:
    if not stderr:
        return False
    s = stderr.strip()
    if not s:
        return False
    s_low = s.lower()
    for p in FAIL_PATTERNS:
        if p.lower() in s_low:
            return True
    return False

# Return Type:
# 1 = PASS (exit 0)
# 3 = PASS (explicit expected nonzero exit)
# 0 = FAIL
# 2 = SKIP
# -1 = TIMEOUT
def run_and_classify(name, rel_path, cmd):
    print(f"{BLUE}RUN {name}{RESET}")

    skip_reason = skip_reason_for_metadata(name, rel_path)
    if skip_reason is not None:
        print(f"{CYAN}→ SKIP ({skip_reason}){RESET}\n")
        return 2, skip_reason

    metadata = parse_test_metadata(rel_path)
    expected_exit = metadata.expected_exit

    stdin_data = None
    if name == "test22.wave":
        stdin_data = "3\n"

    if name == "test74.wave":
        stdin_data = "10\n"

    if name == "test56.wave":
        return run_test56_server(cmd)

    try:
        if metadata.udp_input:
            threading.Thread(
                target=send_udp_test_input,
                daemon=True
            ).start()

        result = subprocess.run(
            cmd,
            cwd=str(ROOT),
            input=stdin_data,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=TIMEOUT_SEC
        )

        if looks_like_fail(result.stderr):
            print(f"{RED}→ FAIL (exit={result.returncode}){RESET}")
            if result.stdout.strip():
                print(f"{BLUE}--- STDOUT ---{RESET}")
                print(result.stdout.rstrip())
            if result.stderr.strip():
                print(f"{YELLOW}--- STDERR ---{RESET}")
                print(result.stderr.rstrip())
            print()
            return 0, None

        if result.returncode == expected_exit:
            if expected_exit != 0:
                print(f"{MAGENTA}→ PASS (expected exit={expected_exit}){RESET}\n")
                return 3, None
            artifact_error = validate_compiled_artifact(
                name,
                ROOT / rel_path,
                TEST_OUTPUT_DIR,
                metadata,
            )
            if artifact_error:
                print(f"{RED}→ FAIL (artifact contract){RESET}")
                print(artifact_error)
                print()
                return 0, None
            print(f"{GREEN}→ PASS{RESET}\n")
            return 1, None

        print(
            f"{RED}→ FAIL (exit={result.returncode}, expected={expected_exit}){RESET}"
        )
        if result.stdout.strip():
            print(f"{BLUE}--- STDOUT ---{RESET}")
            print(result.stdout.rstrip())
        if result.stderr.strip():
            print(f"{YELLOW}--- STDERR ---{RESET}")
            print(result.stderr.rstrip())
        print()
        return 0, None

    except subprocess.TimeoutExpired:
        if name in KNOWN_TIMEOUT:
            print(f"{CYAN}→ SKIP (expected blocking / unimplemented){RESET}\n")
            return 2, "expected blocking / unimplemented"
        else:
            print(f"{YELLOW}→ TIMEOUT ({TIMEOUT_SEC}s){RESET}\n")
            return -1, f"timed out after {TIMEOUT_SEC}s"

entries = list(iter_test_entries())
selected_names = {name for name, _ in entries}
missing_names = sorted(set(ARGS.only) - selected_names)

if missing_names:
    shutil.rmtree(TEST_OUTPUT_DIR, ignore_errors=True)
    print(f"unknown test name(s): {', '.join(missing_names)}", file=sys.stderr)
    sys.exit(2)

if not entries:
    shutil.rmtree(TEST_OUTPUT_DIR, ignore_errors=True)
    print("no tests selected", file=sys.stderr)
    sys.exit(2)

try:
    for name, rel_path in entries:
        result, detail = run_and_classify(
            name,
            rel_path,
            command_for_test(name, rel_path)
        )
        results.append((name, result, detail))

        time.sleep(0.3)
except KeyboardInterrupt:
    print(f"\n{YELLOW}Interrupted by user.{RESET}")
    sys.exit(130)
except ValueError as error:
    print(f"{RED}invalid wave-test metadata: {error}{RESET}", file=sys.stderr)
    sys.exit(2)
finally:
    shutil.rmtree(TEST_OUTPUT_DIR, ignore_errors=True)

pass_zero = [name for name, result, _ in results if result == 1]
pass_nonzero = [name for name, result, _ in results if result == 3]
fail_tests = [name for name, result, _ in results if result == 0]
timeout_tests = [name for name, result, _ in results if result == -1]
skip_tests = [name for name, result, _ in results if result == 2]

print("\n=========================")
print("🎉 FINAL TEST RESULT")
print("=========================\n")

print(f"{GREEN}PASS (exit=0) ({len(pass_zero)}){RESET}")
for name in pass_zero:
    print(f"  - {name}")

print(f"\n{MAGENTA}PASS (expected non-zero exit) ({len(pass_nonzero)}){RESET}")
for name in pass_nonzero:
    print(f"  - {name}")

print(f"\n{CYAN}SKIP ({len(skip_tests)}){RESET}")
for name in skip_tests:
    print(f"  - {name}")

print(f"\n{RED}FAIL ({len(fail_tests)}){RESET}")
for name in fail_tests:
    print(f"  - {name}")

print(f"\n{YELLOW}TIMEOUT ({len(timeout_tests)}){RESET}")
for name in timeout_tests:
    print(f"  - {name}")

print("\n=========================")
print(f"{GREEN}PASS(0): {len(pass_zero)}{RESET}")
print(f"{MAGENTA}PASS(expected !0): {len(pass_nonzero)}{RESET}")
print(f"{CYAN}SKIP: {len(skip_tests)}{RESET}")
print(f"{RED}FAIL: {len(fail_tests)}{RESET}")
print(f"{YELLOW}TIMEOUT: {len(timeout_tests)}{RESET}")
print("=========================\n")

report_failed = False
if ARGS.report_json is not None:
    statuses = {-1: "timeout", 0: "fail", 1: "pass", 2: "skip", 3: "pass"}
    report = {
        "compiler": str(WAVEC),
        "host": {"os": HOST_OS, "arch": HOST_ARCH},
        "summary": {
            "pass": len(pass_zero) + len(pass_nonzero),
            "skip": len(skip_tests),
            "fail": len(fail_tests),
            "timeout": len(timeout_tests),
        },
        "tests": [
            {
                "name": name,
                "status": statuses[result],
                **({"reason": detail} if detail else {}),
            }
            for name, result, detail in results
        ],
    }
    try:
        ARGS.report_json.parent.mkdir(parents=True, exist_ok=True)
        ARGS.report_json.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        print(f"Wrote test report to {ARGS.report_json}")
    except OSError as error:
        print(f"failed to write test report: {error}", file=sys.stderr)
        report_failed = True

if fail_tests or timeout_tests or report_failed:
    sys.exit(1)
