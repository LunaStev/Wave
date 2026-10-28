# SPDX-License-Identifier: MPL-2.0
"""Remove owned Cargo and distribution outputs, preserving unrelated files."""

import argparse
import re
import shutil
from pathlib import Path

from tools.build.runtime import ROOT, release_target_name


def clean(root, targets):
    root = Path(root)
    for directory in (root / "target", root / "dist"):
        if directory.is_symlink():
            directory.unlink()
        elif directory.exists():
            shutil.rmtree(directory)
    names = "|".join(re.escape(release_target_name(t)) for t in targets)
    generated = re.compile(
        rf"wave-v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?"
        rf"-(?:{names})\.(?:tar\.gz|zip)(?:\.sha256|\.metadata\.json)?"
    )
    for path in root.iterdir():
        if generated.fullmatch(path.name) and (path.is_file() or path.is_symlink()):
            path.unlink()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("targets", nargs="+")
    clean(ROOT, parser.parse_args().targets)
