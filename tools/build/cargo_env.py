# SPDX-License-Identifier: MPL-2.0
"""Prepare the LLVM environment for one Cargo command selected by Meson."""

import argparse
import os
import platform
import shlex
import shutil
import subprocess
from pathlib import Path

from tools.build.runtime import (
    ROOT,
    is_freebsd_target,
    is_linux_target,
    is_windows_target,
)

TARGET_DIR = ROOT / "target"


def target_darwin_arch(target):
    if target.startswith("aarch64-"):
        return "arm64"
    if target.startswith("x86_64-"):
        return "x86_64"
    return None


def early_llvm_prefix():
    for env_name in ["WAVE_LLVM_HOME", "LLVM_SYS_211_PREFIX"]:
        value = os.environ.get(env_name)
        if value:
            path = Path(value)
            if path.exists():
                return path

    llvm_config = os.environ.get("LLVM_CONFIG_PATH") or shutil.which("llvm-config")
    if llvm_config:
        result = subprocess.run(
            [llvm_config, "--prefix"],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        if result.returncode == 0:
            path = Path(result.stdout.strip())
            if path.exists():
                return path
    return None


def llvm_dylib_arches():
    prefix = early_llvm_prefix()
    if prefix is None:
        return set()

    lib_dir = prefix / "lib"
    candidates = [
        *lib_dir.glob("libLLVM-*.dylib"),
        lib_dir / "libLLVM.dylib",
        lib_dir / "libLLVM-C.dylib",
    ]

    for candidate in candidates:
        if not candidate.exists():
            continue
        result = subprocess.run(
            ["lipo", "-info", str(candidate)],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        if result.returncode != 0:
            continue

        text = result.stdout.strip()
        if "are:" in text:
            return set(text.split("are:", 1)[1].strip().split())
        if "architecture:" in text:
            return {text.rsplit("architecture:", 1)[1].strip()}
    return set()


def is_target_buildable_with_current_llvm(target):
    if os.environ.get("WAVE_CROSS_LLVM_TARGET") == target:
        return True
    if target.endswith("unknown-linux-gnu"):
        target_arch = target.split("-", 1)[0]
        if target_arch == "riscv64gc":
            target_arch = "riscv64"
        host_arch = {
            "AMD64": "x86_64",
            "arm64": "aarch64",
        }.get(platform.machine(), platform.machine())
        return target_arch == host_arch

    if platform.system() == "Windows" and "windows" in target:
        target_arch = target.split("-", 1)[0]
        host_arch = {
            "AMD64": "x86_64",
            "ARM64": "aarch64",
        }.get(platform.machine(), platform.machine())
        return target_arch == host_arch

    if not target.endswith("apple-darwin"):
        return True

    target_arch = target_darwin_arch(target)
    if target_arch is None:
        return True

    arches = llvm_dylib_arches()
    if not arches:
        return platform.machine() == target_arch
    return target_arch in arches


def configure_windows_release_env(env, target):
    if platform.system() != "Windows" or not is_target_buildable_with_current_llvm(
        target
    ):
        raise RuntimeError(
            f"{target} release builds require a matching native Windows host"
        )
    prefix = early_llvm_prefix()
    llvm_config = env.get("LLVM_CONFIG_PATH") or shutil.which("llvm-config")
    if prefix is None or llvm_config is None:
        raise RuntimeError(
            "Native Windows releases require the LLVM 21 SDK and llvm-config.exe"
        )
    env["LLVM_SYS_211_PREFIX"] = str(prefix)
    env["LLVM_CONFIG_PATH"] = str(llvm_config)


def append_env_words(env, name, words):
    current = env.get(name, "").strip()
    addition = " ".join(words)
    env[name] = f"{current} {addition}" if current else addition


def configure_linux_release_env(env):
    # Use legacy DT_RPATH intentionally: unlike DT_RUNPATH, it applies to
    # transitive dependencies such as libLLVM -> libffi in clean containers.
    append_env_words(
        env,
        "RUSTFLAGS",
        [
            "-C",
            "link-arg=-Wl,--disable-new-dtags",
            "-C",
            "link-arg=-Wl,-z,origin",
            "-C",
            "link-arg=-Wl,-rpath,$ORIGIN/llvm/lib",
        ],
    )


def configure_cross_llvm_env(env, target):
    """llvm-sys discovers PREFIX/bin/llvm-config, not LLVM_CONFIG_PATH."""
    config = Path(env["LLVM_CONFIG_PATH"]).resolve()
    prefix = TARGET_DIR / "cross-llvm-config" / target
    wrapper = prefix / "bin/llvm-config"
    wrapper.parent.mkdir(parents=True, exist_ok=True)
    wrapper.write_text("#!/bin/sh\nexec " + shlex.quote(str(config)) + ' "$@"\n')
    wrapper.chmod(0o755)
    env["LLVM_SYS_211_PREFIX"] = str(prefix)

    # Validate the same executable llvm-sys will query, before starting Cargo.
    def query(*args):
        return subprocess.run(
            [wrapper, *args], env=env, check=True, capture_output=True, text=True
        ).stdout.strip()

    libraries = query("--libnames", "--link-shared").split()
    directory = Path(query("--libdir"))
    if not libraries or not all(
        name.startswith("libLLVM") and (directory / name).is_file()
        for name in libraries
    ):
        raise RuntimeError(
            f"cross llvm-config returned missing shared LLVM libraries: {libraries}"
        )
    system = query("--system-libs", "--link-shared")
    if "-lffi" in system.split():
        raise RuntimeError(
            "cross LLVM was built without FFI but llvm-config requested libffi"
        )
    print(
        f"Cross LLVM link inputs: {directory}: {libraries}; system libraries: {system}",
        flush=True,
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--release", action="store_true")
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command
    if command[:1] == ["--"]:
        command = command[1:]
    if not command:
        parser.error("a Cargo command is required")
    env = os.environ.copy()
    if not is_target_buildable_with_current_llvm(args.target):
        raise RuntimeError(f"LLVM runtime architecture does not match {args.target}")
    if is_windows_target(args.target):
        configure_windows_release_env(env, args.target)
    elif args.release and (
        is_linux_target(args.target) or is_freebsd_target(args.target)
    ):
        configure_linux_release_env(env)
    if env.get("WAVE_CROSS_LLVM_TARGET") == args.target:
        configure_cross_llvm_env(env, args.target)
    subprocess.run(command, cwd=ROOT, env=env, check=True)
    # Publish an actual Meson output only after Cargo succeeds. Cargo owns its
    # dependency graph, so this adapter is invoked on every Meson compile.
    args.output.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(args.binary, args.output)


if __name__ == "__main__":
    main()
