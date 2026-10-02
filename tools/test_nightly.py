# SPDX-License-Identifier: MPL-2.0
"""Nightly tests use an in-memory GitHub; no live release writes."""
import base64
import copy
import hashlib
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from urllib.error import HTTPError, URLError

from tools.ci import nightly, package, release, targets
from tools.check_release_assets import ARCHIVE_TARGETS

A, B, C, OTHER = (c * 40 for c in "abcd")


class FakeGitHub:
    def __init__(self):
        self.release = None
        self.tag = None
        self.files = {}
        self.calls = []
        self.fail = None
        self.upload_failure = False
        self.corrupt = False
        self.run = dict(
            repository={"full_name": nightly.REPO},
            head_repository={"full_name": nightly.REPO},
            event="push",
            head_branch="master",
            path=".github/workflows/ci.yml",
            status="completed",
            conclusion="success",
            head_sha=A,
            run_attempt=1,
        )
        self.jobs = [dict(status="completed", conclusion="success")]

    def api(self, endpoint, *, method="GET", data=None, optional=False):
        self.calls.append((method, endpoint, copy.deepcopy(data)))
        fault = self.fail and self.fail[:2] == (method, endpoint)
        if fault and self.fail[2] == "before":
            self.fail = None
            raise RuntimeError("injected API failure")
        if endpoint == "git/ref/heads/master":
            value = {"object": {"sha": C}}
        elif endpoint.startswith("compare/"):
            before, after = endpoint.removeprefix("compare/").split("...")
            ordered = [A, B, C]
            ahead = (
                before in ordered
                and after in ordered
                and ordered.index(before) < ordered.index(after)
            )
            value = {
                "status": "ahead" if ahead else "behind",
                "merge_base_commit": {"sha": before},
            }
        elif "/jobs?" in endpoint:
            value = {"jobs": self.jobs}
        elif endpoint.startswith("actions/runs/"):
            value = self.run
        elif endpoint.startswith("contents/Cargo.toml"):
            value = {
                "content": base64.b64encode(
                    b'[package]\nversion="0.2.1-pre-beta-dev"'
                ).decode()
            }
        elif endpoint == "git/ref/tags/nightly":
            value = (
                {"object": {"sha": self.tag, "type": "commit"}} if self.tag else None
            )
        elif endpoint in ("git/refs", "git/refs/tags/nightly"):
            self.tag = data["sha"]
            value = {}
        elif endpoint == "releases/tags/nightly" or (
            endpoint == "releases/1" and method == "GET"
        ):
            value = self.release
        elif endpoint.startswith("releases?"):
            value = [self.release] if self.release else []
        elif endpoint == "releases" and method == "POST":
            self.release = dict(data, id=1)
            value = self.release
        elif endpoint == "releases/1" and method == "PATCH":
            self.release.update(data)
            value = self.release
        elif endpoint.startswith("releases/assets/") and method == "DELETE":
            name = next(
                name
                for name, a in self.files.items()
                if a["id"] == int(endpoint.split("/")[-1])
            )
            del self.files[name]
            value = None
        else:
            raise AssertionError((method, endpoint))
        if fault:
            mode = self.fail[2]
            self.fail = None
            if mode == "kill":
                raise SystemExit("simulated process death")
            raise RuntimeError("response lost after commit")
        return copy.deepcopy(value)

    def assets(self, release_id):
        return copy.deepcopy(list(self.files.values()))

    def upload(self, path):
        self.calls.append(("UPLOAD", path.name, None))
        if self.upload_failure:
            raise RuntimeError("upload failed")
        record = nightly.file_record(path)
        self.files[path.name] = dict(
            id=max([a["id"] for a in self.files.values()] + [0]) + 1,
            name=path.name,
            size=record["size"],
            state="uploaded",
            digest="sha256:" + ("0" * 64 if self.corrupt else record["sha256"]),
        )


class NightlyTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.github = FakeGitHub()
        self.version = "0.2.1-pre-beta-dev"
        self.counter = 0

    def generation(self, source=A):
        self.counter += 1
        folder = self.root / str(self.counter)
        folder.mkdir()
        for target in ARCHIVE_TARGETS:
            name = f"wave-v{self.version}-{target}" + (
                ".zip" if "windows" in target else ".tar.gz"
            )
            path = folder / name
            path.write_bytes((target + source).encode())
            digest = hashlib.sha256(path.read_bytes()).hexdigest()
            (folder / (name + ".sha256")).write_text(f"{digest}  {name}\n")
            metadata = dict(
                schema_version=1,
                compiler_version=self.version,
                source_sha=source,
                std_compatibility_revision=4,
                target=target,
                archive=name,
                sha256=digest,
                **package.artifact_contract(target),
            )
            (folder / (name + ".metadata.json")).write_text(json.dumps(metadata))
        output = folder / "staged"
        state = nightly.stage(
            folder,
            output,
            dict(source_sha=source, compiler_version=self.version, ci_run_id=10),
            20 + self.counter,
            1,
            4,
        )
        return output, state

    def seed(self):
        directory, state = self.generation()
        self.assertEqual(nightly.promote(self.github, directory, state), "published")
        self.github.calls.clear()
        return state

    def test_eligibility_checks_repository_event_branch_run_and_ancestry(self):
        self.assertEqual(nightly.eligible(self.github, 10)["source_sha"], A)
        for key, value in [
            ("repository", {"full_name": "fork/Wave"}),
            ("head_repository", {"full_name": "fork/Wave"}),
            ("event", "pull_request"),
            ("head_branch", "feature"),
            ("path", ".github/workflows/other.yml"),
            ("status", "in_progress"),
            ("conclusion", "failure"),
            ("conclusion", "cancelled"),
            ("head_sha", OTHER),
        ]:
            with self.subTest(key=key, value=value):
                original = self.github.run[key]
                self.github.run[key] = value
                with self.assertRaises(ValueError):
                    nightly.eligible(self.github, 10)
                self.github.run[key] = original
        for jobs in [
            [],
            [{"status": "completed", "conclusion": "skipped"}],
            [{"status": "completed", "conclusion": "failure"}],
        ]:
            self.github.jobs = jobs
            with self.assertRaises(ValueError):
                nightly.eligible(self.github, 10)
        for value in [None, "../10", "0", "-1"]:
            with self.assertRaises(ValueError):
                nightly.eligible(self.github, value)

    def test_all_nine_packages_have_generation_specific_matching_sidecars(self):
        directory, state = self.generation()
        archives = list(directory.glob("*.zip")) + list(directory.glob("*.tar.gz"))
        self.assertEqual(len(archives), 9)
        self.assertEqual(len(state["assets"]), 29)
        for path in archives:
            self.assertIn(A, path.name)
            metadata = json.loads(
                (directory / (path.name + ".metadata.json")).read_text()
            )
            self.assertEqual(metadata["archive"], path.name)
            self.assertEqual(metadata["compiler_version"], self.version)
            self.assertEqual(metadata["source_sha"], A)
            self.assertIn(path.name, (directory / (path.name + ".sha256")).read_text())
        self.assertEqual(
            {t.archive_target for t in targets.TARGETS.values() if t.distribution},
            set(ARCHIVE_TARGETS),
        )

    def test_bad_package_sets_fail_before_any_remote_write(self):
        for bad in ("missing", "checksum", "metadata", "revision"):
            with self.subTest(bad=bad):
                directory, _ = self.generation()
                source = directory.parent
                archive = next(source.glob("*.tar.gz"))
                if bad == "missing":
                    archive.unlink()
                elif bad == "checksum":
                    archive.write_bytes(b"changed")
                elif bad == "metadata":
                    (source / (archive.name + ".metadata.json")).write_text("{}")
                with self.assertRaises(ValueError):
                    nightly.stage(
                        source,
                        source / "bad",
                        dict(source_sha=A, compiler_version=self.version),
                        1,
                        1,
                        999 if bad == "revision" else 4,
                    )
        self.assertEqual(self.github.calls, [])

    def test_first_publication_and_replacement_preserve_previous_until_commit(self):
        previous = self.seed()
        directory, state = self.generation(B)
        self.assertEqual(nightly.promote(self.github, directory, state), "published")
        self.assertEqual(self.github.tag, B)
        self.assertEqual(nightly.active_state(self.github.release), state)
        self.assertFalse(self.github.release["draft"])
        self.assertTrue(self.github.release["prerelease"])
        self.assertEqual(self.github.release["make_latest"], "false")
        calls = self.github.calls
        commit = next(
            i for i, c in enumerate(calls) if c[:2] == ("PATCH", "releases/1")
        )
        self.assertTrue(
            all(i > commit for i, c in enumerate(calls) if c[0] == "DELETE")
        )
        self.assertTrue(set(previous["assets"]).isdisjoint(self.github.files))
        self.assertEqual(set(state["assets"]), set(self.github.files))

    def test_failed_or_corrupt_upload_preserves_current_generation_and_tag(self):
        for mode in ("upload_failure", "corrupt"):
            with self.subTest(mode=mode):
                self.github = FakeGitHub()
                previous = self.seed()
                directory, state = self.generation(B)
                setattr(self.github, mode, True)
                with self.assertRaises((ValueError, RuntimeError)):
                    nightly.promote(self.github, directory, state)
                self.assertEqual(nightly.active_state(self.github.release), previous)
                self.assertEqual(self.github.tag, A)
                self.assertTrue(set(previous["assets"]).issubset(self.github.files))
                self.assertFalse(any(c[0] == "DELETE" for c in self.github.calls))

    def test_out_of_order_runs_and_duplicate_publication(self):
        self.seed()
        directory, state = self.generation(C)
        nightly.promote(self.github, directory, state)
        before = copy.deepcopy(self.github.release)
        for source, expected in [(B, "stale"), (OTHER, "stale"), (C, "unchanged")]:
            directory, state = self.generation(source)
            self.assertEqual(nightly.promote(self.github, directory, state), expected)
            self.assertEqual(self.github.tag, C)
            self.assertEqual(self.github.release, before)

    def test_body_failure_rolls_back_tag_without_deleting_old_assets(self):
        previous = self.seed()
        directory, state = self.generation(B)
        self.github.fail = ("PATCH", "releases/1", "before")
        with self.assertRaises(RuntimeError):
            nightly.promote(self.github, directory, state)
        self.assertEqual(self.github.tag, A)
        self.assertEqual(nightly.active_state(self.github.release), previous)
        self.assertTrue(set(previous["assets"]).issubset(self.github.files))
        self.assertEqual(nightly.promote(self.github, directory, state), "published")

    def test_lost_promotion_response_is_read_back_not_rolled_back(self):
        self.seed()
        directory, state = self.generation(B)
        self.github.fail = ("PATCH", "releases/1", "after")
        self.assertEqual(nightly.promote(self.github, directory, state), "published")
        self.assertEqual(self.github.tag, B)

    def test_retry_recovers_after_process_death_between_tag_and_body(self):
        previous = self.seed()
        directory, state = self.generation(B)
        # Model SIGKILL after the tag request: no Python finally/except runs.
        for name in state["assets"]:
            self.github.upload(directory / name)
        self.github.tag = B
        self.assertEqual(self.github.tag, B)
        self.assertEqual(nightly.active_state(self.github.release), previous)
        self.assertTrue(set(previous["assets"]).issubset(self.github.files))
        self.assertEqual(nightly.promote(self.github, directory, state), "published")

    def test_interrupted_first_upload_keeps_draft_and_can_retry(self):
        directory, state = self.generation()
        self.github.upload_failure = True
        with self.assertRaises(RuntimeError):
            nightly.promote(self.github, directory, state)
        self.assertTrue(self.github.release["draft"])
        self.github.upload_failure = False
        self.assertEqual(nightly.promote(self.github, directory, state), "published")

    def test_unmanaged_release_and_conflicting_assets_are_not_overwritten(self):
        self.seed()
        self.github.release["body"] = "manually owned release"
        directory, state = self.generation(B)
        with self.assertRaises(ValueError):
            nightly.promote(self.github, directory, state)
        self.assertEqual(self.github.release["body"], "manually owned release")

    def test_cleanup_failure_is_recoverable_without_rolling_back_new_release(self):
        self.seed()
        old_asset = next(iter(self.github.files.values()))
        directory, state = self.generation(B)
        self.github.fail = ("DELETE", f"releases/assets/{old_asset['id']}", "before")
        self.assertEqual(nightly.promote(self.github, directory, state), "published")
        self.assertEqual(self.github.tag, B)
        self.assertIn(old_asset["name"], self.github.files)
        self.assertEqual(nightly.promote(self.github, directory, state), "unchanged")
        self.assertEqual(set(self.github.files), set(state["assets"]))

    def test_conflicting_immutable_upload_is_never_clobbered(self):
        previous = self.seed()
        directory, state = self.generation(B)
        name = next(iter(state["assets"]))
        self.github.upload(directory / name)
        self.github.files[name]["digest"] = "sha256:" + "0" * 64
        with self.assertRaisesRegex(ValueError, "conflicting immutable"):
            nightly.promote(self.github, directory, state)
        self.assertEqual(nightly.active_state(self.github.release), previous)
        self.assertEqual(self.github.tag, A)
        self.assertTrue(set(previous["assets"]).issubset(self.github.files))

    def test_publication_authorization_precedes_any_release_write(self):
        identity = dict(source_sha=A, compiler_version=self.version, ci_run_id=10)
        base = dict(
            GITHUB_REPOSITORY=nightly.REPO,
            GITHUB_ACTIONS="true",
            GITHUB_REF="refs/heads/master",
            GITHUB_RUN_ID="30",
            GITHUB_RUN_ATTEMPT="1",
        )
        for changes, flag in [
            ({}, False),
            ({"GITHUB_REF": "refs/heads/topic"}, True),
            ({"GITHUB_ACTIONS": "false"}, True),
        ]:
            with self.subTest(changes=changes, flag=flag), patch.dict(
                os.environ, dict(base, **changes)
            ), patch.object(nightly, "eligible", return_value=identity), patch.object(
                nightly, "checkout_identity"
            ), patch.object(
                nightly, "promote"
            ) as promote:
                args = ["--stage", "publish"] + (["--publish"] if flag else [])
                self.assertEqual(nightly.main(args), 1)
                promote.assert_not_called()

    def test_gate_exports_the_validated_sha_and_version(self):
        identity = dict(source_sha=A, compiler_version=self.version, ci_run_id=10)
        output = self.root / "outputs"
        with patch.dict(
            os.environ,
            {"GITHUB_REPOSITORY": nightly.REPO, "GITHUB_OUTPUT": str(output)},
        ), patch.object(nightly, "eligible", return_value=identity), patch.object(
            nightly, "installers_ready"
        ), patch.object(
            nightly, "promote"
        ) as promote:
            self.assertEqual(nightly.main(["--stage", "gate", "--ci-run-id", "10"]), 0)
            self.assertIn("source_sha=" + A, output.read_text())
            self.assertIn("compiler_version=" + self.version, output.read_text())
            promote.assert_not_called()

    def test_package_uses_the_existing_target_package_plan(self):
        identity = dict(source_sha=A, compiler_version=self.version, ci_run_id=10)
        with patch.dict(os.environ, {"GITHUB_REPOSITORY": nightly.REPO}), patch.object(
            nightly, "eligible", return_value=identity
        ), patch.object(nightly, "checkout_identity"), patch(
            "tools.ci.common.main", return_value=0
        ) as run:
            self.assertEqual(
                nightly.main(
                    ["--stage", "package", "--target", "linux-riscv64", "--provision"]
                ),
                0,
            )
            run.assert_called_once_with(
                "package", ["--target", "linux-riscv64", "--provision"]
            )
            self.assertEqual(os.environ["RELEASE_VERSION"], self.version)

    def test_installers_must_match_reviewed_deployed_bytes(self):
        data = b"fixture"
        pins = self.root / "pins.json"
        pins.write_text(
            json.dumps(
                {
                    url: hashlib.sha256(data).hexdigest()
                    for url in [
                        "https://wave-lang.dev/install.sh",
                        "https://wave-lang.dev/install.ps1",
                    ]
                }
            )
        )
        requests = []

        def deployed_installer(request, *, timeout):
            # Model the CDN's rejection of Python's default User-Agent.
            if request.get_header("User-agent") != "Wave-Nightly/1.0":
                raise HTTPError(request.full_url, 403, "Forbidden", {}, None)
            self.assertEqual(timeout, 30)
            requests.append(request.full_url)
            return io.BytesIO(data)

        with patch.object(nightly, "INSTALLERS_FILE", pins), patch.object(
            nightly, "urlopen", side_effect=deployed_installer
        ):
            nightly.installers_ready()
        self.assertEqual(set(requests), set(json.loads(pins.read_text())))
        with patch.object(nightly, "INSTALLERS_FILE", pins), patch.object(
            nightly, "urlopen", return_value=io.BytesIO(b"old")
        ):
            with self.assertRaises(ValueError):
                nightly.installers_ready()

    def test_gate_reports_installer_url_and_exports_nothing_on_network_failure(self):
        identity = dict(source_sha=A, compiler_version=self.version, ci_run_id=10)
        urls = list(json.loads(nightly.INSTALLERS_FILE.read_text()))
        pins = self.root / "pins.json"
        pins.write_text(json.dumps({
            url: hashlib.sha256(b"fixture").hexdigest() for url in urls
        }))
        for index, url in enumerate(urls):
            for error in (
                HTTPError(url, 403, "Forbidden", {}, None),
                URLError("connection failed"),
                TimeoutError("timed out"),
            ):
                if isinstance(error, HTTPError):
                    self.addCleanup(error.close)
                with self.subTest(url=url, error=type(error).__name__):
                    output = self.root / "failed-outputs"
                    stderr = io.StringIO()
                    responses = [io.BytesIO(b"fixture") for _ in range(index)] + [error]
                    with patch.dict(
                        os.environ,
                        {
                            "GITHUB_REPOSITORY": nightly.REPO,
                            "GITHUB_OUTPUT": str(output),
                        },
                    ), patch.object(
                        nightly, "eligible", return_value=identity
                    ), patch.object(
                        nightly, "urlopen", side_effect=responses
                    ), patch.object(nightly, "INSTALLERS_FILE", pins), patch(
                        "sys.stderr", stderr
                    ):
                        self.assertEqual(nightly.main(["--stage", "gate"]), 1)
                    self.assertIn(
                        f"cannot verify deployed installer {url}", stderr.getvalue()
                    )
                    self.assertIn(str(error), stderr.getvalue())
                    self.assertFalse(output.exists())

    def test_publication_is_opt_in_and_versioned_rules_remain_strict(self):
        with patch.dict(os.environ, {"GITHUB_REPOSITORY": "fork/Wave"}), patch.object(
            nightly, "GitHub"
        ) as github:
            self.assertEqual(nightly.main(["--stage", "publish"]), 1)
            github.assert_not_called()


if __name__ == "__main__":
    unittest.main()
