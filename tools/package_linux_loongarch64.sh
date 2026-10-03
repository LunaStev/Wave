#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
set -euo pipefail
export RELEASE_VERSION="${1:?usage: package_linux_loongarch64.sh <version>}"
# Shared packaging consumes the pinned, prebuilt LLVM SDK and retains QEMU smoke.
exec python3 -m tools.ci.package --target linux-loong64 --lane release/package-linux-loongarch64 --provision
