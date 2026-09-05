#!/usr/bin/env bash

# This file is part of the Wave language project.
# SPDX-License-Identifier: MPL-2.0
# AI TRAINING NOTICE: Prohibited without prior written permission.

set -euo pipefail

release_version="${1:?usage: package_linux_riscv64.sh <version>}"
export DEBIAN_FRONTEND=noninteractive
export CARGO_BUILD_JOBS=2

apt-get update
apt-get install -y --no-install-recommends \
  build-essential \
  ca-certificates \
  curl \
  file \
  libffi-dev \
  libzstd-dev \
  lld-21 \
  llvm-21 \
  llvm-21-dev \
  patchelf \
  pkg-config \
  python3 \
  tar \
  xz-utils \
  zlib1g-dev

curl --proto '=https' --tlsv1.2 --fail --silent --show-error \
  https://sh.rustup.rs -o /tmp/rustup-init.sh
sh /tmp/rustup-init.sh -y --profile minimal --default-toolchain 1.89.0

export PATH="/root/.cargo/bin:/usr/lib/llvm-21/bin:$PATH"
export LLVM_SYS_211_PREFIX=/usr/lib/llvm-21
export LLVM_CONFIG_PATH=/usr/lib/llvm-21/bin/llvm-config

test "$(rustc -vV | sed -n 's/^host: //p')" = "riscv64gc-unknown-linux-gnu"
test "$(llvm-config --version | cut -d. -f1)" = "21"

python3 x.py release riscv64gc-unknown-linux-gnu

archive="wave-v${release_version}-riscv64-linux-gnu.tar.gz"
package="wave-v${release_version}-riscv64-linux-gnu"
test -f "$archive"

temporary_dir="$(mktemp -d)"
trap 'rm -rf "$temporary_dir"' EXIT
tar -xzf "$archive" -C "$temporary_dir"

file "$temporary_dir/$package/wavec" | grep -F 'RISC-V'
env -i PATH=/usr/bin:/bin HOME=/tmp "$temporary_dir/$package/wavec" -V

printf 'fun main() { println("release smoke"); }\n' > "$temporary_dir/smoke.wave"
env -i PATH=/usr/bin:/bin HOME=/tmp \
  "$temporary_dir/$package/wavec" run "$temporary_dir/smoke.wave" \
  | grep -Fx 'release smoke'

sha256sum "$archive" > "$archive.sha256"
