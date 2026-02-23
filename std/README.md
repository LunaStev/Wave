# Wave Standard Library

This is Wave's standard library. This standard library operates independently of Wave's compiler and is not part of the compiler itself.

## Dependency Policy

- `std/libc/*` is the only place where `extern(c)` bindings are allowed.
- Modules outside `std/libc/*` must not import or rely on libc bindings.
- Non-libc modules should be implemented directly in Wave (or raw syscall layers under `std/sys/linux/*`).

## Core Modules

- `std::time`: sleep and clock helpers.
- `std::env`: cwd and environment lookup helpers.
- `std::path`: allocation-free path utilities.
- `std::mem`: manual memory utilities for non-GC code.
- `std::buffer`: growable byte buffer built on `std::mem`.

## Layout

- High-level module entry points stay as `std/<name>.wave`.
- Implementations are split by role under `std/<name>/*.wave` (example: `std/time/*.wave`).
