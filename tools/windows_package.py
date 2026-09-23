"""Fail-closed native PE inspection and package dependency closure."""
# SPDX-License-Identifier: MPL-2.0
from collections import deque
from pathlib import Path
import re
import struct
import subprocess

MACHINES = {"x86_64-pc-windows-msvc": 0x8664, "aarch64-pc-windows-msvc": 0xAA64}
SYSTEM_DLLS = set("""advapi32 bcrypt bcryptprimitives cabinet cfgmgr32 comdlg32
    crypt32 cryptbase dbghelp dbgcore dnsapi dwmapi gdi32 imagehlp imm32 iphlpapi
    kernel32 ncrypt netapi32 ntdll ole32 oleaut32 powrprof profapi psapi rpcrt4
    secur32 setupapi shell32 shlwapi user32 userenv ucrtbase uuid version winhttp
    winmm winspool wintrust ws2_32 wtsapi32 normaliz msvcrt""".split())
VC_RUNTIME = re.compile(r"^(?:vcruntime140(?:_1)?|msvcp140(?:_\d+|_atomic_wait|_codecvt_ids)?|concrt140)\.dll$", re.I)


def system_dll(name):
    name = name.lower()
    return name.removesuffix(".dll") in SYSTEM_DLLS or name.startswith(("api-ms-win-", "ext-ms-win-"))


def pe_machine(path):
    path = Path(path)
    data = path.read_bytes()
    def fail(reason):
        raise ValueError(f"{path}: {reason}")
    if len(data) < 64 or data[:2] != b"MZ":
        fail("missing PE DOS header")
    offset = struct.unpack_from("<I", data, 60)[0]
    if offset < 64 or offset + 24 > len(data) or data[offset:offset + 4] != b"PE\0\0":
        fail("invalid PE header offset/signature")
    machine, sections = struct.unpack_from("<HH", data, offset + 4)
    optional_size = struct.unpack_from("<H", data, offset + 20)[0]
    optional = offset + 24
    table = optional + optional_size
    if optional_size < 112 or table + sections * 40 > len(data):
        fail("truncated PE optional header or section table")
    if struct.unpack_from("<H", data, optional)[0] != 0x20b:
        fail("expected PE32+ optional header")
    for index in range(sections):
        raw_size, raw_offset = struct.unpack_from("<II", data, table + index * 40 + 16)
        if raw_size and (raw_offset < table + sections * 40 or raw_offset + raw_size > len(data)):
            fail("truncated or overlapping PE section")
    return machine


def imports(path, inspector):
    pe_machine(path)
    inspector = Path(inspector)
    if not inspector.is_file():
        raise FileNotFoundError(f"{path}: required PE inspector is missing: {inspector}")
    result = subprocess.run([str(inspector), "--coff-imports", str(path)],
                            capture_output=True, text=True, errors="replace", timeout=30)
    if result.returncode or "Format: COFF-" not in result.stdout or result.stderr.strip():
        raise ValueError(f"{path}: PE import inspection failed ({result.returncode}): "
                         + result.stderr + result.stdout)
    names = []
    scopes = []
    name = None

    def incomplete():
        raise ValueError(f"{path}: incomplete PE import inspection")

    for line in result.stdout.splitlines():
        line = line.strip()
        if line.endswith("{"):
            kind = line[:-1].strip()
            # DelayImport contains per-symbol Import records, not more DLLs.
            if not scopes:
                if kind not in ("Import", "DelayImport"):
                    incomplete()
                name = None
            elif scopes != ["DelayImport"] or kind != "Import":
                incomplete()
            scopes.append(kind)
        elif line == "}":
            if not scopes:
                incomplete()
            if len(scopes) == 1:
                if name is None:
                    incomplete()
                names.append(name)
            scopes.pop()
        elif line.startswith("Name:"):
            if len(scopes) != 1 or name is not None:
                incomplete()
            name = line.removeprefix("Name:").strip()
    if scopes:
        incomplete()
    for name in names:
        if not name or Path(name).name != name or any(c in name for c in ("/", "\\", ":", "\0")):
            raise ValueError(f"{path}: invalid imported DLL name {name!r}")
    return names


def dependency_closure(binaries, target, inspector, search_dirs, runtime_dirs=()):
    """Return redistributable payloads and verified VC-runtime prerequisites.

    VC runtime DLLs are required installed prerequisites, never assumed to be
    Windows system DLLs or copied from an arbitrary Visual Studio tree.
    """
    expected = MACHINES[target]
    queue = deque(Path(p) for p in binaries)
    seen = set()
    resolved_names = {}
    payloads, prerequisites = {}, {}
    directories = [Path(p) for p in search_dirs]
    runtime_directories = [Path(p) for p in runtime_dirs]
    while queue:
        binary = queue.popleft().resolve()
        if binary in seen:
            continue
        seen.add(binary)
        if pe_machine(binary) != expected:
            raise ValueError(f"{binary}: wrong PE machine for {target}")
        for name in imports(binary, inspector):
            lower = name.lower()
            if any(part in lower for part in ("libgcc", "libstdc++", "winpthread", "mingw")):
                raise ValueError(f"{binary}: retired MinGW dependency {name}")
            if system_dll(name):
                continue
            runtime = bool(VC_RUNTIME.fullmatch(name))
            candidates = runtime_directories if runtime else [binary.parent, *directories]
            resolved = None
            for directory in candidates:
                if directory.is_dir():
                    resolved = next((p for p in directory.iterdir() if p.is_file() and p.name.lower() == lower), None)
                    if resolved:
                        break
            if resolved is None:
                raise FileNotFoundError(f"{binary}: missing {'VC runtime prerequisite' if runtime else 'dependency'} {name}")
            resolved = resolved.resolve()
            previous = resolved_names.get(lower)
            if previous and previous != resolved and previous.read_bytes() != resolved.read_bytes():
                raise ValueError(f"conflicting DLL providers for {name}: {previous}, {resolved}")
            resolved_names[lower] = resolved
            (prerequisites if runtime else payloads)[lower] = resolved
            queue.append(resolved)
    return payloads, prerequisites
