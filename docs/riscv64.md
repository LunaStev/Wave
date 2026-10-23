# RISC-V 64 hosted and freestanding workflows

Run these commands from the repository root with a compiler built from this checkout.
Use `target/debug/wavec` in place of `wavec` while developing. LLVM 21, `ld.lld`,
a compatible `riscv64-linux-gnu-gcc` toolchain/sysroot and `qemu-riscv64` are required
for the hosted cross-execution example. Object emission does not need QEMU or target libc.

```sh
wavec print target-list
wavec print target-spec --target riscv64-unknown-linux-gnu --format=json
wavec print sysroot --target riscv64-unknown-linux-gnu
# Set this to a complete target sysroot on your machine.
RV64_SYSROOT=/path/to/complete/riscv64-sysroot
wavec build examples/basics/sorting_search.wave --target riscv64-unknown-linux-gnu --sysroot "$RV64_SYSROOT" -o /tmp/wave-rv64
qemu-riscv64 -L "$RV64_SYSROOT" /tmp/wave-rv64
```

The GNU/Linux default is RV64GC with LP64D. Automatic discovery looks for a complete
cross-toolchain sysroot; inspect `print sysroot` rather than assuming `/usr/riscv64-linux-gnu`
on every distribution. If discovery is empty, install target libc/development libraries
first. To inspect explicit selection without linking, use:

```sh
wavec build examples/basics/sorting_search.wave --target riscv64-unknown-linux-gnu --sysroot "$RV64_SYSROOT" --dry-run
```

The explicit value takes precedence and the dry run displays the selected commands.
Use the same sysroot with QEMU's `-L`; host x86 libraries are not target libraries.
A missing sysroot is a link-configuration failure, not evidence that IR/object emission
is unavailable.

```sh
wavec check examples/platform/freestanding/checksum.wave --target riscv64-unknown-none-elf
wavec build examples/platform/freestanding/checksum.wave --target riscv64-unknown-none-elf --freestanding --emit=obj -o /tmp/wave-checksum-rv64.o
llvm-readobj --file-headers /tmp/wave-checksum-rv64.o
llvm-nm --undefined-only /tmp/wave-checksum-rv64.o
```

Freestanding RV64 defaults to the compiler's RV64GC/LP64D contract. This example ends
at a relocatable object: it supplies no entry point, startup assembly, linker script,
firmware integration or QEMU-system boot. Optimized aggregate/loop lowering can require
memory primitives; the wider provider contract is tracked in [#918](https://github.com/wavefnd/Wave/issues/918).
The startup/system-smoke work is [#371](https://github.com/wavefnd/Wave/issues/371).

RV32, musl, vector V, big-endian RISC-V and embedded ABI profiles are not covered by
these commands. Check [the exact target table](targets.md) before selecting a triple.

The hosted example was executed with QEMU and a disposable glibc 2.36 sysroot;
this is user-mode emulation, not native RV64 execution. The local cross-GCC installation
alone had an empty sysroot, which correctly failed runtime validation until libc was supplied.
