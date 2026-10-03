# SPDX-License-Identifier: MPL-2.0
"""Reject unusable or unpinned prebuilt SDKs without touching an installation."""

import hashlib
import json
from pathlib import Path
import tarfile
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock

from tools.ci import llvm_bundle as bundle


class BundleTests(unittest.TestCase):
    def sdk(self, prefix, target="linux-riscv64"):
        for directory in ("bin", "lib", "include/llvm/Config", "include/llvm-c"):
            (prefix / directory).mkdir(parents=True, exist_ok=True)
        header = bytearray(20)
        header[:6] = b"\x7fELF\x02\x01"
        header[18:20] = bundle.MACHINES[target].to_bytes(2, "little")
        for name in bundle.TOOLS:
            path = prefix / "bin" / name
            path.write_bytes(header)
            path.chmod(0o755)
        for name in ("include/llvm/Config/llvm-config.h", "include/llvm-c/Core.h", "lib/libLLVM.so"):
            (prefix / name).touch()
        (prefix / "bundle.json").write_text(json.dumps(bundle.identity(target)))

    def test_identity_architecture_and_header_failures(self):
        with tempfile.TemporaryDirectory() as folder:
            prefix = Path(folder) / "llvm"
            self.sdk(prefix)
            bundle.check_layout(prefix, "linux-riscv64")
            with self.assertRaisesRegex(ValueError, "identity"):
                bundle.check_layout(prefix, "linux-loong64")
            (prefix / "bin/llc").write_bytes(b"not an ELF")
            with self.assertRaisesRegex(ValueError, "ELF"):
                bundle.check_layout(prefix, "linux-riscv64")
            self.sdk(prefix)
            (prefix / "include/llvm-c/Core.h").unlink()
            with self.assertRaisesRegex(ValueError, "header"):
                bundle.check_layout(prefix, "linux-riscv64")

    def test_rejects_build_tree_paths_and_missing_backends(self):
        with tempfile.TemporaryDirectory() as folder:
            prefix = Path(folder) / "llvm"
            self.sdk(prefix)
            for responses, message in (
                ([bundle.PINS["LLVM_SOURCE_VERSION"], "X86"], "backend"),
                ([bundle.PINS["LLVM_SOURCE_VERSION"], " ".join(bundle.BACKENDS), "/old/build"], "relocatable"),
            ):
                runner = SimpleNamespace(run=Mock(side_effect=responses))
                with self.assertRaisesRegex(ValueError, message):
                    bundle.validate(runner, prefix, "linux-riscv64", "/sysroot")

    def test_pin_validation_precedes_download(self):
        runner = SimpleNamespace(run=Mock())
        entry = dict(bundle.identity("linux-riscv64"), url="https://example.invalid/sdk.tar.xz", sha256="0" * 64)
        for changes in ({"sha256": "unknown"}, {"url": "http://example.invalid/sdk"}, {"llvm_version": "22.1.0"}):
            with self.subTest(changes=changes), tempfile.TemporaryDirectory() as folder:
                with self.assertRaises(ValueError):
                    bundle.fetch_bundle(runner, entry | changes, Path(folder) / "sdk", "linux-riscv64", "/sysroot")
        runner.run.assert_not_called()

    def test_checksum_failure_does_not_install_or_execute(self):
        with tempfile.TemporaryDirectory() as folder:
            destination = Path(folder) / "sdk"
            def run(command, **kwargs):
                self.assertEqual(command[0], "curl")
                Path(command[-1]).write_bytes(b"corrupt archive")
            entry = dict(bundle.identity("linux-riscv64"), url="https://example.invalid/sdk.tar.xz", sha256="0" * 64)
            with self.assertRaisesRegex(ValueError, "checksum"):
                bundle.fetch_bundle(SimpleNamespace(run=run), entry, destination, "linux-riscv64", "/sysroot")
            self.assertFalse(destination.exists())

    def test_archive_cannot_write_outside_staging(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            source = root / "bad.tar.xz"
            with tarfile.open(source, "w:xz") as stream:
                item = tarfile.TarInfo("../../escape")
                stream.addfile(item)
            payload = source.read_bytes()
            def run(command, **kwargs):
                self.assertEqual(command[0], "curl")
                Path(command[-1]).write_bytes(payload)
            entry = dict(bundle.identity("linux-riscv64"), url="https://example.invalid/sdk.tar.xz", sha256=hashlib.sha256(payload).hexdigest())
            with self.assertRaises(tarfile.FilterError):
                bundle.fetch_bundle(SimpleNamespace(run=run), entry, root / "sdk", "linux-riscv64", "/sysroot")
            self.assertFalse((root / "escape").exists())
            self.assertFalse((root / "sdk").exists())


if __name__ == "__main__":
    unittest.main()
