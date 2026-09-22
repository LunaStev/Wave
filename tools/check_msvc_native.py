# SPDX-License-Identifier: MPL-2.0
"""Required native MSVC hosted and bidirectional ABI execution (no skip mode)."""
import argparse
import ctypes
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import sys

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from tools.process_tree import run_process, timeout_output

ROOT = Path(__file__).resolve().parent.parent
TARGETS = {"x86_64-pc-windows-msvc": 0x8664, "aarch64-pc-windows-msvc": 0xAA64}


def pe_machine(path):
    data = Path(path).read_bytes()
    if len(data) < 64 or data[:2] != b"MZ":
        raise ValueError(f"not a PE image: {path}")
    offset = struct.unpack_from("<I", data, 60)[0]
    if offset + 6 > len(data) or data[offset:offset + 4] != b"PE\0\0":
        raise ValueError(f"invalid PE header: {path}")
    return struct.unpack_from("<H", data, offset + 4)[0]


def require_native(target, wavec, clang):
    if os.name != "nt":
        raise RuntimeError("MSVC runtime verification requires a native Windows host")
    from ctypes import wintypes
    api = ctypes.WinDLL("kernel32", use_last_error=True)
    api.GetCurrentProcess.restype = wintypes.HANDLE
    api.IsWow64Process2.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.USHORT), ctypes.POINTER(wintypes.USHORT)]
    api.IsWow64Process2.restype = wintypes.BOOL
    process, native = wintypes.USHORT(), wintypes.USHORT()
    if not api.IsWow64Process2(api.GetCurrentProcess(), ctypes.byref(process), ctypes.byref(native)):
        raise ctypes.WinError(ctypes.get_last_error())
    expected = TARGETS[target]
    if process.value != 0 or native.value != expected:
        raise RuntimeError(f"native {target} required; process={process.value:#x}, host={native.value:#x}")
    for tool in (wavec, clang):
        if pe_machine(tool) != expected:
            raise RuntimeError(f"tool is not native {target}: {tool}")
    if not os.environ.get("LIB") or not os.environ.get("INCLUDE"):
        raise RuntimeError("matching MSVC Developer Prompt LIB and INCLUDE are required")
    if not os.environ.get("VCToolsInstallDir"):
        raise RuntimeError("matching MSVC Developer Prompt VCToolsInstallDir is required")


class Audit:
    def __init__(self, options):
        self.options = options
        self.output = options.output.resolve()
        self.output.mkdir(parents=True, exist_ok=False)
        self.sources = self.output / "sources"
        shutil.copytree(ROOT / "tests/fixtures/msvc_native", self.sources)
        for suffix in ("c", "wave"):
            shutil.copy2(ROOT / "tests/fixtures/c_abi_edges" / ("interop." + suffix),
                         self.sources / ("abi_edges." + suffix))
        home = self.output / "home"
        shutil.copytree(ROOT / "std", home / ".wave/lib/wave/std")
        self.env = dict(os.environ, HOME=str(home), USERPROFILE=str(home), NO_COLOR="1")
        self.env["WAVE_LLVM_HOME"] = str(options.llvm_bin.parent)
        self.commands = []
        self.failures = []
        self.clang = options.llvm_bin / "clang-cl.exe"
        self.linker = options.llvm_bin / "lld-link.exe"
        self.readobj = options.llvm_bin / "llvm-readobj.exe"

    def command(self, args, directory, *, expected=0, stdout=None, diagnostic=None):
        args = list(map(str, args))
        record = {"command": args, "cwd": str(directory), "expected_exit": expected}
        self.commands.append(record)
        try:
            result = run_process(args, cwd=directory, env=self.env, timeout=90,
                                 capture_output=True, text=True, errors="replace")
            record.update(exit=result.returncode, stdout=result.stdout, stderr=result.stderr)
            if expected is None:
                if result.returncode == 0:
                    raise AssertionError("negative link fixture unexpectedly succeeded")
            elif result.returncode != expected:
                raise AssertionError(f"expected exit {expected}, got {result.returncode}")
            if stdout is not None and result.stdout.replace("\r\n", "\n") != stdout:
                raise AssertionError(f"unexpected stdout: {result.stdout!r}")
            if diagnostic and diagnostic.lower() not in (result.stdout + result.stderr).lower():
                raise AssertionError(f"missing diagnostic: {diagnostic}")
            return result
        except subprocess.TimeoutExpired as error:
            record.update(timeout=True, output=timeout_output(error))
            raise
        finally:
            (self.output / "commands.json").write_text(json.dumps(self.commands, indent=2), encoding="utf-8")

    def wave(self, source, directory, opt, crt, *extra):
        return [self.options.wavec, "build", self.sources / (source + ".wave"),
                "--target", self.options.target, f"-O{opt}",
                "-Clinker=" + str(self.linker), "--out-dir", directory,
                *(["--static"] if crt == "static" else []), *extra]

    def c_object(self, source, directory, opt, crt):
        obj = directory / (source + ".obj")
        self.command([self.clang, "--target=" + self.options.target, "/nologo", "/TC", "/c",
                      "/Od" if opt == 0 else "/O2", "/MT" if crt == "static" else "/MD",
                      "/Fo" + str(obj), self.sources / (source + ".c")], directory)
        self.command([self.clang, "--target=" + self.options.target, "/nologo", "/TC", "/c",
                      "/Od" if opt == 0 else "/O2", "/MT" if crt == "static" else "/MD",
                      "/clang:-S", "/clang:-emit-llvm", "/clang:-o", "/clang:" + str(directory / (source + "-c.ll")),
                      self.sources / (source + ".c")], directory)
        if struct.unpack_from("<H", obj.read_bytes())[0] != TARGETS[self.options.target]:
            raise AssertionError(f"C peer has the wrong COFF machine: {obj}")
        return obj

    def execute(self, executable, directory, expected, stdout):
        if pe_machine(executable) != TARGETS[self.options.target]:
            raise AssertionError(f"generated executable has wrong machine: {executable}")
        imports = self.command([self.readobj, "--coff-imports", executable], directory).stdout.lower()
        if any(name in imports for name in ("libgcc", "libstdc++", "libwinpthread")):
            raise AssertionError("MSVC fixture unexpectedly depends on a MinGW runtime")
        self.command([executable], directory, expected=expected, stdout=stdout)

    def case(self, name, action):
        directory = self.output / name
        directory.mkdir()
        try:
            action(directory)
            print(f"PASS {name}", flush=True)
        except (OSError, ValueError, RuntimeError, AssertionError, subprocess.TimeoutExpired) as error:
            self.failures.append({"case": name, "error": str(error)})
            print(f"FAIL {name}: {error}", flush=True)

    def hosted(self, directory, source, opt, crt, expected, stdout):
        exe = directory / (source + ".exe")
        self.command(self.wave(source, directory, opt, crt, "--emit=ir,obj,bin", "-o", exe), directory)
        self.command([self.readobj, "--symbols", directory / (source + ".o")], directory)
        self.execute(exe, directory, expected, stdout)

    def abi(self, directory, opt, crt):
        peer = self.c_object("abi", directory, opt, crt)
        exe = directory / "abi.exe"
        self.command(self.wave("abi", directory, opt, crt, peer, "--emit=ir,obj,bin", "-o", exe), directory)
        self.execute(exe, directory, 0, "native ABI checked\n")

    def abi_edges(self, directory, opt, crt):
        peer = self.c_object("abi_edges", directory, opt, crt)
        exe = directory / "abi-edges.exe"
        self.command(self.wave("abi_edges", directory, opt, crt, peer, "-o", exe), directory)
        self.execute(exe, directory, 0, "")

    def custom_entry(self, directory):
        self.command(self.wave("custom_entry", directory, 0, "dynamic", "--emit=obj"), directory)
        exe = directory / "custom-entry.exe"
        self.command([self.options.wavec, "build", directory / "custom_entry.o",
                      "--target", self.options.target, "-Clinker=" + str(self.linker),
                      "--entry=native_entry", "-Cno-default-libs", "--link=kernel32", "-o", exe], directory)
        self.execute(exe, directory, 23, "")

    def negative(self, directory, crt, mismatch=False):
        exe = directory / "preserved.exe"
        sentinel = b"previous output must survive a failed link\n"
        exe.write_bytes(sentinel)
        if mismatch:
            objects = [self.c_object("crt_mismatch", directory, 0, "dynamic"),
                       self.c_object("crt_static", directory, 0, "static")]
            command = self.wave("implicit", directory, 0, crt, *objects, "-o", exe)
            diagnostic = "RuntimeLibrary"
        else:
            command = self.wave("missing_helper", directory, 0, crt, "-o", exe)
            diagnostic = "wave_missing_runtime_helper"
        self.command(command, directory, expected=None, diagnostic=diagnostic)
        if exe.read_bytes() != sentinel:
            raise AssertionError("failed link replaced the previous output")

    def unsupported_mode(self, directory):
        self.command(self.wave("implicit", directory, 0, "static", "--shared", "-o", directory / "invalid.dll"),
                     directory, expected=None, diagnostic="cannot combine --shared and --static")

    def stack(self, directory, opt, crt):
        peer = self.c_object("stack", directory, opt, crt)
        exe = directory / "stack.exe"
        self.command(self.wave("stack", directory, opt, crt, peer, "--emit=ir,obj,bin", "-o", exe), directory)
        self.command([self.readobj, "--unwind", "--symbols", directory / "stack.o"], directory,
                     diagnostic="RuntimeFunction")
        self.execute(exe, directory, 0, "native stack checked\n")

    def dll(self, directory, opt, crt):
        # Wave shared mode uses the dynamic CRT. Both peer CRT modes retain
        # allocation ownership within the module that created each buffer.
        dll = directory / "wave-library.dll"
        exports = ["-Clink-arg=/EXPORT:" + name for name in ("wave_packet", "wave_allocate", "wave_release")]
        self.command(self.wave("dll", directory, opt, "dynamic", "--shared", *exports, "-o", dll), directory)
        self.command([self.readobj, "--coff-exports", dll], directory, diagnostic="wave_packet")
        for imported in (False, True):
            exe = directory / ("import-consumer.exe" if imported else "loader.exe")
            args = [self.clang, "--target=" + self.options.target, "/nologo", "/TC",
                    "/Od" if opt == 0 else "/O2", "/MT" if crt == "static" else "/MD",
                    self.sources / "dll_host.c", "/Fe" + str(exe), "/link", "/INCREMENTAL:NO"]
            if imported:
                args[4:4] = ["/DIMPORT_CONSUMER"]
                args.append(directory / "wave-library.lib")
            self.command(args, directory)
            self.execute(exe, directory, 0, "native DLL host checked\n")
        self.command([self.clang, "--target=" + self.options.target, "/nologo", "/TC", "/LD",
                      "/Od" if opt == 0 else "/O2", "/MT" if crt == "static" else "/MD",
                      self.sources / "dll_peer.c", "/Fe" + str(directory / "peer.dll"),
                      "/link", "/IMPLIB:" + str(directory / "peer.lib")], directory)
        exe = directory / "consumer.exe"
        self.command(self.wave("dll_consumer", directory, opt, crt, directory / "peer.lib", "-o", exe), directory)
        self.execute(exe, directory, 0, "native DLL consumer checked\n")

    def response(self, directory, linker):
        # Stay below CreateProcess's input limit while exceeding the driver's
        # conservative link-argument budget, using distinct real object paths.
        objects_dir = directory / ("한글 object directory " + " ".join(["nested"] * 8))
        objects_dir.mkdir()
        source = objects_dir / "empty.c"
        source.write_text("typedef int no_external_symbols;\n", encoding="utf-8")
        obj = objects_dir / "empty.obj"
        self.command([self.clang, "/nologo", "/c", source, "/Fo" + str(obj)], directory)
        objects = []
        for i in range(64):
            copy = objects_dir / f"translation-unit-{i:03}.obj"
            shutil.copyfile(obj, copy)
            objects.append(copy)
        exe = directory / "long-paths.exe"
        command = self.wave("implicit", directory, 0, "dynamic", *objects, "-o", exe)
        command = ["-Clinker=" + str(linker) if str(arg).startswith("-Clinker=") else arg
                   for arg in command]
        self.command(command, directory)
        self.execute(exe, directory, 0, "native implicit main\n")
        before = exe.read_bytes()
        self.command([*command, "--link=wave_deliberately_missing"], directory,
                     expected=None, diagnostic="wave_deliberately_missing")
        if exe.read_bytes() != before or list(directory.glob(".wave-output-*.tmp")):
            raise AssertionError("response-file link failure changed output or leaked temporary files")

    def run(self):
        try:
            require_native(self.options.target, self.options.wavec, self.clang)
            self.command([self.clang, "--version"], self.output)
            self.command([self.options.wavec, "-V"], self.output)
            for opt in (0, 2):
                for crt in ("dynamic", "static"):
                    for source, expected, stdout in (
                        ("implicit", 0, "native implicit main\n"),
                        ("exit", 37, "native exit 37\n"),
                        ("helpers", 0, "native helpers checked\n"),
                        ("wide_numeric", 0, "native wide numeric checked\n"),
                    ):
                        self.case(f"{source}-O{opt}-{crt}",
                                  lambda d, s=source, o=opt, c=crt, e=expected, out=stdout:
                                  self.hosted(d, s, o, c, e, out))
                    self.case(f"abi-O{opt}-{crt}", lambda d, o=opt, c=crt: self.abi(d, o, c))
                    self.case(f"abi-edges-O{opt}-{crt}", lambda d, o=opt, c=crt: self.abi_edges(d, o, c))
                    self.case(f"stack-O{opt}-{crt}", lambda d, o=opt, c=crt: self.stack(d, o, c))
                    self.case(f"dll-O{opt}-{crt}", lambda d, o=opt, c=crt: self.dll(d, o, c))
            self.case("response-lld", lambda d: self.response(d, self.linker))
            vc = Path(os.environ["VCToolsInstallDir"]) / "bin"
            arch = "arm64" if self.options.target.startswith("aarch64-") else "x64"
            vc_linker = vc / ("Hostarm64" if arch == "arm64" else "Hostx64") / arch / "link.exe"
            if not vc_linker.is_file():
                raise RuntimeError(f"native MSVC link.exe is required: {vc_linker}")
            self.case("response-msvc", lambda d: self.response(d, vc_linker))
            self.case("custom-entry", self.custom_entry)
            for crt in ("dynamic", "static"):
                self.case(f"missing-helper-{crt}", lambda d, c=crt: self.negative(d, c))
            self.case("mismatched-crt", lambda d: self.negative(d, "static", True))
            self.case("unsupported-static-dll", self.unsupported_mode)
        except (OSError, ValueError, RuntimeError, AssertionError) as error:
            self.failures.append({"case": "prerequisites", "error": str(error)})
        (self.output / "result.json").write_text(json.dumps({
            "target": self.options.target, "failures": self.failures,
            "passed": not self.failures, "commands": len(self.commands),
        }, indent=2), encoding="utf-8")
        if self.failures:
            raise RuntimeError(f"{len(self.failures)} native MSVC checks failed; see {self.output}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--wavec", type=lambda p: Path(p).resolve(), required=True)
    parser.add_argument("--llvm-bin", type=lambda p: Path(p).resolve(), required=True)
    parser.add_argument("--target", choices=TARGETS, required=True)
    parser.add_argument("--output", type=Path, required=True)
    Audit(parser.parse_args()).run()


if __name__ == "__main__":
    main()
