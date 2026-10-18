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
            "wave-darwin-secondary-{}-{}",
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

#[test]
fn darwin_arm64_syscalls_declare_both_kernel_return_registers() {
    let target = "aarch64-apple-darwin";
    if llvm::codegen::target::target_spec_for_triple(target).is_none() {
        eprintln!("Darwin arm64 syscall IR check skipped: AArch64 is disabled");
        return;
    }
    let case = Case::new();
    let source = case.0.join("syscalls.wave");
    fs::write(
        &source,
        r#"import("std::sys::macos::arm64::syscall")::{syscall0, syscall1, syscall2, syscall3, syscall4, syscall5, syscall6};
        pub fun probe(id: i64, value: i64) -> i64 {
            return syscall0(id) + syscall1(id, value) + syscall2(id, value, value)
                + syscall3(id, value, value, value) + syscall4(id, value, value, value, value)
                + syscall5(id, value, value, value, value, value)
                + syscall6(id, value, value, value, value, value, value);
        }
        fun main() -> i32 { return 0; }
        "#,
    )
    .unwrap();
    for optimization in ["-O0", "-O1", "-O2", "-O3"] {
        let directory = case.0.join(optimization);
        checked(
            case.compiler()
                .arg("build")
                .arg(&source)
                .args(["--target", target, optimization, "--emit=ir,obj", "--out-dir"])
                .arg(&directory),
        );
        let ir = fs::read_to_string(directory.join("syscalls.ll")).unwrap();
        let traps: Vec<_> = ir.lines().filter(|line| line.contains("svc #0x80")).collect();
        assert!(traps.len() >= 7, "expected all seven syscall wrappers: {ir}");
        for trap in traps {
            assert!(trap.contains("={x0}") && trap.contains("={x1}"), "{optimization}: {trap}");
        }
    }
}

#[test]
fn darwin_allocations_retain_runtime_sizes_across_syscalls() {
    let case = Case::new();
    let fixture = root().join("tests/fixtures/native_providers/darwin_secondary.wave");
    for target in ["x86_64-apple-darwin", "aarch64-apple-darwin"] {
        if llvm::codegen::target::target_spec_for_triple(target).is_none() {
            continue;
        }
        let native = cfg!(target_os = "macos")
            && ((cfg!(target_arch = "aarch64") && target.starts_with("aarch64"))
                || (cfg!(target_arch = "x86_64") && target.starts_with("x86_64")));
        for optimization in ["-O0", "-O1", "-O2", "-O3"] {
            let directory = case.0.join(format!("{target}-{optimization}"));
            let mut compiler = case.compiler();
            compiler.arg("build").arg(&fixture).args([
                "--target",
                target,
                optimization,
                "--emit=ir,obj",
                "--out-dir",
            ]);
            compiler.arg(&directory);
            checked(&mut compiler);
            assert!(directory.join("darwin_secondary.o").is_file());
            if native {
                let executable = directory.join("allocations");
                checked(
                    case.compiler()
                        .arg("build")
                        .arg(&fixture)
                        .args(["--target", target, optimization, "--emit=bin", "-o"])
                        .arg(&executable),
                );
                checked(&mut Command::new(executable));
            }
        }
    }
}
