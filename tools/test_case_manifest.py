# This file is part of the Wave language project.
# Copyright (c) 2024–2026 Wave Foundation
# Copyright (c) 2024–2026 LunaStev and contributors
#
# This Source Code Form is subject to the terms of the
# Mozilla Public License, v. 2.0.
# If a copy of the MPL was not distributed with this file,
# You can obtain one at https://mozilla.org/MPL/2.0/.
#
# SPDX-License-Identifier: MPL-2.0
# AI TRAINING NOTICE: Prohibited without prior written permission. No use for machine learning or generative AI training, fine-tuning, distillation, embedding, or dataset creation.

import unittest

from tools.case_manifest import (
    MIN_CASES_PER_SUITE,
    CaseManifestError,
    load_case_manifest,
)


class CaseManifestTests(unittest.TestCase):
    def setUp(self):
        self.manifest = load_case_manifest()

    def test_native_hosts_select_shared_and_platform_suites(self):
        linux = self.manifest.native_target("linux", "amd64")
        self.assertEqual(
            linux.suites,
            ("shared", "shared/amd64", "linux/amd64"),
        )

        macos = self.manifest.native_target("macos", "arm64")
        self.assertEqual(
            macos.suites,
            ("shared", "shared/arm64", "macos/arm64"),
        )

    def test_disabled_roadmap_targets_do_not_enter_ci(self):
        matrices = self.manifest.github_matrices()
        ci_ids = {
            entry["id"]
            for matrix in matrices.values()
            for entry in matrix["include"]
        }
        for target_id in (
            "openbsd-amd64",
            "netbsd-arm64",
            "fuchsia-riscv64",
            "windows-riscv64",
            "wasi-wasm64",
        ):
            target = self.manifest.target(target_id)
            self.assertFalse(target.enabled)
            self.assertFalse(target.ci)
            self.assertNotIn(target_id, ci_ids)

    def test_named_roadmap_operating_systems_reserve_all_architectures(self):
        for os_name in ("openbsd", "netbsd", "fuchsia", "android", "ios"):
            for arch in ("amd64", "arm64", "riscv64"):
                with self.subTest(os=os_name, arch=arch):
                    target = self.manifest.target(f"{os_name}-{arch}")
                    self.assertEqual(target.suite, f"{os_name}/{arch}")
                    self.assertEqual(target.status, "planned")
                    self.assertFalse(target.enabled)
                    self.assertFalse(target.ci)

    def test_wasm64_is_enabled_without_claiming_a_wasi64_abi(self):
        bare = self.manifest.target("wasm64-unknown")
        self.assertTrue(bare.enabled)
        self.assertTrue(bare.ci)
        self.assertEqual(bare.target, "wasm64-unknown-unknown")
        self.assertEqual(
            bare.suites,
            ("shared", "shared/wasm64", "wasm/wasm64"),
        )

        wasi = self.manifest.target("wasi-wasm64")
        self.assertFalse(wasi.enabled)
        self.assertFalse(wasi.ci)

    def test_named_processor_roadmap_cells_are_planned_only(self):
        cells = (
            ("linux", "loong64"),
            ("linux", "rhea"),
            ("linux", "shakti"),
            ("linux", "xiangshan"),
            ("linux", "t-head"),
            ("freestanding", "k-riscv"),
        )
        for os_name, arch in cells:
            with self.subTest(os=os_name, arch=arch):
                target = self.manifest.target(f"{os_name}-{arch}")
                self.assertEqual(target.status, "planned")
                self.assertFalse(target.enabled)
                self.assertFalse(target.ci)

    def test_unknown_target_is_rejected(self):
        with self.assertRaises(CaseManifestError):
            self.manifest.target("missing-target")

    def test_every_configured_suite_has_the_baseline_case_count(self):
        counts = {}
        for source in self.manifest.sources():
            suite = source.rsplit("/", 1)[0]
            if source.endswith("/main.wave"):
                suite = suite.rsplit("/", 1)[0]
            counts[suite] = counts.get(suite, 0) + 1

        for target in self.manifest.targets:
            self.assertGreaterEqual(counts[target.suite], MIN_CASES_PER_SUITE)


if __name__ == "__main__":
    unittest.main()
