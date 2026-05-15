#!/usr/bin/env bash

# Run a Windows llvm-config.exe through Wine while returning paths usable by
# the Linux-hosted MinGW linker.
set -euo pipefail

LLVM_CONFIG_EXE="${LLVM_CONFIG_EXE:-/opt/llvm-win/bin/llvm-config.exe}"

wine "$LLVM_CONFIG_EXE" "$@" \
    2> >(sed '/fixme:winediag:loader_init/d;/Please mention your exact version/d' >&2) \
    | sed -E 's#Z:/#/#g'
