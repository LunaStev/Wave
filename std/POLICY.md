# Wave STD Policy

This document defines non-negotiable rules for the Wave standard library (`std/`).

## 1) Layering Rules

- `std/libc/**` is the only directory allowed to declare `extern(c)`.
- Outside `std/libc/**`, modules must use Wave code and/or `std/sys/**` raw syscall wrappers.
- High-level modules (`std/io`, `std/fs`, `std/net`, `std/process`, `std/mem`, etc.) must not import `std::libc::*`.

## 2) Syntax Rules

- `var` declarations are banned in `std/**`.
- Use `let` for immutable values.
- Use `let mut` for mutable values.

## 3) Runtime Contract Rules

- `std/sys/**` should preserve raw syscall style: success `>= 0`, errors as negative `-errno`.
- High-level `std/**` modules may define additional library-level error codes when needed.

## 4) Compatibility Rules

- All `std/` code must remain compatible with the `v0.1.8-pre-beta` compiler baseline.
- Avoid patterns known to break older codegen paths (for example, complex index expressions in single brackets).

## 5) Automated Check

Run:

```bash
./tools/check_std_policy.sh
```

The checker validates:

- `extern(c)` appears only under `std/libc/**`
- no `std::libc::*` import outside `std/libc/**`
- no `var` usage in `std/**`
