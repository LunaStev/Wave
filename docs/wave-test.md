# Wave case metadata

Cases live under `tests/cases/shared`, `shared/<arch>` or `<os>/<arch>`.
A multi-file case uses `testN/main.wave`. Suite numbers must be contiguous,
starting at 1. The OS/architecture directories and `cases.toml` select platforms;
`host-os` and `host-arch` metadata are rejected. Keep portable language and
algorithm checks in `shared`, ABI checks in `shared/<arch>`, and OS calls in the
OS suite. Support and CI booleans describe different things: planned cells are
not validated targets merely because they have source files.

Put comma-separated `// wave-test: key=value` lines before the first declaration.
Blank lines and ordinary comments may appear between metadata lines. Keys cannot
repeat; empty values, unknown keys and invalid combinations are errors.

| Key | Default | Contract |
| --- | --- | --- |
| `mode` | `run` | `run`, `check`, or `build` |
| `runner` | `native` | `native`, `server`, or `compile`; `build` and `compile` must be paired |
| `expected-exit` | `0` | Integer process exit status; a failed compiler never satisfies a runtime exit contract |
| `stdin` | absent | One inline input line; the runner appends LF |
| `stdin-file` | absent | UTF-8 fixture input, with no extra newline; mutually exclusive with `stdin` |
| `stdout-file`, `stderr-file` | absent | Exact UTF-8 output fixtures; an empty file asserts empty output |
| `env-file` | absent | UTF-8 JSON object of string keys/values overlaid on the child environment; only the declared keys change |
| `udp-input` | `false` | Repeated local UDP input on port 8080 during a native run; the sender stops when that case finishes |
| `runtime` | `portable` | `portable` permits compatible target runtimes; `native` limits execution to native/QEMU; `compile` excludes runtime selection |
| `requires` | empty | Pipe-separated host capabilities: `stdio`, `filesystem`, `environment`, `clock`, `sockets`, `process` |
| `target` | current target | Explicit compiler triple for a compile case |
| `emit` | `obj` | Compile output: `obj`, `asm`, `ir`, or `bc` |
| `freestanding` | `false` | Add `--freestanding` to the compile command |
| `object-arch` | absent | ELF `amd64`/`x86_64`, `arm64`/`aarch64`, `riscv64`, `loong64`/`loongarch64`, or WebAssembly `wasm32`/`wasm64` |
| `object-bits` | absent | Expected 32/64-bit ELF class or WebAssembly memory address width |
| `riscv-float-abi` | absent | `lp64`, `lp64f`, `lp64d`; requires `object-arch=riscv64` |
| `loongarch-float-abi` | absent | `lp64s`, `lp64f`, `lp64d`; requires `object-arch=loongarch64` |
| `asm-contains`, `asm-not-contains` | empty | Pipe-separated instruction tokens, ignoring comments, directives and labels |

Input/output/environment fixtures and `udp-input` require `mode=run, runner=native`.
`server` requires `mode=run` and uses the owned HTTP challenge/readiness protocol
rather than an expected blocking timeout. All compile-specific keys require
`runner=compile`. Object contracts require `emit=obj`; assembly contracts require
`emit=asm`. Artifact assertions require `expected-exit=0`. Boolean values accept
`true/false`, `yes/no`, and `1/0` without case sensitivity.

Fixture paths are relative to `tests/fixtures`. Use `/` separators; absolute
paths, Windows drives, `..`, non-normalized paths and links escaping that root
are rejected. Files must exist and decode as UTF-8. Python text-mode universal
newline handling normalizes CRLF and CR to LF in both fixtures and process
streams; all other characters, blank lines, trailing spaces and final-newline
presence are significant. Environment fixture names must be nonempty and exclude
`=` and NUL; values are strings without NUL. Empty values are allowed. Invalid JSON
or non-string values fail before launching the case. Use unique test key prefixes
and `requires=environment` for environment-provider boundary cases. Output mismatches include a bounded unified diff and
fail the case with `phase=output` in its JSON report. Invalid UTF-8 output fails
when an output fixture is declared.

For example, a portable output contract can use:

```wave
// wave-test: runtime=portable, requires=stdio
// wave-test: stdout-file=wave-test/hello.stdout, stderr-file=wave-test/empty.stderr
fun main() {
    println("Hello World");
}
```

A cross-compiled artifact can use:

```wave
// wave-test: mode=build, runner=compile, target=riscv64-unknown-linux-gnu
// wave-test: object-arch=riscv64, object-bits=64, riscv-float-abi=lp64d
fun main() -> i32 {
    return 0;
}
```

The runner validates ELF relocatable headers and ABI flags without external
tools. For a resolved `wasm32-*` or `wasm64-*` target, every emitted object is
also checked for WebAssembly magic/version and consistent memory/import/Memory64
address-width information. A successful compiler exit with the wrong artifact
kind or address width fails the case.

Run a native suite with `python3 tools/run_tests.py --wavec target/debug/wavec`.
Use `--target-id linux-amd64` for manifest selection, or repeat `--suite shared`
and `--suite linux/amd64` for explicit suites. These modes cannot be combined.
Repeat `--only shared/test2.wave` or `--skip shared/test2.wave` to filter cases.
Unknown `--only` names are errors. `--report-json /tmp/cases.json` preserves the
selection, original case names, exits, failures, skips and timeouts; report paths
may not overwrite compiler/test/std inputs.

Local runs are unsharded by default. `--shard-count 2 --shard-index 0` runs one
zero-based shard after filtering. Stable suite/numeric-case order is partitioned
by index, so every selected case belongs to exactly one shard. CI expands each
target into two shards and records shard identity in check/artifact/report names;
QEMU/WebAssembly runtime selection uses the same partition rule. Compare stage
`duration_seconds` values between identical target/revision runs to measure CI
wall time; provisioning and compiler build time are separate from case time.

`python3 tools/case_manifest.py runtime-sources wasm-wasi` lists runtime-eligible
sources. Selection uses metadata and each target's declared capabilities, not
source-text or filename guesses. Cases needing unavailable services still compile
in their target suite. Native-only cases are excluded on WebAssembly. Compile-only, interactive input,
UDP/server and expected-nonzero cases are excluded from unattended cross-runtime lists.
Compilation, capability exclusions and actual execution are separate evidence;
a missing runtime capability is never reported as a passed execution.
