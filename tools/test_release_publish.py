# This file is part of the Wave language project.
# SPDX-License-Identifier: MPL-2.0
"""Execute the actual publish step with a fake gh; never contact GitHub."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import textwrap
import unittest


@unittest.skipUnless(shutil.which("bash"), "requires bash")
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


if __name__ == "__main__":
    unittest.main()
