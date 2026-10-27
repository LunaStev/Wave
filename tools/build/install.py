# SPDX-License-Identifier: MPL-2.0
"""Install a complete adjacent runtime through Meson's prefix/DESTDIR contract."""

import argparse
import os
import shutil
import sys
import tempfile
from pathlib import Path

# Meson executes install scripts by absolute path, including from another cwd.
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from tools.build import runtime
from tools.ci.package import stage_package


def install(binary, target, bindir):
    prefix = Path(os.environ["MESON_INSTALL_DESTDIR_PREFIX"])
    bindir = Path(bindir)
    if bindir.is_absolute() or ".." in bindir.parts:
        raise ValueError(
            "Wave runtime installation requires a bindir inside the prefix"
        )
    destination = prefix / bindir
    if os.environ.get("MESON_INSTALL_DRY_RUN") == "1":
        print(f"Would install the complete Wave runtime into {destination}")
        return
    # Stage and validate everything before modifying an existing installation.
    with tempfile.TemporaryDirectory(prefix="wave-install-") as temporary:
        stage = stage_package(
            runtime, target, binary, "payload", dist_dir=Path(temporary)
        )
        destination.mkdir(parents=True, exist_ok=True)
        for source in stage.iterdir():
            output = destination / source.name
            if source.is_dir():
                if output.is_symlink():
                    output.unlink()
                if output.exists():
                    shutil.rmtree(output)
                shutil.copytree(source, output)
            else:
                # Replacing the directory entry also preserves any file that
                # shares an inode with the previous installed compiler.
                with tempfile.TemporaryDirectory(
                    prefix=".wave-install-", dir=destination
                ) as pending:
                    candidate = Path(pending) / source.name
                    shutil.copy2(source, candidate)
                    os.replace(candidate, output)
    print(f"Installed the complete Wave runtime into {destination}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--bindir", required=True)
    parser.add_argument("--llvm-prefix", default="")
    args = parser.parse_args()
    if args.llvm_prefix:
        os.environ["LLVM_SYS_211_PREFIX"] = args.llvm_prefix
        os.environ["WAVE_LLVM_HOME"] = args.llvm_prefix
        config = "llvm-config.exe" if os.name == "nt" else "llvm-config"
        os.environ["LLVM_CONFIG_PATH"] = str(Path(args.llvm_prefix) / "bin" / config)
    install(args.binary, args.target, args.bindir)
