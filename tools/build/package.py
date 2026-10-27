# SPDX-License-Identifier: MPL-2.0
"""Create one Meson distribution with the shared CI archive/identity policy."""

import argparse
from pathlib import Path

from tools.ci.package import package_target


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--version", required=True)
    args = parser.parse_args()
    package_target(args.target, args.binary, args.version)


if __name__ == "__main__":
    main()
