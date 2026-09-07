#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Build libc-independent sys cases and run them in a supplied FreeBSD 14.3 VM.

Requires clang/lld, qemu-img, genisoimage and the selected qemu-system binary.
The input qcow2 image is read through a fresh writable overlay. No downloads,
network device, host filesystem sharing or third-party Python modules are used.
"""

import argparse
import os
from pathlib import Path
import re
import select
import shutil
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
ARCHES = {"amd64": "x86_64", "arm64": "aarch64", "riscv64": "riscv64"}


def run(*args, **kwargs):
    subprocess.run([str(arg) for arg in args], check=True, **kwargs)


class Console:
    def __init__(self, process, log):
        self.process = process
        self.log = log
        self.buffer = ""

    def expect(self, pattern, timeout=240):
        deadline = time.monotonic() + timeout
        while True:
            match = re.search(pattern, self.buffer)
            if match:
                self.buffer = self.buffer[match.end():]
                return match
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError(f"Guest did not reach {pattern!r}; see {self.log.name}")
            ready, _, _ = select.select([self.process.stdout], [], [], min(remaining, 1))
            if ready:
                chunk = os.read(self.process.stdout.fileno(), 65536)
                if not chunk:
                    raise RuntimeError(f"Guest exited before {pattern!r}; see {self.log.name}")
                self.log.write(chunk)
                self.log.flush()
                self.buffer += chunk.decode("utf-8", errors="replace")

    def send(self, command, paced=False):
        for part in command if paced else [command]:
            self.process.stdin.write(part.encode())
            self.process.stdin.flush()
            if paced:
                # The BIOS loader's serial input buffer is much smaller than sh's.
                time.sleep(0.3)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--arch", choices=ARCHES, required=True)
    parser.add_argument("--image", type=Path, required=True, help="Verified official FreeBSD 14.3 UFS qcow2 image")
    parser.add_argument("--firmware", type=Path, help="AArch64 QEMU_EFI.fd")
    parser.add_argument("--kernel", type=Path, help="FreeBSD RISC-V ELF kernel from the matching release")
    parser.add_argument("--compiler", type=Path, default=ROOT / "target/debug/wavec")
    parser.add_argument("--clang", default="clang-21")
    parser.add_argument("--linker", default="ld.lld")
    parser.add_argument("--out-dir", type=Path, default=ROOT / ".tmp/freebsd-sys")
    args = parser.parse_args()
    for path in [args.image, args.compiler]:
        if not path.is_file():
            parser.error(f"File does not exist: {path}")
    if args.arch == "arm64" and (args.firmware is None or not args.firmware.is_file()):
        parser.error("--firmware must name the matching UEFI firmware")
    if args.arch == "riscv64" and (args.kernel is None or not args.kernel.is_file()):
        parser.error("RISC-V requires --kernel from the matching FreeBSD release")
    args.out_dir.mkdir(parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix=f"{args.arch}-", dir=args.out_dir.resolve()))
    print(f"Artifacts and guest log: {work}", flush=True)
    home = work / "home"
    std = home / ".wave/lib/wave/std"
    shutil.copytree(ROOT / "std", std)
    env = dict(os.environ, HOME=str(home))
    iso_root = work / "cases"
    iso_root.mkdir()
    triple = f"{ARCHES[args.arch]}-unknown-freebsd"
    runtime = work / "start.o"
    run(args.clang, f"--target={triple}", "-O2", "-fno-builtin", "-ffreestanding",
        "-fno-stack-protector", "-c", ROOT / "tests/fixtures/freebsd_case_runtime/start.c", "-o", runtime)
    cases = sorted((ROOT / f"tests/cases/freebsd/{args.arch}").glob("test*.wave"),
        key=lambda path: int(path.stem.removeprefix("test")))
    for source in cases:
        for opt in ("O0", "O2"):
            objects = work / f"{source.stem}-{opt}"
            run(args.compiler.resolve(), "build", source,
                "--target", triple, "--emit=obj", f"-{opt}", "--out-dir", objects, env=env)
            run(args.linker, "-static", "-e", "_start", runtime, objects / f"{source.stem}.o",
                "-o", iso_root / f"{source.stem}-{opt}")
    (iso_root / "run.sh").write_text(
        "for name in " + " ".join(source.stem for source in cases) + "; do\n  for opt in O0 O2; do\n"
        "    /mnt/$name-$opt\n    echo \"WAVE-RESULT $name-$opt $?\"\n"
        "  done\ndone\n"
    )
    iso = work / "cases.iso"
    run("genisoimage", "-quiet", "-R", "-o", iso, iso_root)
    overlay = work / "guest.qcow2"
    run("qemu-img", "create", "-f", "qcow2", "-F", "qcow2", "-b", args.image.resolve(), overlay)
    block_device = "virtio-blk-device" if args.arch == "riscv64" else "virtio-blk-pci"
    command = [f"qemu-system-{ARCHES[args.arch]}", "-m", "768", "-smp", "2", "-accel", "tcg",
        "-drive", f"file={overlay},format=qcow2,if=none,id=system",
        "-device", f"{block_device},drive=system",
        "-drive", f"file={iso},format=raw,if=none,id=cases,readonly=on",
        "-device", f"{block_device},drive=cases", "-nographic", "-monitor", "none", "-nic", "none"]
    if args.arch == "arm64":
        command += ["-machine", "virt", "-cpu", "cortex-a72", "-bios", str(args.firmware.resolve())]
    elif args.arch == "riscv64":
        # FreeBSD 14.3 boots with the SBI timer on QEMU 10; its Sstc path stalls.
        command += ["-machine", "virt,acpi=off", "-cpu", "rv64,sstc=false", "-bios", "default",
            "-kernel", str(args.kernel.resolve()), "-append", "-s"]
    with (work / "guest.log").open("wb") as log:
        process = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        console = Console(process, log)
        try:
            if args.arch == "riscv64":
                # Direct boot preserves QEMU's FDT for FreeBSD 14.3. A kernel
                # without loader-provided root metadata asks for its root disk.
                prompt = console.expect(r"mountroot> |RETURN for /bin/sh:")
                if prompt.group(0).startswith("mountroot>"):
                    console.send("ufs:/dev/vtbd0p3\n")
                    console.expect("RETURN for /bin/sh:")
            else:
                console.expect("Autoboot in")
                console.send("3", paced=True)
                console.expect(r"OK ")
                if args.arch == "amd64":
                    console.send("set console=comconsole\r", paced=True)
                    console.expect(r"OK ")
                console.send("boot -s\r", paced=True)
                console.expect("RETURN for /bin/sh:")
            console.send("\r")
            console.expect(r"root@[^\r\n]*# ")
            console.send("mount -uw /\nmount -t cd9660 /dev/vtbd1 /mnt\nsh /mnt/run.sh\n")
            for source in cases:
                for opt in ("O0", "O2"):
                    result = console.expect(rf"WAVE-RESULT {source.stem}-{opt} (\d+)\r", timeout=60)
                    status = int(result.group(1))
                    if status:
                        raise RuntimeError(f"{args.arch} {source.stem}-{opt} exited {status}; see {log.name}")
                    print(f"PASS {args.arch} {source.stem}-{opt}", flush=True)
            console.send("poweroff\n")
            if process.wait(timeout=60) != 0:
                raise RuntimeError(f"QEMU failed during shutdown; see {log.name}")
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
