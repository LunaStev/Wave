#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
set -euo pipefail
export RELEASE_VERSION="${1:?usage: package_linux_riscv64.sh <version>}"
exec python3 -m tools.ci.package --target linux-riscv64 --provision
