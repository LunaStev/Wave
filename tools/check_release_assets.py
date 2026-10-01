#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Verify complete archive/checksum coverage before producing SHA256SUMS."""
import argparse
import hashlib
import os
from pathlib import Path
import re
import sys
import tempfile


ARCHIVE_TARGETS = (
    "x86_64-linux-gnu", "aarch64-linux-gnu", "riscv64-linux-gnu", "loongarch64-linux-gnu",
    "x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc",
    "aarch64-apple-darwin", "x86_64-apple-darwin",
)


def verify(directory, version):
    directory = Path(directory)
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?", version):
        raise ValueError(f"invalid release version: {version!r}")
    expected = {f"wave-v{version}-{target}" + (".zip" if "windows" in target else ".tar.gz")
                for target in ARCHIVE_TARGETS}
    archives = {p.name for p in directory.iterdir() if p.name.endswith((".tar.gz", ".zip"))}
    sidecars = {p.name for p in directory.iterdir() if p.name.endswith(".sha256")}
    for label, actual, required in (("archives", archives, expected),
                                    ("checksum files", sidecars, {name + ".sha256" for name in expected})):
        if actual != required:
            raise ValueError(f"release {label}: missing={sorted(required - actual)}, unexpected={sorted(actual - required)}")
    records = []
    for name in sorted(expected):
        archive, sidecar = directory / name, directory / (name + ".sha256")
        if archive.is_symlink() or sidecar.is_symlink() or not archive.is_file() or not sidecar.is_file():
            raise ValueError(f"archive/checksum must be regular files: {name}")
        lines = sidecar.read_text(encoding="ascii").splitlines()
        match = re.fullmatch(r"([0-9a-fA-F]{64}) [ *](.+)", lines[0]) if len(lines) == 1 else None
        if match is None or match[2] != name:
            raise ValueError(f"{sidecar.name}: expected exactly one checksum record naming {name}")
        digest = hashlib.sha256()
        with archive.open("rb") as stream:
            for block in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(block)
        if digest.hexdigest() != match[1].lower():
            raise ValueError(f"checksum mismatch: {name}")
        records.append(f"{digest.hexdigest()}  {name}\n")
    # Never publish a partial manifest or follow an existing manifest symlink.
    fd, temporary = tempfile.mkstemp(prefix=".SHA256SUMS-", dir=directory)
    try:
        with os.fdopen(fd, "w", encoding="ascii", newline="\n") as stream:
            stream.writelines(records)
        os.replace(temporary, directory / "SHA256SUMS")
    finally:
        Path(temporary).unlink(missing_ok=True)
    return len(records)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--version", required=True)
    options = parser.parse_args(argv)
    try:
        count = verify(options.directory, options.version)
    except (OSError, ValueError) as error:
        print(f"Release asset validation failed: {error}", file=sys.stderr)
        return 1
    print(f"Verified {count} release archives and checksum records")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
