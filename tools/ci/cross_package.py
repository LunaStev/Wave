# SPDX-License-Identifier: MPL-2.0
"""Build native distributions using pinned SDKs; LLVM itself is built separately."""
from pathlib import Path
import shlex

from tools.ci.llvm_toolchain import prerequisites, consume


def cross_package(r, _):
    if not r.provision:
        raise ValueError("cross toolchain provisioning requires --provision")
    host_bin, sysroot, prefix = prerequisites(r)
    sdk = consume(r, sysroot)
    loong = r.target.id == "linux-loong64"
    qemu = "qemu-loongarch64" if loong else "qemu-riscv64"
    wrapper = r.temp / "llvm-config-target"
    wrapper.write_text("#!/bin/sh\nexec " + shlex.join([qemu, "-L", str(sysroot), str(sdk / "bin/llvm-config")]) + ' "$@"\n')
    wrapper.chmod(0o755)
    target = r.target.rust_target
    r.run(["rustup", "target", "add", target])
    values = {
        "WAVE_CROSS_LLVM_TARGET": target,
        "WAVE_LLVM_HOME": sdk,
        "WAVE_LLVM_BIN": sdk / "bin",
        "WAVE_LLVM_LIB": sdk / "lib",
        "WAVE_HOST_LLVM_MC": host_bin / "llvm-mc",
        "WAVE_LLVM_MC": host_bin / "llvm-mc",
        "LLVM_SYS_211_PREFIX": sdk,
        "LLVM_CONFIG_PATH": wrapper,
        f"CARGO_TARGET_{target.replace('-', '_').upper()}_LINKER": prefix + "gcc",
        "CC_" + target.replace("-", "_"): prefix + "gcc",
        "CXX_" + target.replace("-", "_"): prefix + "g++",
        "QEMU_LD_PREFIX": sysroot,
        "WAVE_CROSS_SYSROOT": sysroot,
    }
    if loong:
        values["WAVE_LOONGARCH64_SYSROOT"] = sysroot
    for key, value in values.items():
        r.setenv(key, value)
    # Both compiler and its linker must execute during the extracted std smoke.
    if not Path("/proc/sys/fs/binfmt_misc/" + qemu).is_file():
        raise ValueError(f"{qemu} binfmt registration is required for package validation")
    r.run(["@python", "x.py", "release", target], timeout=3600)
    from tools.ci.package import package_smoke
    package_smoke(r, {})


riscv_package = cross_package
loongarch_package = cross_package
