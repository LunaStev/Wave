# This file is part of the Wave language project.
# SPDX-License-Identifier: MPL-2.0
"""Execute the actual publish step with a fake gh; never contact GitHub."""
import hashlib
import sys
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import textwrap
import unittest


@unittest.skipUnless(os.name != "nt" and shutil.which("bash"), "requires POSIX bash")
class ReleasePublishTests(unittest.TestCase):
    def run_publish(self, remote, status=0):
        workflow = Path(__file__).resolve().parents[1] / ".github/workflows/release.yml"
        block = workflow.read_text().split("      - name: Create GitHub release\n", 1)[1]
        script = textwrap.dedent(block.split("        run: |\n", 1)[1])
        script = script.replace("${{ inputs.draft }}", "true").replace("${{ inputs.prerelease }}", "true")
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            gh = root / "gh"
            gh.write_text('#!/bin/bash\nprintf "%s\\n" "$*" >> "$AUDIT_GH_LOG"\n'
                          'if [[ "$1" == api ]]; then\n'
                          ' printf "%s\\n" "$AUDIT_REMOTE_SHA"\n exit "$AUDIT_API_STATUS"\nfi\n')
            gh.chmod(0o755)
            env = dict(os.environ, PATH=str(root) + os.pathsep + os.environ.get("PATH", ""),
                       AUDIT_GH_LOG=str(root / "calls"), AUDIT_REMOTE_SHA=remote,
                       AUDIT_API_STATUS=str(status), GITHUB_SHA="a" * 40,
                       GITHUB_REPOSITORY="wavefnd/Wave", RELEASE_VERSION="0.2.1-pre-beta")
            result = subprocess.run(["bash", "-c", script], cwd=root, env=env, capture_output=True, text=True, timeout=10)
            return result, (root / "calls").read_text().splitlines()

    def test_matching_master_publishes_after_verification(self):
        result, calls = self.run_publish("a" * 40)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(calls[0], "api repos/wavefnd/Wave/git/ref/heads/master --jq .object.sha")
        self.assertTrue(calls[1].startswith("release create "))
        self.assertIn("--target " + "a" * 40, calls[1])

    def test_changed_missing_malformed_or_unavailable_master_never_publishes(self):
        for remote, status in [("b" * 40, 0), ("", 0), ("not-a-sha", 0), ("a" * 40, 1)]:
            with self.subTest(remote=remote, status=status):
                result, calls = self.run_publish(remote, status)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(len(calls), 1)


class ReleaseAssetTests(unittest.TestCase):
    def setUp(self):
        from tools import check_release_assets
        self.validator = check_release_assets
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.version = "0.2.1-pre-beta"
        self.names = []
        for target in check_release_assets.ARCHIVE_TARGETS:
            name = f"wave-v{self.version}-{target}" + (".zip" if "windows" in target else ".tar.gz")
            self.names.append(name)
            (self.root / name).write_bytes(name.encode())
            digest = hashlib.sha256(name.encode()).hexdigest()
            (self.root / (name + ".sha256")).write_text(f"{digest}  {name}\n")

    def verify(self):
        return self.validator.verify(self.root, self.version)

    def test_complete_set_writes_exactly_one_verified_record_per_archive(self):
        # Accept the binary marker emitted by checksum utilities as well.
        sidecar = self.root / (self.names[0] + ".sha256")
        sidecar.write_text(sidecar.read_text().replace("  ", " *"))
        self.assertEqual(self.verify(), 8)
        lines = (self.root / "SHA256SUMS").read_text().splitlines()
        self.assertEqual([line[66:] for line in lines], sorted(self.names))
        for line in lines:
            self.assertEqual(line[:64], hashlib.sha256((self.root / line[66:]).read_bytes()).hexdigest())

    def test_empty_duplicate_malformed_wrong_name_and_unsafe_records_fail(self):
        name = self.names[0]
        sidecar = self.root / (name + ".sha256")
        valid = sidecar.read_text()
        for invalid in ["", "\n", valid + valid, valid + "\n", "not a checksum\n",
                        valid.replace(name, self.names[1]), valid.replace(name, "../" + name),
                        valid.replace(name, str((self.root / name).resolve())), "0" * 64 + f"  {name}\n"]:
            with self.subTest(record=invalid):
                (self.root / "SHA256SUMS").write_text("previous manifest")
                sidecar.write_text(invalid)
                with self.assertRaises(ValueError):
                    self.verify()
                self.assertEqual((self.root / "SHA256SUMS").read_text(), "previous manifest")
        sidecar.write_text(valid)
        (self.root / name).write_bytes(b"changed archive")
        with self.assertRaisesRegex(ValueError, "checksum mismatch"):
            self.verify()

    def test_missing_or_unexpected_archives_and_sidecars_fail(self):
        for name in (self.names[0], self.names[0] + ".sha256"):
            path = self.root / name
            contents = path.read_bytes()
            path.unlink()
            with self.assertRaisesRegex(ValueError, "missing="):
                self.verify()
            path.write_bytes(contents)
        for name in ("unexpected.zip", "unexpected.sha256"):
            path = self.root / name
            path.touch()
            with self.assertRaisesRegex(ValueError, "unexpected="):
                self.verify()
            path.unlink()

    def test_cli_failure_does_not_create_a_partial_manifest(self):
        (self.root / (self.names[0] + ".sha256")).write_text("")
        result = subprocess.run([sys.executable, "-m", "tools.check_release_assets", "--directory", str(self.root),
                                 "--version", self.version], cwd=Path(__file__).resolve().parents[1],
                                text=True, capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 1)
        self.assertIn("exactly one checksum record", result.stderr)
        self.assertFalse((self.root / "SHA256SUMS").exists())


if __name__ == "__main__":
    unittest.main()
