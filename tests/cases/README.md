# Wave end-to-end test layout

Cases are grouped by the platform contract they exercise. Every configured directory contains
at least ten numbered entry points starting at `test1.wave`. `cases.toml` is the source of truth
for supported and planned target combinations and for whether each target enters CI.

- `shared/`: operating-system and architecture independent cases, automatically selected for
  every target.
- `shared/<arch>/`: architecture-specific cases shared by multiple operating systems. Native
  architectures exercise inline assembly; WebAssembly directories exercise their pointer-width
  and virtual-ISA contracts.
- `<os>/<arch>/`: cases that require that operating-system and architecture pair, or compile
  artifacts for that target.
- `freestanding/<arch>/`: target-only freestanding artifact checks.

Running `python3 tools/run_tests.py` selects the current native target from `cases.toml`.
Use `--target-id` for another enabled target, or repeatable `--suite` arguments to select an
explicit layout, for example:

```sh
python3 tools/run_tests.py --target-id linux-amd64
python3 tools/run_tests.py --suite shared --suite linux/amd64
```

Metadata comments remain only for execution contracts such as compile-only artifacts, stdin,
UDP fixtures, expected exit codes, and server probes. Platform selection belongs in the path.
Target suites are derived automatically as `shared`, `shared/<arch>`, and `<os>/<arch>`.
Architecture and platform directories contain only ISA, ABI, runtime, or OS-specific contracts;
portable language behavior belongs in `shared/` and is never repeated in a target allowlist.

## Manifest controls

- `[supported]`: enables an OS/architecture cell for local selection and execution details.
- `[ci]`: adds an enabled OS/architecture cell to the matrix generated for `cases.yml`.
- `executor`: selects `native`, `compile`, `qemu`, `wasm`, or `none` execution.
- `exclude`: removes an exceptional exact `tests/cases`-relative test name from a target's
  automatically derived suites. Prefer execution metadata or a more accurate directory first.

To reserve a future platform, add the same OS/architecture cell as `false` to both tables and
create its `<os>/<arch>/` directory. Flip `supported` to `true` and add one `[[target]]` execution
table when the backend becomes available; flip `ci` independently when a CI runner is ready.
Android and iOS are reserved this way until their NDK/SDK, linker, runtime, and ABI contracts are
implemented; their cases are still syntax-checked on every cases workflow run.
`wasm64-unknown-unknown` is enabled as Wave's preferred WebAssembly width. `wasm64-wasip1`
remains planned until a stable 64-bit WASI Preview 1 ABI and host runtime are available.
The roadmap also reserves Linux suites for LoongArch64 (`loong64`), SiPearl Rhea
(`rhea`, AArch64), SHAKTI, XiangShan, and T-Head (`shakti`, `xiangshan`, and
`t-head`, RISC-V). `freestanding/k-riscv` represents the Korean K-RISC-V
processor project—not the standard RISC-V cryptography `K` extension—and is
reserved conservatively because its Wave/Whale OS and ABI contract has not been
defined yet. These
processor/project names do not claim new base ISAs or current backend support;
their cells remain disabled until Wave and Whale implement the contracts.
The deterministic baseline generator fills new suites to ten Wave cases without overwriting
existing tests:

```sh
python3 tools/populate_case_matrix.py
python3 tools/case_manifest.py validate
python3 tools/case_manifest.py github-output
```
