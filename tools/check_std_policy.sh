#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

failed=0

echo "[check] std policy validation"

echo "[check] rule: extern(c) only in std/libc/**"
extern_hits="$(rg -n "extern\\(c\\)" std --glob '*.wave' || true)"
if [[ -n "$extern_hits" ]]; then
  non_libc_extern="$(printf '%s\n' "$extern_hits" | rg -v '^std/libc/' || true)"
  if [[ -n "$non_libc_extern" ]]; then
    echo "[FAIL] extern(c) found outside std/libc:"
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

echo "[check] rule: std/** must not use 'var' declarations"
var_hits="$(rg -n "\\bvar\\b" std --glob '*.wave' || true)"
if [[ -n "$var_hits" ]]; then
  echo "[FAIL] var declaration found in std:"
  printf '%s\n' "$var_hits"
  failed=1
fi

if [[ "$failed" -ne 0 ]]; then
  echo "[result] FAILED"
  exit 1
fi

echo "[result] OK"
