#!/usr/bin/env python3

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

"""
Wave: get_maintainer.py

Reads MAINTAINERS file and determines which maintainers should be CC'd
based on file paths included in a patch or manually provided.

Usage:
    python3 tools/get_maintainer.py <file1> <file2> ...

Example:
    python3 tools/get_maintainer.py front/parser/ast.rs
"""

import sys
import ntpath
import posixpath
from pathlib import Path

MAINTAINERS_FILE = Path(__file__).resolve().parent.parent / "MAINTAINERS"


def parse_maintainers(path=MAINTAINERS_FILE):
    sections = []
    current = None
    for line in Path(path).read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("[") and line.endswith("]"):
            current = {"name": line[1:-1], "maintainers": [], "files": []}
            sections.append(current)
        elif line.startswith(("M:", "F:")):
            if current is None:
                raise ValueError("MAINTAINERS fields require a section")
            current["maintainers" if line.startswith("M:") else "files"].append(
                line[2:].strip()
            )
    return sections


def pattern_matches(pattern, path):
    if pattern.endswith("/"):
        return path == pattern[:-1] or path.startswith(pattern)
    return path == pattern


def maintainers_for_paths(paths, sections):
    maintainers = set()
    for path in paths:
        path = path.replace("\\", "/")
        if path.startswith("/") or ntpath.splitdrive(path)[0]:
            continue
        path = posixpath.normpath(path)
        if path == ".." or path.startswith("../"):
            continue
        for section in sections:
            if any(pattern_matches(pattern, path) for pattern in section["files"]):
                maintainers.update(section["maintainers"])
    return sorted(maintainers)


def stale_patterns(sections, tracked_paths):
    return sorted(
        pattern
        for section in sections
        for pattern in section["files"]
        if not any(pattern_matches(pattern, path) for path in tracked_paths)
    )


def main(argv=None):
    files = sys.argv[1:] if argv is None else argv
    if not files:
        print("Usage: get_maintainer.py <file1> [file2] ...")
        return 1
    try:
        sections = parse_maintainers()
    except (OSError, ValueError) as error:
        print(
            f"Error: cannot read MAINTAINERS at {MAINTAINERS_FILE}: {error}",
            file=sys.stderr,
        )
        return 1
    maintainers = maintainers_for_paths(files, sections)
    print(
        "Maintainers to CC:"
        if maintainers
        else "No specific maintainers found. Using default:"
    )
    for maintainer in maintainers or ["luna@lunastev.org"]:
        print("  " + maintainer)
    return 0


if __name__ == "__main__":
    sys.exit(main())
