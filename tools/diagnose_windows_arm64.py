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

"""Reduce native Windows ARM64 compiler crashes and collect debugger evidence."""

import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys

try:
    from tools.process_tree import run_process, timeout_output
except ModuleNotFoundError:
    from process_tree import run_process, timeout_output


def probe(label, command, directory, environment, timeout=30):
    command = [str(argument) for argument in command]
    lines = [f"[{label}] {subprocess.list2cmdline(command)}"]
    try:
        result = run_process(command, capture_output=True, text=True,
                             errors="replace", env=environment, timeout=timeout)
        code = result.returncode
        lines.extend([result.stdout, result.stderr,
                      f"exit: {code} (0x{code & 0xffffffff:08X})"])
    except subprocess.TimeoutExpired as error:
        code = 1
        lines.extend([timeout_output(error), f"timed out after {timeout}s"])
    except OSError as error:
        code = 1
        lines.append(str(error))
    text = "\n".join(line.rstrip() for line in lines if line) + "\n"
    (directory / f"{label}.log").write_text(text, encoding="utf-8")
    print(text, flush=True)
    return code


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--wavec", type=Path, required=True)
    parser.add_argument("--llvm-bin", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    options = parser.parse_args()
    if os.name != "nt":
        parser.error("this probe must run on native Windows ARM64")
    image = options.wavec.read_bytes()
    pe = int.from_bytes(image[0x3c:0x40], "little")
    if image[:2] != b"MZ" or image[pe:pe+4] != b"PE\0\0" or image[pe+4:pe+6] != b"\x64\xaa":
        parser.error("wavec must be a native ARM64 PE image")
    directory = options.output.resolve()
    directory.mkdir(parents=True, exist_ok=True)
    environment = dict(os.environ, WAVE_CODEGEN_TRACE="1")
    root = Path(__file__).resolve().parent.parent
    probe_home = directory / "home"
    shutil.copytree(root / "std", probe_home / ".wave/lib/wave/std", dirs_exist_ok=True)
    environment.update(HOME=str(probe_home), USERPROFILE=str(probe_home))
    compiler = options.wavec.resolve()
    debugger = options.llvm_bin.resolve() / "lldb.exe"
    for label, command in [
        ("compiler-version", [compiler, "-V"]),
        ("default-target", [compiler, "print", "default-target"]),
        ("llvm-version", [options.llvm_bin / "llvm-config.exe", "--version"]),
        ("debugger-version", [debugger, "--version"]),
    ]:
        probe(label, command, directory, environment)
    minimal = directory / "minimal.wave"
    minimal.write_text("fun main() -> i32 {\n    return 0;\n}\n", encoding="utf-8")
    fixture = root / "tests/cases/windows/arm64/test1.wave"
    failed = False
    for source in [minimal, fixture]:
        if probe(f"{source.stem}-check", [compiler, "check", source], directory, environment):
            failed = True
            continue
        for abi in ["gnu", "msvc"]:
            target = f"aarch64-pc-windows-{abi}"
            for emit in ["ir", "obj"]:
                label = f"{source.stem}-{abi}-{emit}"
                command = [compiler, "build", source, f"--target={target}",
                           f"--emit={emit}", "--out-dir", directory / label]
                if probe(label, command, directory, environment):
                    failed = True
                    probe(f"{label}-debugger", [
                        debugger, "--batch", "--no-lldbinit", "-o", "run",
                        "-k", "thread backtrace all", "-k", "register read",
                        "-k", "image list", "--", *command,
                    ], directory, environment, timeout=90)
                    break
    return int(failed)


if __name__ == "__main__":
    sys.exit(main())
