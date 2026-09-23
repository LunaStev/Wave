# SPDX-License-Identifier: MPL-2.0
"""Portable checks for the native diagnostic command orchestration."""
import argparse
import contextlib
import io
from pathlib import Path
import tempfile
import subprocess
import unittest
from unittest.mock import patch

from tools import diagnose_windows_arm64 as diagnostic


class DiagnosticTests(unittest.TestCase):
    def diagnose(self, failures=()):
        calls = []

        def probe(label, command, *args, **kwargs):
            calls.append((label, list(map(str, command))))
            return int(label in failures)

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            options = argparse.Namespace(wavec=root / "wavec.exe", llvm_bin=root / "llvm", output=root)
            with patch.object(diagnostic, "probe", side_effect=probe), \
                 patch.object(diagnostic.shutil, "copytree"), contextlib.redirect_stdout(io.StringIO()):
                status = diagnostic.diagnose(options)
        return status, calls

    def test_supported_matrix_only_contains_msvc_ir_and_objects(self):
        status, calls = self.diagnose()
        self.assertEqual(status, 0)
        builds = [(label, command) for label, command in calls if "build" in command]
        self.assertEqual([label for label, _ in builds], [
            "minimal-msvc-ir", "minimal-msvc-obj", "test1-msvc-ir", "test1-msvc-obj"])
        for _, command in builds:
            self.assertIn("--target=aarch64-pc-windows-msvc", command)
            self.assertFalse(any("windows-gnu" in arg for arg in command))
        self.assertFalse(any(label.endswith("-debugger") for label, _ in calls))

    def test_real_msvc_failure_still_fails_and_collects_debugger_evidence(self):
        status, calls = self.diagnose({"minimal-msvc-ir"})
        self.assertEqual(status, 1)
        labels = [label for label, _ in calls]
        self.assertIn("minimal-msvc-ir-debugger", labels)
        self.assertNotIn("minimal-msvc-obj", labels)
        self.assertIn("test1-msvc-obj", labels)

    def test_required_probe_failures_survive_successful_compilation(self):
        for label in ("compiler-version", "default-target", "llvm-version"):
            with self.subTest(label=label):
                status, calls = self.diagnose({label})
                self.assertEqual(status, 1)
                self.assertIn("test1-msvc-obj", [name for name, _ in calls])

    def test_optional_debugger_failure_does_not_hide_compiler_failures(self):
        for failures, expected in [({"debugger-version"}, 0),
                                   ({"debugger-version", "minimal-msvc-ir"}, 1)]:
            with self.subTest(failures=failures):
                status, calls = self.diagnose(failures)
                self.assertEqual(status, expected)
                self.assertFalse(any(label.endswith("-debugger") for label, _ in calls))

    def test_required_probe_launch_failure_and_timeout_reach_final_status(self):
        for error in (OSError("probe unavailable"),
                      subprocess.TimeoutExpired("wavec", 30, output=b"partial probe")):
            with self.subTest(error=error), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                options = argparse.Namespace(wavec=root / "wavec.exe", llvm_bin=root / "llvm", output=root)

                def execute(command, **kwargs):
                    if command[1:] == ["-V"]:
                        raise error
                    return subprocess.CompletedProcess(command, 0, "ok", "")

                with patch.object(diagnostic, "run_process", side_effect=execute), \
                     patch.object(diagnostic.shutil, "copytree"), contextlib.redirect_stdout(io.StringIO()):
                    self.assertEqual(diagnostic.diagnose(options), 1)
                log = (root / "compiler-version.log").read_text()
                self.assertIn("compiler-version", log)
                self.assertIn("timed out" if isinstance(error, subprocess.TimeoutExpired)
                              else "probe unavailable", log)
                self.assertTrue((root / "test1-msvc-obj.log").exists())


if __name__ == "__main__":
    unittest.main()
