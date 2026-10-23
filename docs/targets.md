# Compiler target support and validation

This table lists each exact registered triple from a compiler built with the default
LLVM features. `--no-default-features` can restrict the available LLVM backends.
The Whale adapter remains Linux amd64/O0 verified textual IR; this LLVM table does
not imply native Whale execution or backend parity.

Inspect the installed compiler with:

```sh
wavec print target-list
wavec print target-spec --target riscv64-unknown-linux-gnu --format=json
wavec print sysroot --target riscv64-unknown-linux-gnu
```

| Exact triple | Architecture | Format | Runtime/link requirements | Configured CI execution |
|---|---|---|---|---|
| `aarch64-apple-darwin` | aarch64 | macho | Apple SDK/system libraries and Darwin linker | native build and case lanes |
| `aarch64-pc-windows-msvc` | aarch64 | coff | MSVC: Windows SDK, UCRT/MSVC libraries, native COFF linker | native build and case lanes |
| `aarch64-unknown-freebsd` | aarch64 | elf | Compatible FreeBSD base/sysroot and ELF linker | compile/object checks only |
| `aarch64-unknown-linux-gnu` | aarch64 | elf | Target glibc/startup objects, compatible ELF linker and sysroot | native build and case lanes |
| `aarch64-unknown-none-elf` | aarch64 | elf | Freestanding; caller supplies startup/link script/runtime, object output needs no sysroot | IR/object emission coverage; no dedicated native runtime lane |
| `loongarch64-unknown-linux-gnu` | loongarch64 | elf | Target glibc/startup objects, compatible ELF linker and sysroot | QEMU user-mode case lane |
| `riscv64-unknown-freebsd` | riscv64 | elf | Compatible FreeBSD base/sysroot and ELF linker | compile/object checks only |
| `riscv64-unknown-linux-gnu` | riscv64 | elf | Target glibc/startup objects, compatible ELF linker and sysroot | QEMU user-mode case lane |
| `riscv64-unknown-none-elf` | riscv64 | elf | Freestanding; caller supplies startup/link script/runtime, object output needs no sysroot | IR/object emission coverage; no dedicated native runtime lane |
| `wasm32-unknown-unknown` | wasm32 | wasm | wasm-ld and host env imports; no hosted OS runtime | Node host/WASI runtime lane |
| `wasm32-wasip1` | wasm32 | wasm | wasm-ld; WASI Preview 1 host and preopens | Node host/WASI runtime lane |
| `wasm64-unknown-unknown` | wasm64 | wasm | wasm-ld, Memory64 host and env imports | Node host/WASI runtime lane |
| `x86_64-apple-darwin` | x86_64 | macho | Apple SDK/system libraries and Darwin linker | native build and case lanes |
| `x86_64-pc-windows-msvc` | x86_64 | coff | MSVC: Windows SDK, UCRT/MSVC libraries, native COFF linker | native build and case lanes |
| `x86_64-unknown-freebsd` | x86_64 | elf | Compatible FreeBSD base/sysroot and ELF linker | FreeBSD VM case lane |
| `x86_64-unknown-linux-gnu` | x86_64 | elf | Target glibc/startup objects, compatible ELF linker and sysroot | native build and case lanes |
| `x86_64-unknown-none-elf` | x86_64 | elf | Freestanding; caller supplies startup/link script/runtime, object output needs no sysroot | compile/object checks only |

CI entries describe configured lanes in `tools/ci/targets.py` and `plans.json`, not a
promise that every std operation is executed. A cross-object emission is not native
execution. See [std system capabilities](../std/SYSTEMS.md) for provider limitations.

RV64 Linux defaults to RV64GC/LP64D. LoongArch Linux defaults to LP64D; LP64S
is also supported and LP64F object emission is available, but hosted LP64F linking
is rejected because the supported glibc runtime does not provide that ABI.

An explicit `--sysroot` takes precedence over automatic cross-toolchain discovery.
Hosted cross-linking requires matching target libraries. Freestanding output does
not supply boot code or a kernel runtime. [RV64 workflows](riscv64.md) show both.

Planned contracts include musl, RV32/i686, Android, iOS, Fuchsia, OpenBSD, NetBSD,
DragonFly, and 64-bit WASI. They are not advertised as supported by this table.
Feature-specific CPU profiles and vector extensions require their own validation.
Refresh this table from the compiler registry when adding targets; retain a separate
execution level for each exact triple rather than inferring it from the architecture.
