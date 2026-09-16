# SPDX-License-Identifier: MPL-2.0
"""Fail-closed behavior of the native MSVC execution gate."""
import argparse
import json
from pathlib import Path
import struct
import sys
import tempfile
import unittest

from tools.check_msvc_native import Audit, pe_machine


class NativeGateTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="wave-msvc-gate-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def audit(self):
        return Audit(argparse.Namespace(
            output=self.root / "audit", wavec=self.root / "missing-wavec.exe",
            llvm_bin=self.root / "missing-llvm", target="aarch64-pc-windows-msvc"))

    def test_unavailable_native_environment_fails_and_preserves_sources(self):
        audit = self.audit()
        with self.assertRaisesRegex(RuntimeError, "native MSVC checks failed"):
            audit.run()
        result = json.loads((audit.output / "result.json").read_text())
        self.assertFalse(result["passed"])
        self.assertEqual(result["failures"][0]["case"], "prerequisites")
        self.assertTrue((audit.output / "sources/abi.c").is_file())
        self.assertTrue((audit.output / "sources/abi.wave").is_file())

    def test_child_exit_and_diagnostic_are_preserved_on_failure(self):
        audit = self.audit()
        with self.assertRaisesRegex(AssertionError, "expected exit 0, got 37"):
            audit.command([sys.executable, "-c", "import sys; print('fixture failure'); sys.exit(37)"], self.root)
        log = json.loads((audit.output / "commands.json").read_text())[-1]
        self.assertEqual(log["exit"], 37)
        self.assertEqual(log["stdout"].strip(), "fixture failure")

    def test_unrelated_negative_failure_does_not_count_as_expected_diagnostic(self):
        audit = self.audit()
        with self.assertRaisesRegex(AssertionError, "missing diagnostic"):
            audit.command([sys.executable, "-c", "raise SystemExit(1)"], self.root,
                          expected=None, diagnostic="wave_missing_runtime_helper")
        with self.assertRaisesRegex(AssertionError, "unexpectedly succeeded"):
            audit.command([sys.executable, "-c", "pass"], self.root, expected=None)

    def test_pe_machine_requires_an_image_and_valid_header_offset(self):
        image = self.root / "arm64.exe"
        data = bytearray(134)
        data[:2] = b"MZ"
        struct.pack_into("<I", data, 60, 128)
        data[128:132] = b"PE\0\0"
        struct.pack_into("<H", data, 132, 0xAA64)
        image.write_bytes(data)
        self.assertEqual(pe_machine(image), 0xAA64)
        for bad in (data[:133], b"not an executable", data[:60] + b"\xff" * 4 + data[64:]):
            image.write_bytes(bad)
            with self.assertRaises(ValueError):
                pe_machine(image)


if __name__ == "__main__":
    unittest.main()
