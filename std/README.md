# Wave Standard Library

This is Wave's standard library. This standard library operates independently of Wave's compiler and is not part of the compiler itself.

## License

The Wave standard library in this directory is licensed under the

[Apache License 2.0](LICENSE). It may be modified, redistributed, and embedded

in other products under the terms of that license.

The Wave compiler and other repository components outside `std/` remain

licensed under the repository's [Mozilla Public License 2.0](../LICENSE).

## Dependency Policy

- General-purpose `extern(c)` bindings belong under `std/libc/*`.
- Target providers under `std/sys/*` may use the small set of hosted C ABI
  bindings explicitly approved by `tools/check_std_policy.sh` when the OS does
  not expose an equivalent stable raw syscall contract.
- Modules outside `std/libc/*` must not import `std::libc::*`; portable modules
  depend on the target-independent `std::sys::*` provider surface instead.

## Core Modules

- `std::time`: portable clocks and sleep, normalized durations, UTC calendar
  conversion, and fixed ISO 8601 parsing/formatting.
- `std::env`: cwd and environment lookup helpers.
- `std::path`: allocation-free path utilities.
- `std::math`: checked integer arithmetic, bit operations, IEEE-754 helpers,
  roots, and trigonometry.
- `std::mem`: raw allocation plus checked byte-size, copy, move, comparison,
  alignment, runtime target page sizing, and bounded C-string helpers for
  non-GC code.
- `std::buffer`: checked growable byte storage built on `std::mem`.
- `std::io`: fd-level read/write/seek/copy helpers.
- `std::fs`: basic open/read/write/copy/metadata helpers.
- `fs_read_all` reads into caller-owned byte storage; it does not allocate or return `str`.
- `std::bytes`: endian primitives, borrowed byte views, checked random access,
  and transactional byte readers/writers.
- `std::process`: fork/exec/wait and stdio redirection helpers.
- `std::debug`: lightweight marked logging, value tracing, assertions, and
  fatal diagnostics for development builds.

## Layout

- Public modules are split by role under `std/<name>/*.wave` (for example,
  `std/time/duration.wave` and `std/bytes/cursor.wave`).
- Target-specific providers stay under `std/sys/<os>/<arch>/` when the native
  ABI differs by architecture.
