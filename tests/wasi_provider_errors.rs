// SPDX-License-Identifier: MPL-2.0
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_CASE: AtomicU64 = AtomicU64::new(0);

struct Case(PathBuf);

impl Case {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "wave-wasi-provider-errors-{}-{}",
            std::process::id(),
            NEXT_CASE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&directory).unwrap();
        Self(directory)
    }

    fn compiler(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_wavec"));
        command.current_dir(&self.0).arg("--std-root").arg(root().join("std"));
        command
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn checked(command: &mut Command) {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?}: {}\n{}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn available(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok_and(|output| output.status.success())
}

#[test]
fn wasi_provider_failures_keep_the_wasi_errno_domain() {
    let target = "wasm32-wasip1";
    if llvm::codegen::target::target_spec_for_triple(target).is_none() {
        eprintln!("WASI errno execution skipped: WebAssembly target is disabled");
        return;
    }
    if !available("node") || !available("wasm-ld") {
        eprintln!("WASI errno execution skipped: Node and wasm-ld are required");
        return;
    }
    let case = Case::new();
    let fixtures = root().join("tests/fixtures/wasi_provider_errors");
    for optimization in ["-O0", "-O2"] {
        let directory = case.0.join(optimization);
        checked(
            case.compiler()
                .arg("build")
                .arg(fixtures.join("provider.wave"))
                .args(["--target", target, optimization, "--emit=obj", "--out-dir"])
                .arg(&directory),
        );
        let module = directory.join("errors.wasm");
        let mut link = Command::new("wasm-ld");
        link.args(["--no-entry", "--export-memory"]);
        for function in [
            "invalid_arguments",
            "unsupported_calls",
            "file_stat",
            "file_seek",
            "clock_value",
            "environment_value",
            "read_value",
            "write_value",
        ] {
            link.arg(format!("--export={function}"));
        }
        checked(link.arg(directory.join("provider.o")).arg("-o").arg(&module));
        checked(Command::new("node").arg(fixtures.join("host.cjs")).arg(module));
    }
}

#[test]
fn wasm_allocator_errors_follow_the_target_without_losing_allocations() {
    if !available("node") {
        eprintln!("WebAssembly allocator execution skipped: Node is required");
        return;
    }
    let case = Case::new();
    let fixture = root().join("tests/fixtures/wasi_provider_errors/allocator.wave");
    for target in ["wasm32-wasip1", "wasm32-unknown-unknown", "wasm64-unknown-unknown"] {
        if llvm::codegen::target::target_spec_for_triple(target).is_none() {
            continue;
        }
        for optimization in ["-O0", "-O2"] {
            checked(
                case.compiler()
                    .arg("build")
                    .arg(&fixture)
                    .args(["--target", target, optimization, "--run", "--out-dir"])
                    .arg(case.0.join(format!("{target}-{optimization}"))),
            );
        }
    }
}
