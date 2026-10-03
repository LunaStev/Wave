#!/usr/bin/env bash

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

#
# Wave: Patch Verification Script
# This script applies a patch (via git am) and verifies:
# - DCO (Signed-off-by)
# - Build
# - Tests
# - Formatting
# - Lint (clippy)
#
# Usage:
#   ./tools/verify_patch.sh path/to/patch.patch
#
# Requirements:
#   - cargo
#   - rustfmt
#   - clippy
#

set -euo pipefail

if [[ $# != 1 || ! -f "$1" ]]; then
    echo "Usage: $0 path/to/patch.patch" >&2
    exit 1
fi
patch_file="$(realpath "$1")"
repo="$(git rev-parse --show-toplevel)"
cd "$repo"
if [[ -n "$(git status --porcelain)" || -d "$(git rev-parse --git-path rebase-apply)" || -d "$(git rev-parse --git-path rebase-merge)" ]]; then
    echo "Patch verification requires a clean repository without an active rebase or git am." >&2
    exit 1
fi
original="$(git symbolic-ref --short -q HEAD || git rev-parse HEAD)"
base="$(git rev-parse HEAD)"
test_branch=""
cleanup() {
    status=$?
    trap - EXIT INT TERM
    if [[ -n "$test_branch" ]]; then
        if [[ -d "$(git rev-parse --git-path rebase-apply)" ]]; then
            git am --abort || status=1
        fi
        if git checkout --quiet "$original"; then
            git branch -D "$test_branch" >/dev/null || status=1
        else
            echo "Could not restore $original; preserved $test_branch for recovery." >&2
            status=1
        fi
    fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
# Only record ownership after successful creation; never delete an existing branch.
candidate="patch-verify-$(date +%s)-$$"
git checkout --quiet -b "$candidate"
test_branch="$candidate"
git am "$patch_file"
unsigned=0
while read -r commit; do
    if ! git show -s --format=%B "$commit" | git interpret-trailers --parse | grep -Eq '^Signed-off-by: .+ <[^<>[:space:]]+@[^<>[:space:]]+>$'; then
        echo "Missing DCO Signed-off-by trailer: $commit" >&2
        unsigned=1
    fi
done < <(git rev-list "$base..HEAD")
# Bash 3.2 does not apply errexit to a failing [[ ... ]] command here.
# Reject explicitly so macOS cannot continue to Cargo after a DCO failure.
if [[ "$unsigned" != 0 ]]; then
    exit 1
fi
for phase in fmt build test clippy; do
    echo "Verifying cargo $phase"
    case "$phase" in
        fmt) cargo fmt --all --check ;;
        build) cargo build --locked --release --jobs 2 ;;
        test) cargo test --locked --workspace --all-targets --jobs 2 ;;
        clippy) cargo clippy --locked --workspace --all-targets --jobs 2 -- -D warnings ;;
    esac
done
echo "Patch verification passed."
