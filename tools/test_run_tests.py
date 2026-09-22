import io
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tools.run_tests import parse_args, resolve_wavec, main
from tools import run_tests as runner
from tools.test_contracts import TestMetadata
from types import SimpleNamespace

class TestRunTestsCLI(unittest.TestCase):
    def test_explicit_compiler_wins_over_stale_default_and_missing_is_fatal(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            stale = root / "target/release/wavec.exe"
            stale.parent.mkdir(parents=True)
            stale.touch()
            selected = root / "msvc-wavec.exe"
            selected.touch()
            self.assertEqual(resolve_wavec(root, selected), selected.resolve())
            with self.assertRaises(ValueError):
                resolve_wavec(root, root / "missing.exe")

    def test_explicit_native_manifest_target_still_executes_cases(self):
        for target_id, triple in [("windows-amd64", "x86_64-pc-windows-msvc"),
                                   ("windows-arm64", "aarch64-pc-windows-msvc")]:
            with self.subTest(target=target_id), \
                 patch.object(runner, "ARGS", SimpleNamespace(suite=[], target_id=target_id)), \
                 patch.object(runner, "WAVEC", "selected-wavec.exe"), \
                 patch.object(runner, "parse_test_metadata", return_value=TestMetadata()):
                runner.configured_target.cache_clear()
                try:
                    self.assertIsNone(runner.manifest_compile_target())
                    self.assertEqual(runner.command_for_test("case", "case.wave"),
                                     ["selected-wavec.exe", "run", "case.wave", "--target", triple])
                finally:
                    runner.configured_target.cache_clear()

    def test_help_shows_usage_and_creates_no_output_dir(self):
        tmp_dir = tempfile.gettempdir()
        before_dirs = set(Path(tmp_dir).glob("wave-test-output-*"))

        stdout = io.StringIO()
        with patch("sys.stdout", stdout):
            with self.assertRaises(SystemExit) as cm:
                parse_args(["--help"])

        self.assertEqual(cm.exception.code, 0)
        self.assertIn("Run Wave end-to-end tests", stdout.getvalue())

        after_dirs = set(Path(tmp_dir).glob("wave-test-output-*"))
        new_dirs = after_dirs - before_dirs
        self.assertEqual(len(new_dirs), 0, f"Unexpected temp dir created: {new_dirs}")

    def test_main_help_creates_no_output_dir(self):
        tmp_dir = tempfile.gettempdir()
        before_dirs = set(Path(tmp_dir).glob("wave-test-output-*"))

        stdout = io.StringIO()
        with patch("sys.stdout", stdout):
            with self.assertRaises(SystemExit) as cm:
                main(["--help"])

        self.assertEqual(cm.exception.code, 0)
        self.assertIn("Run Wave end-to-end tests", stdout.getvalue())

        after_dirs = set(Path(tmp_dir).glob("wave-test-output-*"))
        new_dirs = after_dirs - before_dirs
        self.assertEqual(len(new_dirs), 0, f"Unexpected temp dir created: {new_dirs}")

    def test_missing_compiler_retains_actionable_error(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            empty_root = Path(tmp_dir)
            stdout = io.StringIO()
            with patch("sys.stdout", stdout):
                with self.assertRaises(SystemExit) as cm:
                    resolve_wavec(empty_root)

            self.assertEqual(cm.exception.code, 1)
            self.assertIn(
                "wavec not found. Run `cargo build --release` or `cargo build` first.",
                stdout.getvalue(),
            )

    def test_found_compiler(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            fake_root = Path(tmp_dir)
            fake_wavec = fake_root / "target" / "release" / "wavec"
            fake_wavec.parent.mkdir(parents=True, exist_ok=True)
            fake_wavec.touch()

            result = resolve_wavec(fake_root)
            self.assertEqual(result, fake_wavec)

if __name__ == "__main__":
    unittest.main()
