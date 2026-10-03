# SPDX-License-Identifier: MPL-2.0
"""Verify CI SDK artifacts and stage an immutable generation for the website."""

import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import shutil
import tempfile

from tools.ci.llvm_bundle import MACHINES, identity, sha256


def prepare_publication(inputs, destination):
    inputs, destination = Path(inputs).resolve(), Path(destination).resolve()
    records = []
    for path in sorted(inputs.rglob("wave-llvm-*.json")):
        entry = json.loads(path.read_text())
        target = entry.get("target")
        if any(entry.get(k) != v for k, v in identity(target).items()):
            raise ValueError(f"wrong toolchain identity in {path.name}")
        revision = entry.get("revision", "")
        if not isinstance(revision, str) or not re.fullmatch(r"r[1-9][0-9]*", revision):
            raise ValueError("invalid toolchain revision")
        filename = f"wave-llvm-{entry['llvm_version']}-{target}-{revision}.tar.xz"
        relative = Path("llvm") / entry["llvm_version"] / revision / filename
        if entry.get("filename") != filename or entry.get("url") != f"https://wave-lang.dev/downloads/toolchains/{relative.as_posix()}":
            raise ValueError("unexpected toolchain filename or URL")
        archive = path.with_name(filename)
        digest = sha256(archive)
        if digest != entry.get("sha256") or archive.stat().st_size != entry.get("size_bytes"):
            raise ValueError(f"toolchain checksum or size mismatch: {filename}")
        if archive.with_name(filename + ".sha256").read_text().strip() != f"{digest}  {filename}":
            raise ValueError(f"invalid checksum sidecar: {filename}")
        records.append((entry, path, archive, relative))
    if len(records) != len(MACHINES) or {e[0]["target"] for e in records} != set(MACHINES):
        raise ValueError("publication requires exactly one verified SDK for both RISC-V64 and LoongArch64")
    generations = {relative.parent for _, _, _, relative in records}
    if len(generations) != 1:
        raise ValueError("SDKs must belong to the same LLVM version and revision")
    generation = generations.pop()
    destination.mkdir(parents=True, exist_ok=True)
    destination.chmod(0o755)
    final = destination / generation
    catalog_path = destination / "index.json"
    catalog = json.loads(catalog_path.read_text()) if catalog_path.exists() else {"schema_version": 1, "bundles": []}
    if catalog.get("schema_version") != 1 or not isinstance(catalog.get("bundles"), list):
        raise ValueError("unsupported website toolchain catalog")
    old = {e["url"]: e for e in catalog["bundles"]}
    for entry, _, _, _ in records:
        previous = old.get(entry["url"])
        if previous and (previous["sha256"] != entry["sha256"] or previous["size_bytes"] != entry["size_bytes"]):
            raise ValueError("published toolchain identity is immutable; use a new revision")
    # Assemble both architectures outside the public version path, then expose together.
    final.parent.mkdir(parents=True, exist_ok=True)
    (destination / "llvm").chmod(0o755)
    final.parent.chmod(0o755)
    with tempfile.TemporaryDirectory(prefix=".toolchains-publish-", dir=destination) as folder:
        stage = Path(folder) / "generation"
        stage.mkdir(mode=0o755)
        stage.chmod(0o755)
        for entry, metadata, archive, relative in records:
            for source in (archive, archive.with_name(archive.name + ".sha256"), metadata):
                if final.exists():
                    existing = final / source.name
                    if not existing.is_file() or sha256(existing) != sha256(source):
                        raise ValueError("published toolchain files are immutable; use a new revision")
                shutil.copyfile(source, stage / source.name)
                (stage / source.name).chmod(0o644)
        if not final.exists():
            stage.rename(final)
        published = datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")
        for entry, _, _, _ in records:
            old[entry["url"]] = dict(entry, published_at=old.get(entry["url"], {}).get("published_at", published))
        catalog["bundles"] = sorted(old.values(), key=lambda e: (e["published_at"], e["target"]), reverse=True)
        temporary_index = Path(folder) / "index.json"
        temporary_index.write_text(json.dumps(catalog, indent=2) + "\n")
        temporary_index.chmod(0o644)
        os.replace(temporary_index, catalog_path)
    return {"schema_version": 1, "bundles": {e["target"]: e for e, _, _, _ in records}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True, help="Downloaded verified CI artifacts")
    parser.add_argument("--destination", type=Path, required=True, help="Platform data/toolchains directory")
    parser.add_argument("--manifest", type=Path, required=True, help="Wave tools/ci/llvm_bundles.json output")
    args = parser.parse_args()
    if args.manifest.resolve().is_relative_to(args.input.resolve()) or args.manifest.resolve().is_relative_to(args.destination.resolve()):
        parser.error("the compiler pin manifest must be outside the artifacts and website data")
    manifest = prepare_publication(args.input, args.destination)
    args.manifest.write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"Published {len(manifest['bundles'])} immutable SDKs; pins written to {args.manifest}")


if __name__ == "__main__":
    main()
