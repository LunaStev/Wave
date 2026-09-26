# SPDX-License-Identifier: MPL-2.0
"""Failure-path tests for the FreeBSD CI runner; no VM or network required."""
import argparse
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

from tools import check_freebsd_sys as runner


class ReportingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.image = self.root / "image.qcow2"
        self.compiler = self.root / "wavec"
        self.image.touch()
        self.compiler.touch()
        cases = self.root / "tests/cases/freebsd/amd64"
        cases.mkdir(parents=True)
        for name in ["test1.wave", "test2.wave"]:
            (cases / name).write_text("fun main() -> i32 { return 0; }")
        self.args = argparse.Namespace(arch="amd64", image=self.image, compiler=self.compiler,
            firmware=None, kernel=None, out_dir=self.root / "out", clang="clang", linker="ld.lld",
            command_timeout=3, boot_timeout=4, case_timeout=5)

    def execute(self, failures=None, missing=None, boot_error=False):
        report = {"commands": [], "cases": []}
        commands = []
        failures = failures or {}

        class Console:
            def __init__(self, *args):
                pass

            def send(self, *args, **kwargs):
                pass

            def expect(self, pattern, timeout=None):
                if boot_error:
                    raise TimeoutError("boot timeout")
                if pattern.startswith("WAVE-RESULT"):
                    name = pattern.split()[1]
                    if name == missing:
                        raise TimeoutError("missing case result")
                    return re.search(pattern, f"WAVE-RESULT {name} {failures.get(name, 0)}\r")
                return re.search(".*", "OK ")

        process = Mock()
        process.wait.return_value = 0
        process.poll.return_value = None
        with patch.object(runner, "ROOT", self.root), patch.object(runner, "Console", Console), \
             patch.object(runner.Commands, "run", lambda _, *args: commands.append(args)), \
             patch.object(runner.subprocess, "Popen", return_value=process):
            try:
                runner.execute(self.args, report)
                error = None
            except Exception as exc:
                error = exc
        return report, commands, process, error

    def test_all_optimization_results_required_and_checkout_std_selected(self):
        report, commands, _, error = self.execute()
        self.assertIsNone(error)
        self.assertEqual([case["name"] for case in report["cases"]],
            ["test1-O0", "test1-O2", "test2-O0", "test2-O2"])
        self.assertTrue(all(case["status"] == "pass" for case in report["cases"]))
        builds = [cmd for cmd in commands if "build" in cmd]
        self.assertEqual(len(builds), 4)
        self.assertTrue(all(cmd[cmd.index("--std-root") + 1] == self.root / "std" for cmd in builds))

    def test_nonzero_result_fails_but_retains_later_results(self):
        report, _, _, error = self.execute(failures={"test1-O2": 17})
        self.assertIsInstance(error, RuntimeError)
        self.assertEqual([c["status"] for c in report["cases"]], ["pass", "fail", "pass", "pass"])
        self.assertEqual(report["cases"][1]["exit_code"], 17)

    def test_missing_result_never_marks_pending_cases_as_pass(self):
        report, _, process, error = self.execute(missing="test1-O2")
        self.assertIsInstance(error, TimeoutError)
        self.assertEqual([c["status"] for c in report["cases"]], ["pass", "missing_result", "not_run", "not_run"])
        process.terminate.assert_called_once()

    def test_boot_failure_preserves_phase_and_stops_guest(self):
        report, _, process, error = self.execute(boot_error=True)
        self.assertIsInstance(error, TimeoutError)
        self.assertEqual(report["phase"], "guest_boot")
        self.assertTrue(all(c["status"] == "not_run" for c in report["cases"]))
        process.terminate.assert_called_once()

    def test_command_failure_and_timeout_are_recorded(self):
        report = {"commands": []}
        command = runner.Commands(report, self.root / "build.log", 3)
        with self.assertRaises(RuntimeError):
            command.run(sys.executable, "-c", "print('failure evidence'); raise SystemExit(7)")
        self.assertEqual(report["commands"][-1]["exit_code"], 7)
        self.assertIn("failure evidence", (self.root / "build.log").read_text())
        with patch.object(runner.subprocess, "run", side_effect=subprocess.TimeoutExpired("clang", 3)):
            with self.assertRaises(subprocess.TimeoutExpired):
                command.run("clang")
        self.assertEqual(report["commands"][-1]["status"], "timeout")

    def test_missing_image_still_writes_machine_readable_failure(self):
        result = self.root / "report.json"
        with patch.object(sys, "argv", ["runner", "--arch", "amd64", "--image", str(self.root / "missing"),
                "--compiler", str(self.compiler), "--report-json", str(result)]):
            self.assertEqual(runner.main(), 1)
        report = json.loads(result.read_text())
        self.assertEqual(report["status"], "fail")
        self.assertEqual(report["phase"], "validation")
        self.assertIn("File does not exist", report["error"])


if __name__ == "__main__":
    unittest.main()
