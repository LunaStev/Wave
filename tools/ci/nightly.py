# SPDX-License-Identifier: MPL-2.0
"""Rolling Nightly identity, staging and recoverable GitHub publication.

The release body's state marker is the committed generation. Assets are immutable
and verified before the tag/body promotion. A retry repairs a tag left ahead of
that marker by an interrupted promotion; old assets survive until commit.
"""

import argparse
import base64
from datetime import datetime, timedelta, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
from urllib.request import Request, urlopen

from tools.ci.common import ROOT
from tools.ci.release import verify_metadata

REPO = "wavefnd/Wave"
SHA = r"[0-9a-f]{40}"
MARKER = r"<!-- wave-nightly-state:(.*?) -->"
INSTALLERS_FILE = Path(__file__).with_name("nightly_installers.json")


def sha(value):
    if not isinstance(value, str) or not re.fullmatch(SHA, value):
        raise ValueError("expected a full source SHA")
    return value


class GitHub:
    def api(self, endpoint, *, method="GET", data=None, optional=False):
        args = ["gh", "api", f"repos/{REPO}/{endpoint}", "--method", method]
        if data is not None:
            args += ["--input", "-"]
        result = subprocess.run(
            args,
            input=json.dumps(data) if data is not None else None,
            text=True,
            capture_output=True,
            timeout=120,
        )
        if result.returncode:
            if optional and "HTTP 404" in result.stderr:
                return None
            raise RuntimeError(f"GitHub {method} {endpoint}: {result.stderr.strip()}")
        return json.loads(result.stdout) if result.stdout.strip() else None

    def assets(self, release_id):
        result = []
        page = 1
        while True:
            batch = self.api(f"releases/{release_id}/assets?per_page=100&page={page}")
            result.extend(batch)
            if len(batch) < 100:
                return result
            page += 1

    def upload(self, path):
        # Never --clobber: an existing usable asset must not be deleted first.
        subprocess.run(
            ["gh", "release", "upload", "nightly", str(path), "--repo", REPO],
            check=True,
            timeout=1800,
        )


def ancestor(github, older, newer):
    sha(older)
    sha(newer)
    if older == newer:
        return True
    comparison = github.api(f"compare/{older}...{newer}")
    return (
        comparison["status"] == "ahead"
        and comparison["merge_base_commit"]["sha"] == older
    )


def eligible(github, run_id):
    if not str(run_id).isdigit() or int(run_id) <= 0:
        raise ValueError("a successful canonical master CI run ID is required")
    run = github.api(f"actions/runs/{run_id}")
    if (
        run.get("repository", {}).get("full_name") != REPO
        or run.get("head_repository", {}).get("full_name") != REPO
        or run.get("event") != "push"
        or run.get("head_branch") != "master"
        or run.get("path") != ".github/workflows/ci.yml"
        or run.get("status") != "completed"
        or run.get("conclusion") != "success"
    ):
        raise ValueError("Nightly requires successful canonical master push CI")
    jobs = []
    page = 1
    while True:
        batch = github.api(
            f"actions/runs/{run_id}/attempts/{run['run_attempt']}/jobs?per_page=100&page={page}"
        )["jobs"]
        jobs.extend(batch)
        if len(batch) < 100:
            break
        page += 1
    if not jobs or any(
        j.get("status") != "completed" or j.get("conclusion") != "success" for j in jobs
    ):
        raise ValueError(
            "CI jobs must all complete successfully; skipped jobs are not validation"
        )
    source = sha(run["head_sha"])
    master = github.api("git/ref/heads/master")["object"]["sha"]
    if not ancestor(github, source, master):
        raise ValueError("CI source is not in canonical master history")
    content = github.api(f"contents/Cargo.toml?ref={source}")
    version = tomllib.loads(base64.b64decode(content["content"]).decode())["package"][
        "version"
    ]
    if not re.fullmatch(r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?", version):
        raise ValueError("invalid compiler version")
    return {"source_sha": source, "compiler_version": version, "ci_run_id": int(run_id)}


def checkout_identity(identity):
    source = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]
    if source != identity["source_sha"] or version != identity["compiler_version"]:
        raise ValueError("checkout differs from the exact CI-validated source/version")


def installers_ready():
    # The installer change lives in wavefnd/wave-platform. Do not publish until
    # those reviewed bytes are actually deployed, regardless of merge order.
    pins = json.loads(INSTALLERS_FILE.read_text())
    required = {"https://wave-lang.dev/install.sh", "https://wave-lang.dev/install.ps1"}
    if set(pins) != required or any(
        not re.fullmatch(r"[0-9a-f]{64}", value) for value in pins.values()
    ):
        raise ValueError("both reviewed installer deployment hashes are required")
    for url, expected in pins.items():
        # Identify this client: the installer CDN rejects Python's default UA.
        request = Request(url, headers={"User-Agent": "Wave-Nightly/1.0"})
        try:
            with urlopen(request, timeout=30) as response:
                data = response.read(1024 * 1024 + 1)
        except OSError as error:
            raise RuntimeError(
                f"cannot verify deployed installer {url}: {error}"
            ) from error
        if len(data) > 1024 * 1024 or hashlib.sha256(data).hexdigest() != expected:
            raise ValueError(f"versioned-only installer is not deployed: {url}")


def file_record(path):
    with path.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    return {"sha256": digest, "size": path.stat().st_size}


def stage(directory, output, identity, run_id, attempt, revision):
    if (
        not str(run_id).isdigit()
        or not str(attempt).isdigit()
        or min(int(run_id), int(attempt)) < 1
    ):
        raise ValueError("invalid Nightly workflow run identity")
    source, version = sha(identity["source_sha"]), identity["compiler_version"]
    verify_metadata(directory, version, source, revision)
    generation = f"{source}-{run_id}-{attempt}"
    output.mkdir()
    records = {}
    checksums = []
    for descriptor in sorted(directory.glob("*.metadata.json")):
        metadata = json.loads(descriptor.read_text())
        original = metadata["archive"]
        name = original.replace(f"wave-v{version}-", f"wave-nightly-{generation}-", 1)
        archive = output / name
        shutil.copyfile(directory / original, archive)
        if file_record(archive)["sha256"] != metadata["sha256"]:
            raise ValueError("package changed while staging Nightly")
        metadata.update(archive=name, channel="nightly", generation=generation)
        (output / (name + ".metadata.json")).write_text(
            json.dumps(metadata, indent=2) + "\n"
        )
        line = f"{metadata['sha256']}  {name}\n"
        (output / (name + ".sha256")).write_text(line)
        checksums.append(line)
    (output / f"nightly-{generation}-SHA256SUMS").write_text("".join(checksums))
    for path in sorted(output.iterdir()):
        records[path.name] = file_record(path)
    state = dict(
        identity,
        schema_version=1,
        generation=generation,
        run_id=int(run_id),
        built_at=datetime.now(timezone.utc).isoformat(),
        assets=records,
    )
    manifest = output / f"nightly-{generation}.json"
    manifest.write_text(json.dumps(state, indent=2) + "\n")
    state["assets"][manifest.name] = file_record(manifest)
    return state


def active_state(release):
    if release is None:
        return None
    if (
        release.get("tag_name") != "nightly"
        or not release.get("prerelease")
    ):
        raise ValueError("refusing to replace an unmanaged Nightly release")
    match = re.search(MARKER, release.get("body") or "")
    if match is None:
        if (
            release.get("draft")
            and not release.get("body")
            and release.get("name") == "Wave Nightly"
        ):
            return None  # Interrupted first publication, before promotion.
        raise ValueError("Nightly release has no recovery state")
    state = json.loads(match[1])
    sha(state["source_sha"])
    if state.get("schema_version") != 1 or not state.get("assets"):
        raise ValueError("invalid Nightly recovery state")
    if release.get("name") != title(state):
        raise ValueError("Nightly title differs from its recovery state")
    return state


def verify_remote(github, release_id, state):
    assets = {a["name"]: a for a in github.assets(release_id)}
    for name, record in state["assets"].items():
        asset = assets.get(name, {})
        if (
            asset.get("state") != "uploaded"
            or asset.get("size") != record["size"]
            or asset.get("digest") != "sha256:" + record["sha256"]
        ):
            raise ValueError(f"unverified Nightly asset: {name}")


def set_tag(github, source):
    sha(source)
    tag = github.api("git/ref/tags/nightly", optional=True)
    if tag is None:
        github.api(
            "git/refs", method="POST", data={"ref": "refs/tags/nightly", "sha": source}
        )
    elif tag["object"]["sha"] != source:
        if tag["object"]["type"] != "commit":
            raise ValueError("Nightly must use a lightweight commit tag")
        github.api(
            "git/refs/tags/nightly", method="PATCH", data={"sha": source, "force": True}
        )


def title(state):
    # Existing unnumbered releases remain readable during migration.
    if "publication_number" not in state:
        return "Wave Nightly"
    number = state["publication_number"]
    if type(number) is not int or number < 1:
        raise ValueError("invalid Nightly publication number")
    built = datetime.fromisoformat(state["built_at"])
    if built.tzinfo is None:
        raise ValueError("Nightly build timestamp requires a timezone")
    date = built.astimezone(timezone(timedelta(hours=9))).date()
    return f"Wave {date.isoformat()}-{number:02d}-nightly"


def notes(state):
    source = state["source_sha"]
    lines = [
        title(state),
        "",
        f"Commit: [{source}](https://github.com/{REPO}/commit/{source})",
        "Branch: master",
        f"Built: {state['built_at']}",
        f"Compiler version: {state['compiler_version']}",
        f"Build: https://github.com/{REPO}/actions/runs/{state['run_id']}",
        f"Validated CI: https://github.com/{REPO}/actions/runs/{state['ci_run_id']}",
        "",
        "This release tracks the latest successfully validated master build.",
        "",
        "Download manually below. Nightly is not available through install.sh or install.ps1.",
        "",
    ]
    if state.get("changelog"):
        lines += [state["changelog"], ""]
    for name in sorted(state["assets"]):
        lines.append(
            f"- [{name}](https://github.com/{REPO}/releases/download/nightly/{name})"
        )
    lines += [
        "",
        "<!-- wave-nightly-state:" + json.dumps(state, separators=(",", ":")) + " -->",
    ]
    return "\n".join(lines)


def cleanup(github, release_id, state):
    for asset in github.assets(release_id):
        if (
            asset["name"].startswith(("wave-nightly-", "nightly-"))
            and asset["name"] not in state["assets"]
        ):
            try:
                github.api(f"releases/assets/{asset['id']}", method="DELETE")
            except RuntimeError as error:
                # A cleanup failure leaves only surplus assets; retry is safe.
                print(f"Nightly cleanup deferred: {error}", file=sys.stderr)


def promote(github, directory, state):
    release = github.api("releases/tags/nightly", optional=True)
    # Draft releases may not be returned by the tag endpoint on first retries.
    if release is None:
        candidates = github.api("releases?per_page=100")
        release = next((r for r in candidates if r["tag_name"] == "nightly"), None)
    previous = active_state(release)
    source = state["source_sha"]
    if previous:
        verify_remote(github, release["id"], previous)
        if source != previous["source_sha"] and not ancestor(
            github, previous["source_sha"], source
        ):
            print("Skipping stale or unrelated Nightly generation")
            return "stale"
        # Reconcile a tag moved before a cancelled/failed release-body update.
        set_tag(github, previous["source_sha"])
        if source == previous["source_sha"]:
            cleanup(github, release["id"], previous)
            return "unchanged"
    if release is None:
        set_tag(github, source)
        release = github.api(
            "releases",
            method="POST",
            data={
                "tag_name": "nightly",
                "target_commitish": source,
                "name": "Wave Nightly",
                "body": "",
                "draft": True,
                "prerelease": True,
                "make_latest": "false",
            },
        )
    existing = {a["name"]: a for a in github.assets(release["id"])}
    for name, record in state["assets"].items():
        if name in existing:
            asset = existing[name]
            if (
                asset.get("digest") != "sha256:" + record["sha256"]
                or asset.get("size") != record["size"]
            ):
                raise ValueError(f"conflicting immutable Nightly asset: {name}")
        else:
            github.upload(directory / name)
    verify_remote(github, release["id"], state)
    # Check ancestry again after a long upload; a rewritten master cannot publish.
    master = github.api("git/ref/heads/master")["object"]["sha"]
    if not ancestor(github, source, master):
        raise ValueError("source left canonical master history during upload")
    # The release-body commit owns the counter, so failed uploads, stale runs
    # and retries of the same source never consume a number. The existing
    # unnumbered Nightly is publication 1; new installations also start at 1.
    state["publication_number"] = (
        previous.get("publication_number", 1) + 1 if previous else 1
    )
    try:
        set_tag(github, source)
        github.api(
            f"releases/{release['id']}",
            method="PATCH",
            data={
                "name": title(state),
                "body": notes(state),
                "draft": False,
                "prerelease": True,
                "make_latest": "false",
                "target_commitish": source,
            },
        )
    except BaseException:
        # The API may have committed before the response was lost. Read back
        # before rollback; otherwise we could corrupt a successful promotion.
        observed = github.api(f"releases/{release['id']}")
        if active_state(observed) != state:
            if previous:
                set_tag(github, previous["source_sha"])
            raise
    observed = github.api(f"releases/{release['id']}")
    if active_state(observed) != state or observed.get("draft"):
        raise ValueError("Nightly release promotion could not be verified")
    verify_remote(github, release["id"], state)
    if github.api("git/ref/tags/nightly")["object"]["sha"] != source:
        raise ValueError("Nightly tag differs from the active generation")
    cleanup(github, release["id"], state)
    return "published"


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--stage", choices=("gate", "package", "publish"), required=True
    )
    parser.add_argument("--ci-run-id", default=os.environ.get("WAVE_NIGHTLY_CI_RUN"))
    parser.add_argument("--target", default="linux-amd64")
    parser.add_argument("--provision", action="store_true")
    parser.add_argument("--report-json", type=Path)
    parser.add_argument("--publish", action="store_true")
    parser.add_argument("--directory", type=Path, default=ROOT / "release-assets")
    options = parser.parse_args(argv)
    try:
        if options.publish and options.stage != "publish":
            raise ValueError("--publish is only valid for the publish stage")
        if os.environ.get("GITHUB_REPOSITORY") != REPO:
            raise ValueError("Nightly requires the canonical repository")
        github = GitHub()
        identity = eligible(github, options.ci_run_id)
        if options.stage == "gate":
            installers_ready()
            with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
                for key, value in identity.items():
                    output.write(f"{key}={value}\n")
            return 0
        checkout_identity(identity)
        if options.stage == "package":
            from tools.ci.common import main as run_plan

            os.environ["RELEASE_VERSION"] = identity["compiler_version"]
            args = ["--target", options.target]
            if options.provision:
                args += ["--provision"]
            if options.report_json:
                args += ["--report-json", str(options.report_json)]
            return run_plan("package", args)
        if (
            not options.publish
            or os.environ.get("GITHUB_ACTIONS") != "true"
            or os.environ.get("GITHUB_REF") != "refs/heads/master"
        ):
            raise ValueError(
                "Nightly publication requires --publish in the canonical master workflow"
            )
        run_id = os.environ["GITHUB_RUN_ID"]
        own_run = github.api(f"actions/runs/{run_id}")
        if (
            own_run.get("path") != ".github/workflows/nightly.yml"
            or own_run.get("repository", {}).get("full_name") != REPO
            or own_run.get("event") not in ("workflow_run", "workflow_dispatch")
        ):
            raise ValueError("publication must run in the canonical Nightly workflow")
        installers_ready()
        revision = json.loads((ROOT / "std/manifest.json").read_text())[
            "compatibility_revision"
        ]
        with tempfile.TemporaryDirectory(prefix="wave-nightly-") as temporary:
            output = Path(temporary) / "assets"
            state = stage(
                options.directory,
                output,
                identity,
                run_id,
                os.environ["GITHUB_RUN_ATTEMPT"],
                revision,
            )
            from tools.ci.release_notes import generate
            state["changelog"] = generate(github.api, identity["source_sha"])
            print(promote(github, output, state))
        return 0
    except (
        OSError,
        ValueError,
        KeyError,
        RuntimeError,
        subprocess.SubprocessError,
    ) as error:
        print(f"Nightly failed: {error}", file=sys.stderr)
        return 1
