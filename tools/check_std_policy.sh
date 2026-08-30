#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

failed=0

echo "[check] std policy validation"

echo "[check] rule: extern(c) only in std/libc/** and approved hosted providers"
extern_hits="$(rg -n "extern\\(c\\)" std --glob '*.wave' || true)"
if [[ -n "$extern_hits" ]]; then
  non_libc_extern="$(printf '%s\n' "$extern_hits" | rg -v '^std/(libc/|sys/(linux|macos|freebsd)/(resolver|interfaces|vector_io|event)\.wave:)' || true)"
  if [[ -n "$non_libc_extern" ]]; then
    echo "[FAIL] extern(c) found outside approved C ABI providers:"
    printf '%s\n' "$non_libc_extern"
    failed=1
  fi
fi

echo "[check] rule: std/** must not import std::libc::*"
libc_import_hits="$(rg -n 'import\("std::libc::' std --glob '*.wave' || true)"
if [[ -n "$libc_import_hits" ]]; then
  non_libc_imports="$(printf '%s\n' "$libc_import_hits" | rg -v '^std/libc/' || true)"
  if [[ -n "$non_libc_imports" ]]; then
    echo "[FAIL] std::libc import found outside std/libc:"
    printf '%s\n' "$non_libc_imports"
    failed=1
  fi
fi

echo "[check] rule: Wave sources must not use retired let declarations"
let_hits="$(rg -n '(^|[({;])[[:space:]]*let([[:space:]]+mut)?[[:space:]]+[[:alpha:]_][[:alnum:]_]*[[:space:]]*:' . --glob '*.wave' || true)"
if [[ -n "$let_hits" ]]; then
  echo "[FAIL] retired let declaration found:"
  printf '%s\n' "$let_hits"
  failed=1
fi

if [[ "$failed" -ne 0 ]]; then
  echo "[result] FAILED"
  exit 1
fi

echo "[result] OK"
