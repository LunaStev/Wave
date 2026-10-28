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


"""Platform-specific LLVM/CRT payload handling for Meson distributions."""

import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
from pathlib import Path

from tools import windows_package

ROOT = Path(__file__).resolve().parents[2]
DIST_DIR = ROOT / "dist"


def is_windows_target(target):
    return target in windows_package.MACHINES


def is_darwin_target(target):
    return target.endswith("apple-darwin")


def is_freebsd_target(target):
    return target.endswith("-unknown-freebsd")


def is_linux_target(target):
    return "linux" in target


def require_tool(tool):
    if shutil.which(tool) is None:
        print(f"[!] Missing required tool: {tool}")
        sys.exit(1)


def llvm_config_path():
    return os.environ.get("LLVM_CONFIG_PATH") or shutil.which("llvm-config")


def llvm_config_dir(flag):
    llvm_config = llvm_config_path()
    if not llvm_config:
        return None

    result = subprocess.run(
        [llvm_config, flag],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if result.returncode != 0:
        return None

    path = Path(result.stdout.strip())
    return path if path.exists() else None


def llvm_prefix():
    for env_name in ["WAVE_LLVM_HOME", "LLVM_SYS_211_PREFIX"]:
        value = os.environ.get(env_name)
        if value:
            path = Path(value)
            if path.exists():
                return path

    return llvm_config_dir("--prefix")


def llvm_bin_dir():
    value = os.environ.get("WAVE_LLVM_BIN")
    if value:
        path = Path(value)
        if path.exists():
            return path

    path = llvm_config_dir("--bindir")
    if path is not None:
        return path

    prefix = llvm_prefix()
    if prefix is not None:
        path = prefix / "bin"
        if path.exists():
            return path
    return None


def llvm_lib_dir():
    value = os.environ.get("WAVE_LLVM_LIB")
    if value:
        path = Path(value)
        if path.exists():
            return path

    path = llvm_config_dir("--libdir")
    if path is not None:
        return path

    prefix = llvm_prefix()
    if prefix is not None:
        for name in ["lib64", "lib"]:
            path = prefix / name
            if path.exists():
                return path
    return None


def unique_existing_dirs(dirs):
    out = []
    seen = set()
    for directory in dirs:
        if directory is None:
            continue
        path = Path(directory)
        if not path.exists() or not path.is_dir():
            continue
        resolved = path.resolve()
        if resolved in seen:
            continue
        seen.add(resolved)
        out.append(resolved)
    return out


def windows_llvm_bin_dirs():
    return unique_existing_dirs(
        [
            *[
                Path(os.environ[name])
                for name in ("WAVE_WINDOWS_LLVM_BIN", "WAVE_LLVM_BIN")
                if os.environ.get(name)
            ],
            *([llvm_bin_dir()] if llvm_bin_dir() else []),
        ]
    )


def file_description(path):
    if shutil.which("file") is None:
        return ""
    result = subprocess.run(
        ["file", "-L", str(path)],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if result.returncode != 0:
        return ""
    return result.stdout.strip()


def expected_binary_arch_token(target):
    arch = target.split("-", 1)[0]
    if arch == "x86_64":
        return "x86-64"
    if arch == "aarch64":
        return "ARM aarch64"
    if arch in {"riscv64", "riscv64gc"}:
        return "RISC-V"
    if arch == "loongarch64":
        return "LoongArch"
    if arch in {"i386", "i586", "i686"}:
        return "Intel 80386"
    return None


def release_target_name(target):
    if target == "riscv64gc-unknown-linux-gnu":
        return "riscv64-linux-gnu"
    if is_freebsd_target(target):
        return target
    return target.replace("-unknown", "")


def is_binary_for_target(path, target):
    if is_windows_target(target):
        return windows_package.pe_machine(path) == windows_package.MACHINES[target]
    desc = file_description(path)
    if not desc:
        return True

    if (is_linux_target(target) or is_freebsd_target(target)) and "ELF" in desc:
        expected = expected_binary_arch_token(target)
        return expected is None or expected in desc

    return True


def require_binary_for_target(path, target, label):
    if is_binary_for_target(path, target):
        return

    print(f"[!] {label} has the wrong architecture for {target}: {path}")
    print(f"    {file_description(path)}")
    sys.exit(1)


def windows_tool_names(tool):
    if tool.lower().endswith(".exe"):
        names = [tool]
    else:
        names = [f"{tool}.exe"]
    if tool == "ld.lld":
        names.append("lld.exe")
    return names


def packaged_tool_name(tool, target):
    if is_windows_target(target) and not tool.lower().endswith(".exe"):
        return f"{tool}.exe"
    return tool


def find_release_tool(tool, target):
    if is_windows_target(target):
        for directory in windows_llvm_bin_dirs():
            for name in windows_tool_names(tool):
                candidate = directory / name
                if candidate.exists():
                    require_binary_for_target(candidate, target, "Windows LLVM tool")
                    return candidate.resolve()
        return None

    names = [tool]
    if platform.system() == "Windows" and not tool.endswith(".exe"):
        names.insert(0, f"{tool}.exe")

    dirs = []
    bin_dir = llvm_bin_dir()
    if bin_dir is not None:
        dirs.append(bin_dir)
    path_hit = shutil.which(tool)
    if path_hit:
        dirs.append(Path(path_hit).parent)

    for directory in dirs:
        for name in names:
            candidate = directory / name
            if candidate.exists():
                require_binary_for_target(candidate, target, "LLVM tool")
                return candidate.resolve()
    return None


def copy_executable(src, dst):
    dst.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, dst)
    mode = dst.stat().st_mode
    dst.chmod(mode | 0o755)


def copy_optional(src, dst):
    if src is None or not Path(src).exists():
        return None
    dst.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, dst)
    return dst


def copy_globbed_files(patterns, dst_dir, target=None):
    copied = []
    dst_dir.mkdir(parents=True, exist_ok=True)
    seen = set()
    for pattern in patterns:
        for src in pattern.parent.glob(pattern.name):
            if src.name in seen or not src.is_file():
                continue
            if target is not None and not is_binary_for_target(src, target):
                print(
                    f"[!] Skipping runtime with wrong architecture for {target}: {src}"
                )
                print(f"    {file_description(src)}")
                continue
            seen.add(src.name)
            dst = dst_dir / src.name
            shutil.copy2(src, dst)
            copied.append(dst)
    return copied


def copy_named_runtime(src, dst_dir, dst_name=None):
    if src is None:
        return None

    src = Path(src)
    if not src.exists() or not src.is_file():
        return None

    dst_dir.mkdir(parents=True, exist_ok=True)
    dst = dst_dir / (dst_name or src.name)
    if dst.exists():
        return dst
    shutil.copy2(src, dst)
    return dst


def llvm_tools_for_target(target):
    common = ["llc", "llvm-as", "llvm-mc"]
    if is_darwin_target(target):
        return [(tool, True) for tool in ["ld64.lld", "ld.lld", *common]]
    if is_windows_target(target):
        return [
            ("ld.lld", True),
            ("lld-link", True),
            ("llvm-readobj", True),
            *[(tool, True) for tool in common],
        ]
    return [(tool, True) for tool in ["ld.lld", *common]]


def copy_lld_tools(stage_dir, target):
    copied = []
    tool_dir = stage_dir / "llvm" / "bin"
    missing_optional = []
    for tool, required in llvm_tools_for_target(target):
        src = find_release_tool(tool, target)
        if src is None:
            if not required:
                missing_optional.append(tool)
                continue
            print(f"[!] Missing LLVM tool for package: {tool}")
            if is_windows_target(target):
                print(
                    f"    Expected a matching Windows executable in: {', '.join(str(p) for p in windows_llvm_bin_dirs())}"
                )
                print(
                    "    Set WAVE_WINDOWS_LLVM_BIN to a complete native Windows LLVM/LLD bin directory."
                )
            sys.exit(1)
        require_binary_for_target(src, target, "LLVM tool")
        dst = tool_dir / packaged_tool_name(tool, target)
        copy_executable(src, dst)
        copied.append((src, dst))

    if missing_optional and is_windows_target(target):
        print("[!] Windows package is missing optional LLD tools:")
        for tool in missing_optional:
            print(f"    missing optional tool: {tool}.exe")
    return copied


def llvm_tool_run_env(tool):
    env = os.environ.copy()
    tool = Path(tool)
    if tool.parent.name == "bin":
        lib_dir = tool.parent.parent / "lib"
        if lib_dir.is_dir():
            current = env.get("LD_LIBRARY_PATH", "")
            env["LD_LIBRARY_PATH"] = f"{lib_dir}:{current}" if current else str(lib_dir)
    return env


def linux_crt_specs():
    return [
        ("x86_64-unknown-linux-gnu", None, "x86_64", None),
        ("aarch64-unknown-linux-gnu", None, "aarch64", None),
        (
            "riscv64-unknown-linux-gnu",
            "lp64",
            "riscv64",
            "+m,+a,+c,+zicsr,+zifencei",
        ),
        (
            "riscv64-unknown-linux-gnu",
            "lp64f",
            "riscv64",
            "+m,+a,+f,+c,+zicsr,+zifencei",
        ),
        (
            "riscv64-unknown-linux-gnu",
            "lp64d",
            "riscv64",
            "+m,+a,+f,+d,+c,+zicsr,+zifencei",
        ),
        (
            "loongarch64-unknown-linux-gnu",
            "lp64s",
            "loongarch64",
            "-f,-d,+lsx,+ual",
        ),
        (
            "loongarch64-unknown-linux-gnu",
            "lp64f",
            "loongarch64",
            "+f,-d,+lsx,+ual",
        ),
        (
            "loongarch64-unknown-linux-gnu",
            "lp64d",
            "loongarch64",
            "+f,+d,+lsx,+ual",
        ),
    ]


def llvm_triple_for_abi(target, abi):
    if target == "loongarch64-unknown-linux-gnu":
        if abi == "lp64s":
            return "loongarch64-unknown-linux-gnusf"
        if abi == "lp64f":
            return "loongarch64-unknown-linux-gnuf32"
    return target


def write_linux_crt_objects(stage_dir, package_target):
    if not is_linux_target(package_target):
        return []

    host_llvm_mc = os.environ.get("WAVE_HOST_LLVM_MC", "").strip()
    llvm_mc = (
        Path(host_llvm_mc)
        if host_llvm_mc
        else find_release_tool("llvm-mc", package_target)
    )
    if llvm_mc is None:
        print("[!] Missing LLVM tool for Linux CRT object: llvm-mc")
        sys.exit(1)

    objects = []
    for target, abi, architecture, attributes in linux_crt_specs():
        source_dir = ROOT / "llvm" / "crt" / "linux" / architecture

        dst_dir = stage_dir / "crt" / target
        if abi is not None:
            dst_dir /= abi
        dst_dir.mkdir(parents=True, exist_ok=True)

        for object_name, source_name in [
            ("crt1.o", "crt1.s"),
            ("Scrt1.o", "crt1.s"),
            ("rcrt1.o", "crt1.s"),
            ("crti.o", "crti.s"),
            ("crtn.o", "crtn.s"),
        ]:
            source = source_dir / source_name
            if not source.exists():
                print(f"[!] Missing Linux CRT source for {target}: {source}")
                sys.exit(1)
            dst = dst_dir / object_name
            command = [
                str(llvm_mc),
                f"-triple={llvm_triple_for_abi(target, abi)}",
                "-filetype=obj",
            ]
            if attributes is not None:
                command.append(f"-mattr={attributes}")
            if abi is not None and target != "loongarch64-unknown-linux-gnu":
                command.append(f"-target-abi={abi}")
            command.extend([str(source), "-o", str(dst)])
            subprocess.run(
                command,
                env=llvm_tool_run_env(llvm_mc),
                check=True,
            )
            objects.append(dst)
    return objects


def ldd_shared_libs(binary):
    if shutil.which("ldd") is None:
        return []

    result = subprocess.run(
        ["ldd", str(binary)],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if result.returncode != 0:
        return []

    libs = []
    for line in result.stdout.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("linux-vdso"):
            continue

        path_text = None
        if "=>" in stripped:
            rhs = stripped.split("=>", 1)[1].strip()
            if rhs.startswith("/"):
                path_text = rhs.split(" ", 1)[0]
        elif stripped.startswith("/"):
            path_text = stripped.split(" ", 1)[0]

        if path_text:
            path = Path(path_text)
            if path.exists():
                libs.append(path)

    return libs


def is_glibc_core_runtime(path):
    name = path.name
    return (
        name == "libc.so.6"
        or name.startswith("ld-linux")
        or name
        in {
            "libdl.so.2",
            "libm.so.6",
            "libpthread.so.0",
            "librt.so.1",
            "libresolv.so.2",
            "libutil.so.1",
        }
    )


def cross_shared_libs(binary):
    """Inspect foreign ELF dependencies without running the target dynamic loader."""
    sysroot = Path(os.environ["WAVE_CROSS_SYSROOT"])
    result = subprocess.run(
        ["readelf", "-d", str(binary)], check=True, text=True, capture_output=True
    )
    needed = re.findall(r"\(NEEDED\).*?\[([^\]]+)\]", result.stdout)
    directories = [
        Path(binary).parent,
        llvm_lib_dir(),
        sysroot / "lib",
        sysroot / "lib64",
        sysroot / "usr/lib",
        sysroot / "usr/lib64",
    ]
    found = []
    for name in needed:
        if is_glibc_core_runtime(Path(name)):
            continue
        source = next(
            (
                directory / name
                for directory in directories
                if directory is not None and (directory / name).is_file()
            ),
            None,
        )
        if source is None:
            raise FileNotFoundError(
                f"unresolved target dependency {name} required by {binary}"
            )
        found.append(source)
    return found


def copy_linux_runtime_deps(stage_dir, binaries):
    root_lib_dir = stage_dir / "llvm" / "lib"
    copied = []
    queue = [Path(p) for p in binaries if Path(p).exists()]
    seen = set()

    while queue:
        current = queue.pop(0)
        for dep in (
            cross_shared_libs(current)
            if os.environ.get("WAVE_CROSS_SYSROOT")
            else ldd_shared_libs(current)
        ):
            resolved = dep.resolve()
            if (
                resolved in seen
                or is_glibc_core_runtime(dep)
                or (
                    platform.system() == "FreeBSD"
                    and (
                        str(dep).startswith("/lib/") or str(dep).startswith("/usr/lib/")
                    )
                )
            ):
                continue
            seen.add(resolved)

            dst = copy_named_runtime(dep, root_lib_dir)
            if dst is not None:
                copied.append(dst)
                queue.append(dst)

    return copied


def copy_windows_runtime_deps(stage_dir, binaries, target):
    inspector = find_release_tool("llvm-readobj", target)
    if inspector is None:
        raise RuntimeError(
            "Windows package dependency inspection requires llvm-readobj.exe"
        )
    runtime_dirs = (
        [Path(os.environ["SystemRoot"]) / "System32"]
        if os.environ.get("SystemRoot")
        else []
    )
    payloads, prerequisites = windows_package.dependency_closure(
        binaries, target, inspector, windows_llvm_bin_dirs(), runtime_dirs
    )
    copied = []
    for source in payloads.values():
        for directory in (stage_dir, stage_dir / "llvm/bin"):
            copied.append(copy_optional(source, directory / source.name))
    (stage_dir / "windows-dependencies.json").write_text(
        json.dumps(
            {
                "target": target,
                "payloads": {name: str(path) for name, path in payloads.items()},
                "vc_runtime_prerequisites": list(prerequisites),
            },
            indent=2,
        ),
        encoding="utf-8",
    )
    return copied


def verify_packaged_runtime_arch(stage_dir, target):
    paths = []
    for directory in [
        stage_dir,
        stage_dir / "llvm" / "bin",
        stage_dir / "llvm" / "lib",
    ]:
        if directory.exists():
            paths.extend(path for path in directory.iterdir() if path.is_file())

    for path in paths:
        if is_windows_target(target):
            if path.suffix.lower() in (".exe", ".dll"):
                require_binary_for_target(path, target, "Packaged runtime")
            continue
        desc = file_description(path)
        if "ELF" not in desc and "PE32" not in desc:
            continue
        require_binary_for_target(path, target, "Packaged runtime")


def resolve_dylib_reference(ref, binary, extra_dirs=None):
    path = Path(ref)
    if path.is_absolute():
        return path if path.exists() else None

    name = path.name
    if ref.startswith(("@loader_path/", "@executable_path/")):
        candidate = Path(binary).parent / ref.split("/", 1)[1]
        if candidate.is_file():
            return candidate
    search_dirs = []
    if ref.startswith("@loader_path/"):
        search_dirs.append(Path(binary).parent)
    if ref.startswith("@executable_path/"):
        search_dirs.append(Path(binary).parent)
    if extra_dirs:
        search_dirs.extend(extra_dirs)
    if ref.startswith("@rpath/"):
        commands = subprocess.run(
            ["otool", "-l", str(binary)], check=True, text=True, capture_output=True
        ).stdout
        for value in re.findall(
            r"cmd LC_RPATH\s+cmdsize \d+\s+path (.+?) \(offset", commands
        ):
            value = value.replace("@loader_path", str(Path(binary).parent)).replace(
                "@executable_path", str(Path(binary).parent)
            )
            search_dirs.append(Path(value))

    for directory in search_dirs:
        candidate = directory / name
        if candidate.exists():
            return candidate
    return None


def copy_darwin_dependency_closure(stage_dir, roots):
    """Copy every non-system dylib and rewrite references before signing."""
    library_dir = stage_dir / "llvm/lib"
    pending = [(Path(source), Path(staged)) for source, staged in roots]
    seen = set()
    origins = {}
    while pending:
        source, staged = pending.pop(0)
        if staged in seen:
            continue
        seen.add(staged)
        for ref in dylib_references(source):
            search = [source.parent.parent / "lib"]
            compiler_lib = llvm_lib_dir()
            if compiler_lib is not None:
                search.append(compiler_lib)
            dependency = resolve_dylib_reference(ref, source, search)
            if dependency is None:
                raise FileNotFoundError(f"cannot resolve {ref} required by {source}")
            name = Path(ref).name
            destination = library_dir / name
            if (
                destination.exists()
                and origins.get(destination) != dependency.resolve()
                and destination.read_bytes() != dependency.read_bytes()
            ):
                name = (
                    hashlib.sha256(dependency.read_bytes()).hexdigest()[:12]
                    + "-"
                    + name
                )
                destination = library_dir / name
            origins[destination] = dependency.resolve()
            if not destination.exists():
                copy_optional(dependency, destination)
            prefix = (
                "@loader_path"
                if staged.parent == library_dir
                else (
                    "@executable_path/llvm/lib"
                    if staged.parent == stage_dir
                    else "@executable_path/../lib"
                )
            )
            subprocess.run(
                ["install_name_tool", "-change", ref, f"{prefix}/{name}", str(staged)],
                check=True,
            )
            pending.append((dependency, destination))
    return list(library_dir.glob("*.dylib"))


def copy_llvm_runtime_libs(stage_dir, target, lld_tool_paths, runtime_roots=None):
    copied = []
    root_lib_dir = stage_dir / "llvm" / "lib"
    lib_dir = llvm_lib_dir()

    if is_windows_target(target):
        dep_roots = list(runtime_roots or [])
        dep_roots.extend(staged for _, staged in lld_tool_paths)
        copied.extend(copy_windows_runtime_deps(stage_dir, dep_roots, target))
        return [p for p in copied if p is not None]

    if is_darwin_target(target):
        return copy_darwin_dependency_closure(
            stage_dir,
            [(path, path) for path in (runtime_roots or [])] + list(lld_tool_paths),
        )

    patterns = []
    if lib_dir is not None and (is_linux_target(target) or is_freebsd_target(target)):
        patterns.extend([lib_dir / "libLLVM*.so*", lib_dir / "liblld*.so*"])

    for tool_src, _ in lld_tool_paths:
        tool_lib_dir = tool_src.parent.parent / "lib"
        if tool_lib_dir.exists() and (
            is_linux_target(target) or is_freebsd_target(target)
        ):
            patterns.extend(
                [tool_lib_dir / "libLLVM*.so*", tool_lib_dir / "liblld*.so*"]
            )

    copied.extend(copy_globbed_files(patterns, root_lib_dir, target))
    if is_linux_target(target) or is_freebsd_target(target):
        dep_roots = list(runtime_roots or [])
        dep_roots.extend(staged for _, staged in lld_tool_paths)
        dep_roots.extend(root_lib_dir.glob("*.so*"))
        copied.extend(copy_linux_runtime_deps(stage_dir, dep_roots))
    return copied


def dylib_references(binary):
    if shutil.which("otool") is None:
        raise FileNotFoundError("macOS dependency inspection requires otool")
    result = subprocess.run(
        ["otool", "-L", str(binary)],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    result.check_returncode()

    refs = []
    for line in result.stdout.splitlines()[1:]:
        ref = line.strip().split(" ", 1)[0]
        if ref.startswith(("/usr/lib/", "/System/")):
            continue
        refs.append(ref)
    return refs


def linux_binary_has_runpath(binary, expected):
    if shutil.which("readelf") is None:
        return True

    result = subprocess.run(
        ["readelf", "-d", str(binary)],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if result.returncode != 0:
        return False
    return expected in result.stdout


def patch_linux_binary(binary, rpath):
    if shutil.which("patchelf") is None:
        return
    subprocess.run(
        ["patchelf", "--force-rpath", "--set-rpath", rpath, str(binary)], check=True
    )


def codesign_macos_binary(binary):
    if shutil.which("codesign") is None:
        raise FileNotFoundError("macOS packages require codesign")

    subprocess.run(
        ["codesign", "--force", "--sign", "-", str(binary)],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=True,
    )


def patch_staged_runtime(stage_dir, target, binary_path, lld_tool_paths):
    if is_darwin_target(target):
        for dylib in (stage_dir / "llvm" / "lib").glob("*.dylib"):
            subprocess.run(
                ["install_name_tool", "-id", f"@loader_path/{dylib.name}", str(dylib)],
                check=True,
            )
        codesign_macos_binary(binary_path)
        for _, staged in lld_tool_paths:
            if staged.exists():
                codesign_macos_binary(staged)
        for dylib in (stage_dir / "llvm" / "lib").glob("*.dylib"):
            codesign_macos_binary(dylib)
    elif is_linux_target(target) or is_freebsd_target(target):
        patch_linux_binary(binary_path, "$ORIGIN/llvm/lib")
        if not linux_binary_has_runpath(binary_path, "$ORIGIN/llvm/lib"):
            print(f"[!] Missing RPATH/RUNPATH in {binary_path.name}")
            print("    Linux release packages must keep wavec as an ELF binary and")
            print("    resolve bundled LLVM from $ORIGIN/llvm/lib.")
            print(
                "    Rebuild with meson compile so Cargo embeds the release runtime path."
            )
            sys.exit(1)
        for _, staged in lld_tool_paths:
            if staged.exists():
                patch_linux_binary(staged, "$ORIGIN/../lib")


def copy_windows_msvc_resources(stage_dir, target):
    prefix = llvm_prefix()
    if prefix is None:
        raise RuntimeError("Windows MSVC packaging requires the LLVM 21 SDK prefix")
    arch = target.split("-", 1)[0]
    relative = Path("lib/clang/21/lib/windows") / f"clang_rt.builtins-{arch}.lib"
    builtins = prefix / relative
    if not builtins.is_file():
        raise FileNotFoundError(f"required arithmetic builtins archive: {builtins}")
    destination = stage_dir / "llvm" / relative
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(builtins, destination)
    licenses = stage_dir / "licenses"
    licenses.mkdir()
    for component, name in [("llvm", "LLVM"), ("compiler-rt", "compiler-rt")]:
        notice = prefix / "wave-notices" / (component + ".txt")
        if not notice.is_file():
            raise FileNotFoundError(
                f"pinned {component} notice missing: {notice}; run the native SDK provisioner"
            )
        shutil.copy2(notice, licenses / (name + ".txt"))
    libxml = Path(os.environ.get("WAVE_LIBXML2_LICENSE", ""))
    if not libxml.is_file():
        raise FileNotFoundError(
            "WAVE_LIBXML2_LICENSE must identify the pinned library's notice"
        )
    shutil.copy2(libxml, licenses / "libxml2.txt")
    shutil.copy2(ROOT / "LICENSE", licenses / "Wave.txt")
    shutil.copy2(ROOT / "std/LICENSE", licenses / "std.txt")
    shutil.copy2(ROOT / "README.md", stage_dir / "README.md")
