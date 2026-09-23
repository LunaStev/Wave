# SPDX-License-Identifier: MPL-2.0
"""Exercise the actual Bash selection blocks without LLVM or target runners."""
from pathlib import Path
import re
import shutil
import subprocess
import unittest


@unittest.skipUnless(shutil.which("bash"), "Bash is required for workflow shell checks")
class RuntimeSelectionTests(unittest.TestCase):
    def blocks(self):
        workflow = (Path(__file__).resolve().parent.parent / ".github/workflows/cases.yml").read_text()
        blocks = re.findall(r'(?m)^          case_sources="\$\(\n.*?^          mapfile[^\n]+',
                            workflow, flags=re.DOTALL)
        self.assertEqual(len(blocks), 3)
        return [block.replace("${{ matrix.id }}", "test-target") for block in blocks]

    def run_selection(self, block, selector, runtime='printf "RUN:%s\\n" "$source"'):
        script = ('set -euo pipefail\npython3() { ' + selector + '; }\n' + block +
                  '\nfor source in "${cases[@]}"; do ' + runtime + '; done\n')
        return subprocess.run(["bash", "-c", script], capture_output=True, text=True, timeout=5)

    def test_failed_or_empty_selector_never_executes_partial_results(self):
        for block in self.blocks():
            for selector in ("return 2", "printf 'partial.wave\\n'; return 2", "return 0"):
                with self.subTest(selector=selector, block=block):
                    result = self.run_selection(block, selector)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertNotIn("RUN:", result.stdout)

    def test_success_preserves_paths_and_order_and_runtime_failure(self):
        for block in self.blocks():
            result = self.run_selection(block, "printf '%s\\n' 'shared/a b.wave' 'shared/c.wave'")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout.splitlines(), ["RUN:shared/a b.wave", "RUN:shared/c.wave"])
            failed = self.run_selection(block, "printf 'shared/a.wave\\n'", "exit 7")
            self.assertEqual(failed.returncode, 7)


if __name__ == "__main__":
    unittest.main()
