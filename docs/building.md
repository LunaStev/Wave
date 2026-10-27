# Building Wave with Meson

Meson defines compiler build options, dependencies, test suites and distribution
tasks. Cargo owns Rust dependency resolution, features, profiles and the existing
`build.rs` scripts. LLVM remains the default backend; the compiler's `--whale`
interface and emitted target matrix are unchanged.

Use Python 3.11+, the Rust version in `tools/ci/toolchains.json`, LLVM 21 and the
existing platform prerequisites described in [CONTRIBUTING](../CONTRIBUTING.md).
Install the pinned Meson/Ninja tools into your development environment:

```sh
python3 -m pip install -r tools/ci/requirements-build.txt
meson setup build -Dllvm_prefix=/path/to/llvm21
meson compile -C build --jobs 2
build/wavec --version
```

On Windows, use `python`, `build\wavec.exe`, the native MSVC toolchain and the
existing native LLVM SDK provisioner. An empty `llvm_prefix` option inherits
`LLVM_SYS_211_PREFIX`, `LLVM_CONFIG_PATH` and the existing `WAVE_LLVM_*` inputs.
Meson does not install SDKs during ordinary configuration or compilation.

Before installation, select the checkout's std explicitly for development checks:
`build/wavec --std-path "$PWD/std" check path/to/program.wave`. Packaged and
installed compilers use their adjacent std without this override.

Cargo uses `--locked`, the pinned Rustup toolchain and two jobs. The optional
`-Dtoolchain=1.89` selects a locally installed alias only if `rustc` reports the
exact pinned `1.89.0` version. It cannot select a newer compiler. Incremental
decisions stay with Cargo: every Meson compile invokes Cargo so source changes,
deleted outputs and build-script inputs cannot be hidden by a stale Meson copy.

## Compiler hosts and build options

The default `compiler_target=native` keeps Cargo's `target/release/wavec` output.
Use a separate build directory for each explicit compiler host or feature set:

```sh
meson setup build-arm64 -Dcompiler_target=aarch64-unknown-linux-gnu
meson compile -C build-arm64 rust-target
meson compile -C build-arm64
meson setup build-debug -Dbuildtype=debug -Dllvm_features=llvm-target-wasm
meson compile -C build-debug
meson configure build-debug
```

`compiler_target` describes the machine that runs **wavec**, not the target of
the programs compiled by Wave. Explicit hosts retain Cargo outputs under
`target/<triple>/<profile>/wavec`. All 15 previous automation host choices remain
available in `meson.options`, including the existing BSD and experimental host
choices. Availability in that list does not establish native validation on that
OS. Existing compiler output support, including wasm64 and LoongArch64, remains
in the [target matrix](targets.md).

Cross compiler builds still require the corresponding Rust target, linker,
sysroot and matching target LLVM. The existing cross LLVM environment and QEMU
package checks remain required. Meson does not substitute host LLVM for a foreign
runtime. `llvm_features=auto` preserves native Windows selections: core64 on
amd64 and aarch64 on ARM64; other hosts use Cargo defaults. An explicit
`llvm_features` selects the corresponding Cargo feature with default features
disabled. `buildtype` accepts `debug` and `release`.

`cargo_target_dir` defaults to the source checkout's `target/` and can be an
absolute path or a source-relative directory. This keeps existing CI caches and
test-harness compiler paths working. Separate concurrent configurations that use
different feature sets should use different Cargo output directories.

## Tests and CI

```sh
meson test -C build --suite tooling --print-errorlogs
meson test -C build --suite policy --print-errorlogs
meson test -C build --suite rust --print-errorlogs
```

The Rust suite runs the workspace/all-target tests using the selected toolchain,
host and features. Runtime prerequisites still apply when selecting a foreign
compiler host. The tooling suite includes real Meson graph tests with controlled
Cargo fixtures; these do not establish native execution on foreign machines.

The `ci` target retains the existing target-specific plans, timeouts, evidence
reports, SDK provisioning controls and release acceptance checks. Configure a
CI-only graph without requiring Rust or LLVM during case-matrix generation:

```sh
meson setup build-ci -Dcompiler=false -Dci_target=linux-amd64 \
  -Dci_lanes=rust/build-linux-amd64 -Dci_report=ci-report.json
meson compile -C build-ci ci
```

Set `-Dci_provision=true` explicitly to permit the existing provisioning steps.
`ci_family` selects `build`, `test`, `validate` or `package`. Inspect a plan without
executing it with the existing `python3 -m tools.ci.test --target linux-amd64
--lane rust/build-linux-amd64 --plan` command. Publication remains an explicit
release/Nightly policy operation; Meson compilation never publishes a release.

## Packages and installation

```sh
meson compile -C build release
meson setup build-install --prefix="$HOME/.wave"
meson compile -C build-install
meson install -C build-install
```

`release` depends on the Cargo build and `package`. Archives remain at the source
root as `wave-v<VERSION>-<target>.tar.gz` or `.zip`, with the same checksum and
metadata sidecars. Staging remains under `dist/`. Packages retain adjacent
`wavec`, `std/`, `llvm/`, `licenses/` and applicable `crt/` directories, committed
std-only membership, runtime architecture checks and source/std identity.
`package-existing` is the CI staging target for a Cargo output already tested by
preceding stages. Both packaging targets require a release configuration.

Meson installation places the complete adjacent payload under the prefix's
`bindir` (default `bin`), including `bin/wavec` and `bin/std/`. Keep the payload
together: moving only `wavec` loses bundled dependencies. This uses the compiler's
existing adjacent std lookup. The website installers, their selected release,
and legacy `~/.wave/lib/wave/std` lookup remain unchanged.

Use `DESTDIR` for staging; the final prefix is preserved inside that directory:

```sh
DESTDIR=/tmp/wave-staging meson install -C build-install
meson install -C build-install --dry-run
```

Installation validates the complete payload before copying it and preserves
unrelated files in `bindir`. `bindir` must remain inside the prefix. Add that
directory to `PATH`. No root access is needed for a user-owned prefix or DESTDIR.

`meson compile -C build --clean` cleans Meson's outputs. The explicit
`meson compile -C build distclean` also removes checkout-owned `target/`, `dist/`
and recognized release archives, preserving `.tmp/` and unrelated files. The old
Tk build selector is replaced by Meson's `setup`/`configure` options; Tk is no
longer a dependency.
