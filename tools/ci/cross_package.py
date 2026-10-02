# SPDX-License-Identifier: MPL-2.0
"""Build the RISC-V distribution on amd64; emulate only target configuration/smoke."""
import hashlib
import json
from pathlib import Path
import shlex
import tarfile

from tools.ci.common import ROOT
from tools.ci.build import download, llvm_setup


def riscv_package(r, _):
    if not r.provision:
        raise ValueError("RISC-V toolchain provisioning requires --provision")
    llvm_setup(
        r,
        {
            "packages": [
                "build-essential",
                "cmake",
                "ninja-build",
                "patchelf",
                "qemu-user",
                "gcc-riscv64-linux-gnu",
                "g++-riscv64-linux-gnu",
                "libc6-dev-riscv64-cross",
            ]
        },
    )
    version = r.env["LLVM_SOURCE_VERSION"]
    root = Path(r.env.get("WAVE_RISCV64_BUILD_ROOT", "/tmp/wave-riscv64-release"))
    root.mkdir(parents=True, exist_ok=True)
    archive = root / f"llvm-project-{version}.src.tar.xz"
    digest = r.env["LLVM_SOURCE_SHA256"]
    if (
        not archive.is_file()
        or hashlib.sha256(archive.read_bytes()).hexdigest() != digest
    ):
        download(
            r,
            f"https://github.com/llvm/llvm-project/releases/download/llvmorg-{version}/{archive.name}",
            archive,
            digest,
        )
    source = root / f"llvm-project-{version}.src"
    if not source.is_dir():
        with tarfile.open(archive) as stream:
            stream.extractall(root, filter="data")
    build = root / "llvm-build"
    host_bin = Path(r.env["LLVM_SYS_211_PREFIX"]) / "bin"
    options = {
        "CMAKE_BUILD_TYPE": "Release",
        "CMAKE_SYSTEM_NAME": "Linux",
        "CMAKE_SYSTEM_PROCESSOR": "riscv64",
        "CMAKE_C_COMPILER": "riscv64-linux-gnu-gcc",
        "CMAKE_CXX_COMPILER": "riscv64-linux-gnu-g++",
        "CMAKE_EXE_LINKER_FLAGS": "-static-libgcc -static-libstdc++",
        "CMAKE_SHARED_LINKER_FLAGS": "-static-libgcc -static-libstdc++",
        "LLVM_NATIVE_TOOL_DIR": str(host_bin),
        "LLVM_HOST_TRIPLE": "riscv64-unknown-linux-gnu",
        "LLVM_DEFAULT_TARGET_TRIPLE": "riscv64-unknown-linux-gnu",
        "LLVM_TARGETS_TO_BUILD": "AArch64;LoongArch;RISCV;WebAssembly;X86",
        "LLVM_ENABLE_PROJECTS": "lld",
        "LLVM_BUILD_LLVM_DYLIB": "ON",
        "LLVM_LINK_LLVM_DYLIB": "ON",
        "LLVM_BUILD_TOOLS": "ON",
    }
    for feature in (
        "INCLUDE_TESTS",
        "INCLUDE_EXAMPLES",
        "INCLUDE_BENCHMARKS",
        "INCLUDE_DOCS",
        "ENABLE_BINDINGS",
        "ENABLE_FFI",
        "ENABLE_LIBEDIT",
        "ENABLE_LIBXML2",
        "ENABLE_ZLIB",
        "ENABLE_ZSTD",
    ):
        options["LLVM_" + feature] = "OFF"
    fingerprint = json.dumps(
        {
            "source": digest,
            "options": options,
            "compiler": r.run(["riscv64-linux-gnu-g++", "--version"]),
            "libc": hashlib.sha256(
                Path("/usr/riscv64-linux-gnu/lib/libc.so.6").read_bytes()
            ).hexdigest(),
        },
        sort_keys=True,
    )
    stamp = root / "toolchain.json"
    required = [
        build / "bin" / name
        for name in ("llvm-config", "llc", "llvm-as", "llvm-mc", "ld.lld")
    ]
    cached = (
        stamp.is_file()
        and stamp.read_text() == fingerprint
        and all(p.is_file() for p in required)
        and (build / "include/llvm/Config/llvm-config.h").is_file()
        and any((build / "lib").glob("libLLVM*.so*"))
    )
    if not cached:
        r.run(
            [
                "cmake",
                "-S",
                source / "llvm",
                "-B",
                build,
                "-G",
                "Ninja",
                *[f"-D{k}={v}" for k, v in options.items()],
            ]
        )
        r.run(
            [
                "cmake",
                "--build",
                build,
                "--parallel",
                "2",
                "--target",
                "llvm-config",
                "llc",
                "llvm-as",
                "llvm-mc",
                "lld",
            ],
            timeout=14400,
        )
        stamp.write_text(fingerprint)
    sysroot = Path("/usr/riscv64-linux-gnu")
    wrapper = root / "llvm-config-target"
    wrapper.write_text(
        "#!/bin/sh\nexec qemu-riscv64 -L "
        + shlex.quote(str(sysroot))
        + " "
        + shlex.quote(str(build / "bin/llvm-config"))
        + ' "$@"\n'
    )
    wrapper.chmod(0o755)
    target = "riscv64gc-unknown-linux-gnu"
    r.run(["rustup", "target", "add", target])
    for key, value in {
        "WAVE_CROSS_LLVM_TARGET": target,
        "WAVE_LLVM_HOME": build,
        "WAVE_LLVM_BIN": build / "bin",
        "WAVE_LLVM_LIB": build / "lib",
        "WAVE_HOST_LLVM_MC": host_bin / "llvm-mc",
        "WAVE_LLVM_MC": host_bin / "llvm-mc",
        "LLVM_SYS_211_PREFIX": build,
        "LLVM_CONFIG_PATH": wrapper,
        "CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER": "riscv64-linux-gnu-gcc",
        "CC_riscv64gc_unknown_linux_gnu": "riscv64-linux-gnu-gcc",
        "CXX_riscv64gc_unknown_linux_gnu": "riscv64-linux-gnu-g++",
        "QEMU_LD_PREFIX": sysroot,
        "WAVE_CROSS_SYSROOT": sysroot,
    }.items():
        r.setenv(key, value)
    r.run(["@python", "x.py", "release", target], timeout=3600)
    from tools.ci.package import package_smoke

    # binfmt registration is a workflow prerequisite: wavec also executes its
    # RISC-V linker and generated executable during this extracted-package test.
    package_smoke(r, {})
