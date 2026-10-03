# SPDX-License-Identifier: MPL-2.0
"""Explicit one-time LLVM SDK builds; ordinary distribution jobs only download."""

import argparse
import json
from pathlib import Path
import shutil
import sys
import tarfile

from tools.ci.build import download, llvm_setup, loongarch_tools
from tools.ci.common import ROOT, Runner
from tools.ci.llvm_bundle import BACKENDS, fetch_bundle, identity, install_bundle, sha256
from tools.ci.targets import PINS, resolve

MANIFEST = Path(__file__).with_name("llvm_bundles.json")


def prerequisites(r):
    packages = ["build-essential", "cmake", "ninja-build", "patchelf", "qemu-user", "xz-utils"]
    if r.target.id == "linux-riscv64":
        packages += ["gcc-riscv64-linux-gnu", "g++-riscv64-linux-gnu", "libc6-dev-riscv64-cross"]
    llvm_setup(r, {"packages": packages})
    host_bin = Path(r.env["LLVM_SYS_211_PREFIX"]) / "bin"
    if r.target.id == "linux-loong64":
        loongarch_tools(r, {})
        sysroot = Path(r.env["WAVE_LOONGARCH64_SYSROOT"])
        prefix = str(sysroot.parent / "bin/loongarch64-unknown-linux-gnu-")
    else:
        sysroot = Path("/usr/riscv64-linux-gnu")
        prefix = "riscv64-linux-gnu-"
    return host_bin, sysroot, prefix


def options(target, host_bin, cross_prefix):
    triple = resolve(target).triple
    def tool(name):
        found = shutil.which(cross_prefix + name)
        if not found:
            raise FileNotFoundError(f"missing cross tool: {cross_prefix}{name}")
        # CMake's FILEPATH options otherwise resolve bare names against the checkout.
        return str(Path(found).resolve())

    values = {
        "CMAKE_BUILD_TYPE": "Release",
        "CMAKE_SYSTEM_NAME": "Linux",
        "CMAKE_SYSTEM_PROCESSOR": triple.split("-")[0],
        "CMAKE_C_COMPILER": tool("gcc"),
        "CMAKE_CXX_COMPILER": tool("g++"),
        "CMAKE_AR": tool("ar"),
        "CMAKE_RANLIB": tool("ranlib"),
        "CMAKE_STRIP": tool("strip"),
        "CMAKE_EXE_LINKER_FLAGS": "-static-libgcc -static-libstdc++",
        "CMAKE_SHARED_LINKER_FLAGS": "-static-libgcc -static-libstdc++",
        "CMAKE_INSTALL_PREFIX": "/opt/wave-llvm",
        "CMAKE_INSTALL_LIBDIR": "lib",
        "LLVM_LIBDIR_SUFFIX": "",
        "LLVM_NATIVE_TOOL_DIR": str(host_bin),
        "LLVM_HOST_TRIPLE": triple,
        "LLVM_DEFAULT_TARGET_TRIPLE": triple,
        "LLVM_TARGETS_TO_BUILD": ";".join(sorted(BACKENDS)),
        "LLVM_ENABLE_PROJECTS": "lld",
        "LLVM_BUILD_LLVM_DYLIB": "ON",
        "LLVM_LINK_LLVM_DYLIB": "ON",
        "LLVM_BUILD_TOOLS": "ON",
        "LLVM_INSTALL_TOOLCHAIN_ONLY": "OFF",
    }
    for feature in ("INCLUDE_TESTS", "INCLUDE_EXAMPLES", "INCLUDE_BENCHMARKS", "INCLUDE_DOCS",
                    "ENABLE_BINDINGS", "ENABLE_FFI", "ENABLE_LIBEDIT", "ENABLE_LIBXML2",
                    "ENABLE_ZLIB", "ENABLE_ZSTD"):
        values["LLVM_" + feature] = "OFF"
    return values


def build_bundle(r, output, revision):
    """This is the only source-build entry point; never called by packaging."""
    if not r.provision:
        raise ValueError("building an LLVM SDK requires --provision")
    target = r.target.id
    identity(target)
    output = Path(output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    host_bin, sysroot, prefix = prerequisites(r)
    version = PINS["LLVM_SOURCE_VERSION"]
    archive = r.temp / f"llvm-project-{version}.src.tar.xz"
    download(r, f"https://github.com/llvm/llvm-project/releases/download/llvmorg-{version}/{archive.name}",
             archive, PINS["LLVM_SOURCE_SHA256"])
    with tarfile.open(archive) as stream:
        stream.extractall(r.temp, filter="data")
    source = r.temp / f"llvm-project-{version}.src"
    build = r.temp / "llvm-build"
    settings = options(target, host_bin, prefix)
    r.run(["cmake", "-S", source / "llvm", "-B", build, "-G", "Ninja",
           *[f"-D{k}={v}" for k, v in settings.items()]])
    r.run(["cmake", "--build", build, "--parallel", "2", "--target",
           "llvm-config", "llc", "llvm-as", "llvm-mc", "lld", "llvm-headers"], timeout=14400)
    name = f"wave-llvm-{version}-{target}-{revision}"
    entry = install_bundle(r, build, source, output / name, target, sysroot)
    entry.update(
        revision=revision,
        filename=name + ".tar.xz",
        size_bytes=(output / (name + ".tar.xz")).stat().st_size,
        url=f"https://wave-lang.dev/downloads/toolchains/llvm/{version}/{revision}/{name}.tar.xz",
        build={"compiler": r.run([prefix + "g++", "--version"]).strip(),
               "options": settings, "source_commit": r.run(["git", "rev-parse", "HEAD"]).strip()},
    )
    (output / (name + ".json")).write_text(json.dumps(entry, indent=2) + "\n")
    # Keep only the actual publication files in the artifact directory.
    shutil.rmtree(output / name)
    print(json.dumps(entry, indent=2))
    return entry


def consume(r, sysroot):
    entries = json.loads(MANIFEST.read_text())["bundles"]
    entry = entries.get(r.target.id)
    if entry is None:
        raise ValueError(f"no published LLVM SDK pin for {r.target.id}; build and publish the SDK first")
    return fetch_bundle(r, entry, r.temp / "llvm-sdk", r.target.id, sysroot)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", choices=("linux-riscv64", "linux-loong64"), required=True)
    parser.add_argument("--output", type=Path, default=ROOT / "llvm-bundles")
    parser.add_argument("--revision", default="r1")
    parser.add_argument("--provision", action="store_true")
    parser.add_argument("--report-json", type=Path, default=ROOT / "llvm-build-report.json")
    args = parser.parse_args(argv)
    import re
    if not re.fullmatch(r"r[1-9][0-9]*", args.revision):
        parser.error("revision must be r followed by a positive integer")
    target = resolve(args.target)
    target.check_host()
    runner = Runner(target, args.report_json, provision=args.provision)
    stage = {"id": "llvm-sdk", "name": "Build and validate relocatable LLVM SDK", "status": "running", "commands": []}
    runner.current = stage
    runner.data["stages"].append(stage)
    try:
        build_bundle(runner, args.output, args.revision)
        stage["status"] = "pass"
        return 0
    except (OSError, ValueError, RuntimeError, tarfile.TarError) as error:
        stage.update(status="fail", reason=str(error))
        print(error, file=sys.stderr)
        return 1
    finally:
        runner.save()


if __name__ == "__main__":
    sys.exit(main())
