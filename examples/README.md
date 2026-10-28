# Wave examples

Run commands below from the repository root. While developing, replace `wavec` with
`target/debug/wavec` built from this checkout. `wavec check PATH` does not execute the
program. [Standard-library examples](std/README.md) retain their own directory and guide.

| Source | Focus | Run/check and requirements |
|---|---|---|
| [applications/conundrum.wave](applications/conundrum.wave) | Deterministic algorithm workload | `wavec run examples/applications/conundrum.wave` on a supported hosted target. |
| [applications/doom.wave](applications/doom.wave) | Larger interactive game demo | `wavec check examples/applications/doom.wave`; interactive host terminal; do not run unattended. |
| [applications/game.wave](applications/game.wave) | Interactive terminal game | `wavec check examples/applications/game.wave`; interactive host terminal; do not run unattended. |
| [applications/graph_dfs_bfs_with_queue.wave](applications/graph_dfs_bfs_with_queue.wave) | Graph traversals with a queue | `wavec run examples/applications/graph_dfs_bfs_with_queue.wave` on a supported hosted target. |
| [applications/observability_gateway.wave](applications/observability_gateway.wave) | Larger observability application demo | `wavec run examples/applications/observability_gateway.wave` on a supported hosted target. |
| [basics/hello.wave](basics/hello.wave) | Hello World | `wavec run examples/basics/hello.wave` on a supported hosted target. |
| [basics/if.wave](basics/if.wave) | Conditional statements | `wavec run examples/basics/if.wave` on a supported hosted target. |
| [basics/sorting_search.wave](basics/sorting_search.wave) | Insertion sort and binary search | `wavec run examples/basics/sorting_search.wave` on a supported hosted target. |
| [basics/star.wave](basics/star.wave) | Nested loops and formatted output | `wavec run examples/basics/star.wave` on a supported hosted target. |
| [language/aggregate_constants.wave](language/aggregate_constants.wave) | Nested constant aggregates | `wavec run examples/language/aggregate_constants.wave` on a supported hosted target. |
| [language/control_flow_full.wave](language/control_flow_full.wave) | Control-flow forms | `wavec run examples/language/control_flow_full.wave` on a supported hosted target. |
| [language/generics.wave](language/generics.wave) | Box, Pair, explicit and nested generic types | `wavec run examples/language/generics.wave` on a supported hosted target. |
| [language/modules/arithmetic.wave](language/modules/arithmetic.wave) | Public helper with a private declaration | `wavec check examples/language/modules/arithmetic.wave`; helper module, run modules/main.wave. |
| [language/modules/facade.wave](language/modules/facade.wave) | Public selective re-export | `wavec check examples/language/modules/facade.wave`; helper module, run modules/main.wave. |
| [language/modules/main.wave](language/modules/main.wave) | Multi-file module entry or native C interoperability entry | `wavec run examples/language/modules/main.wave` on a supported hosted target. |
| [language/pointers_arrays.wave](language/pointers_arrays.wave) | Caller-owned mutation through pointers and fixed arrays | `wavec run examples/language/pointers_arrays.wave` on a supported hosted target. |
| [language/string_placeholder_scanner.wave](language/string_placeholder_scanner.wave) | Format placeholder scanner | `wavec run examples/language/string_placeholder_scanner.wave` on a supported hosted target. |
| [language/struct_proto_semantics.wave](language/struct_proto_semantics.wave) | Struct values and inherent proto methods | `wavec run examples/language/struct_proto_semantics.wave` on a supported hosted target. |
| [language/target_declarations.wave](language/target_declarations.wave) | One interface selected by target OS | `wavec run examples/language/target_declarations.wave` on a supported hosted target. |
| [language/type_enum.wave](language/type_enum.wave) | Aliases and integer enums | `wavec run examples/language/type_enum.wave` on a supported hosted target. |
| [platform/c_abi/main.wave](platform/c_abi/main.wave) | Multi-file module entry or native C interoperability entry | Independent check; link the C helper to execute. See commands below. |
| [platform/file_io.wave](platform/file_io.wave) | Hosted file I/O | `wavec check examples/platform/file_io.wave`; hosted filesystem writes during execution. |
| [platform/freestanding/checksum.wave](platform/freestanding/checksum.wave) | Freestanding exported checksum | Check for none-elf; emit object only. See commands below. |
| [platform/tcp.wave](platform/tcp.wave) | TCP program | `wavec check examples/platform/tcp.wave`; native networking/peer required for execution. |
| [platform/udp.wave](platform/udp.wave) | UDP program | `wavec check examples/platform/udp.wave`; native networking/peer required for execution. |
| [platform/wasm_host/module.wave](platform/wasm_host/module.wave) | Small host import/export module | Check/build for wasm32; Node host required. See commands below. |
| [platform/wasm_module.wave](platform/wasm_module.wave) | Existing comprehensive bare-Wasm CI module | Check/build for wasm32; Node host required. See commands below. |
| [platform/wasm_run.wave](platform/wasm_run.wave) | Existing bare-Wasm run fixture | Check/build for wasm32; Node host required. See commands below. |
| [platform/wasm_wasi.wave](platform/wasm_wasi.wave) | Existing WASI filesystem CI command | Check/build for wasm32; Node host required. See commands below. |

## Small projects

```sh
wavec run examples/language/modules/main.wave
wavec run examples/language/generics.wave
wavec run examples/language/pointers_arrays.wave
wavec run examples/basics/sorting_search.wave
wavec check examples/language/target_declarations.wave --target x86_64-unknown-linux-gnu
wavec check examples/language/target_declarations.wave --target aarch64-pc-windows-msvc
```

The modules project demonstrates an alias inside `import("./arithmetic" as arithmetic)`,
selective imports, `pub import` re-export and private declarations. Public Wave names
control module visibility; they do not create a C ABI export.

```sh
clang -c examples/platform/c_abi/helper.c -o /tmp/wave-helper.o
wavec build examples/platform/c_abi/main.wave /tmp/wave-helper.o -o /tmp/wave-c-abi
/tmp/wave-c-abi

wavec check examples/platform/wasm_host/module.wave --target wasm32-unknown-unknown
wavec build examples/platform/wasm_host/module.wave --target wasm32-unknown-unknown -o /tmp/wave-host.wasm
node examples/platform/wasm_host/host.mjs /tmp/wave-host.wasm

wavec check examples/platform/freestanding/checksum.wave --target x86_64-unknown-none-elf
wavec build examples/platform/freestanding/checksum.wave --freestanding --emit=obj --target x86_64-unknown-none-elf -o /tmp/wave-checksum.o
llvm-readobj --file-headers /tmp/wave-checksum.o
llvm-nm --undefined-only /tmp/wave-checksum.o
```

These native C commands were checked on Linux amd64. Other native hosts require their
matching C compiler/object ABI; a Linux object cannot be linked into a Windows program.
The freestanding project ends at object emission and supplies no startup or boot code.
The small Wasm host supplies exactly the imported `env.host_add` function and checks
`wave_answer(40) == 42`, with no npm dependencies.

Cross execution requires the matching target toolchain, sysroot and runtime.
See the [published documentation](https://wave-lang.dev/docs/).
CI smoke scripts use the relocated comprehensive Wasm fixtures in
`platform/`; their behavior is preserved.
