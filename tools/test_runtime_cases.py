# SPDX-License-Identifier: MPL-2.0
"""Runtime reports distinguish execution from compilation without real SDKs."""
import argparse
import contextlib
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from tools import run_runtime_cases as runner
from tools.process_tree import run_process


class RuntimeReportTests(unittest.TestCase):
    def run_suite(self, executor="wasm", outcomes=(), missing_output=False, sources=None):
        calls = []
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            selected = ["shared/test1.wave", "shared/test2/main.wave"]
            for name in selected:
                path = root / "tests/cases" / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("fun main() {}")
            options = argparse.Namespace(
                wavec=Path(sys.executable), target_id="test-target", sources=selected if sources is None else sources,
                report_json=root / "report.json", qemu="fake-qemu", sysroot=root,
                build_timeout=3, timeout=3)
            target = SimpleNamespace(enabled=True, executor=executor, target="test-triple", suites=("shared",))

            def process(command, **kwargs):
                index = len(calls)
                calls.append(command)
                outcome = outcomes[index] if index < len(outcomes) else 0
                if isinstance(outcome, BaseException):
                    raise outcome
                if "-o" in command and outcome == 0 and not missing_output:
                    Path(command[command.index("-o") + 1]).touch()
                # A real short-lived subprocess supplies stdout, stderr, and exit status.
                return run_process([sys.executable, "-c",
                    "import sys; print('runtime output'); "
                    "print('env.printf unavailable ' + 'x' * 5000, file=sys.stderr); "
                    f"sys.exit({outcome})"], **kwargs)

            with patch.object(runner, "ROOT", root), \
                 patch.object(runner, "load_case_manifest", return_value=SimpleNamespace(target=lambda _: target)), \
                 patch.object(runner, "run_process", side_effect=process), \
                 contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                status = runner.execute(options)
            return status, json.loads(options.report_json.read_text()), calls

    def test_wasm_success_is_explicitly_runtime_and_preserves_selection(self):
        status, report, calls = self.run_suite()
        self.assertEqual(status, 0)
        self.assertEqual(report["phase"], "runtime")
        self.assertEqual(report["selection"], {
            "mode": "target", "id": "test-target", "target": "test-triple",
            "executor": "wasm", "suites": ["shared"]})
        self.assertEqual(report["summary"]["pass"], 2)
        self.assertTrue(all(command[1] == "run" for command in calls))
        self.assertEqual(report["tests"][0]["commands"][0]["phase"], "build-and-run")

    def test_host_import_failure_is_reported_and_later_case_still_runs(self):
        status, report, _ = self.run_suite(outcomes=[7])
        self.assertEqual(status, 1)
        self.assertEqual([row["status"] for row in report["tests"]], ["fail", "pass"])
        command = report["tests"][0]["commands"][0]
        self.assertEqual(command["actual_exit"], 7)
        self.assertEqual(command["expected_exit"], 0)
        self.assertIn("env.printf", command["stderr"])
        self.assertEqual(len(command["stderr"]), 4096)
        self.assertTrue(command["stderr_truncated"])

    def test_qemu_build_and_execution_have_separate_status_and_use_exact_output(self):
        status, report, calls = self.run_suite(executor="qemu")
        self.assertEqual(status, 0)
        for row, build, run in zip(report["tests"], calls[::2], calls[1::2]):
            self.assertEqual([cmd["phase"] for cmd in row["commands"]], ["build", "run"])
            self.assertEqual(build[build.index("-o") + 1], run[-1])
            self.assertEqual(run[0], "fake-qemu")
        self.assertTrue(any(arg.endswith("main.wave") for arg in calls[2]))

    def test_qemu_compile_failure_or_missing_artifact_never_counts_as_execution(self):
        for missing, outcomes in [(False, [7]), (True, [])]:
            with self.subTest(missing=missing):
                status, report, _ = self.run_suite(executor="qemu", outcomes=outcomes, missing_output=missing)
                self.assertEqual(status, 1)
                first = report["tests"][0]
                self.assertEqual(first["status"], "fail")
                self.assertEqual(len(first["commands"]), 1)
                self.assertEqual(first["commands"][0]["phase"], "build")

    def test_successful_qemu_build_cannot_hide_program_failure(self):
        status, report, _ = self.run_suite(executor="qemu", outcomes=[0, 7])
        self.assertEqual(status, 1)
        self.assertEqual(report["tests"][0]["status"], "fail")
        build, run = report["tests"][0]["commands"]
        self.assertEqual(build["actual_exit"], 0)
        self.assertEqual(run["actual_exit"], 7)
        self.assertEqual(run["phase"], "run")
        self.assertEqual(report["tests"][1]["status"], "pass")

    def test_timeout_and_launch_failure_preserve_reason_without_passing(self):
        for error, expected in [(subprocess.TimeoutExpired("host", 6, output=b"partial"), "timeout"),
                                (OSError("host unavailable"), "fail")]:
            with self.subTest(error=error):
                status, report, _ = self.run_suite(outcomes=[error])
                self.assertEqual(status, 1)
                self.assertEqual(report["tests"][0]["status"], expected)
                self.assertEqual(report["tests"][1]["status"], "pass")
                self.assertIn("reason", report["tests"][0])
                if expected == "timeout":
                    self.assertEqual(report["tests"][0]["commands"][0]["output"], "partial")

    def test_interruption_keeps_pending_cases_not_run(self):
        status, report, calls = self.run_suite(outcomes=[KeyboardInterrupt()])
        self.assertEqual(status, 130)
        self.assertEqual([row["status"] for row in report["tests"]], ["interrupted", "not_run"])
        self.assertEqual(report["summary"]["pass"], 0)
        self.assertEqual(len(calls), 1)

    def test_empty_unsafe_or_duplicate_selection_fails_before_execution(self):
        for sources in ([], ["../outside.wave"], ["shared/test1.wave"] * 2):
            with self.subTest(sources=sources):
                status, report, calls = self.run_suite(sources=sources)
                self.assertEqual(status, 2)
                self.assertIn("error", report)
                self.assertFalse(calls)

    def test_timeout_rejects_unbounded_values(self):
        for value in ("nan", "inf", "0", "-1"):
            with self.subTest(value=value), self.assertRaises(argparse.ArgumentTypeError):
                runner.positive_seconds(value)


if __name__ == "__main__":
    unittest.main()
