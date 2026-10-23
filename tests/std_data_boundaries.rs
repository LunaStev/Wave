// SPDX-License-Identifier: MPL-2.0
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Case(PathBuf);

impl Case {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "wave-std-data-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
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

fn fixture(name: &str) -> PathBuf {
    root().join("tests/fixtures/std_data").join(name)
}

fn checked(command: &mut Command) {
    let result = command.output().unwrap_or_else(|error| panic!("{command:?}: {error}"));
    assert!(
        result.status.success(),
        "{command:?}: {}\n{}\n{}",
        result.status,
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

fn supported(target: &str) -> bool {
    llvm::codegen::target::target_spec_for_triple(target).is_some()
}

fn host_target() -> String {
    let os = match std::env::consts::OS {
        "macos" => "apple-darwin",
        "windows" => "pc-windows-msvc",
        "freebsd" => "unknown-freebsd",
        _ => "unknown-linux-gnu",
    };
    format!("{}-{os}", std::env::consts::ARCH)
}

fn run(case: &Case, source: &Path, target: &str, opt: &str) {
    checked(
        case.compiler()
            .arg("build")
            .arg(source)
            .args(["--target", target, opt, "--run", "--out-dir"])
            .arg(case.0.join(format!("{target}-{opt}"))),
    );
}

#[test]
fn bounded_std_data_preserves_bits_ranges_and_failure_outputs() {
    let case = Case::new();
    let mut targets = vec![host_target()];
    if Command::new("node").arg("--version").output().is_ok_and(|result| result.status.success()) {
        targets.extend([
            "wasm32-unknown-unknown".to_owned(),
            "wasm32-wasip1".to_owned(),
            "wasm64-unknown-unknown".to_owned(),
        ]);
    }
    for target in targets {
        if !supported(&target) {
            continue;
        }
        for name in ["core.wave", "float.wave", "address.wave", "path.wave"] {
            // Path's existing NUL scans may optimize into a host strlen import;
            // its target-policy regressions execute on native hosts instead.
            if target.starts_with("wasm") && name == "path.wave" {
                continue;
            }
            for opt in ["-O0", "-O2"] {
                run(&case, &fixture(name), &target, opt);
            }
        }
    }
}

#[test]
fn unc_paths_preserve_complete_volume_on_windows_targets() {
    let case = Case::new();
    for target in ["x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"] {
        if !supported(target) {
            continue;
        }
        for opt in ["-O0", "-O2"] {
            if target == host_target() {
                run(&case, &fixture("path.wave"), target, opt);
            } else {
                checked(
                    case.compiler()
                        .arg("build")
                        .arg(fixture("path.wave"))
                        .args(["--target", target, opt, "--emit=obj", "-o"])
                        .arg(case.0.join("path.o")),
                );
            }
        }
    }
}

#[test]
fn read_to_end_real_files_keep_full_and_exact_fit_allocations() {
    let target = host_target();
    if !supported(&target) {
        return;
    }
    let case = Case::new();
    fs::write(case.0.join("empty.dat"), []).unwrap();
    fs::write(case.0.join("exact.dat"), (0..64).collect::<Vec<u8>>()).unwrap();
    fs::write(case.0.join("extra.dat"), (0..65).collect::<Vec<u8>>()).unwrap();
    fs::write(case.0.join("chunks.dat"), (0..9000).map(|i| (i % 251) as u8).collect::<Vec<_>>())
        .unwrap();
    for opt in ["-O0", "-O2"] {
        run(&case, &fixture("read_files.wave"), &target, opt);
    }
}

#[test]
fn read_to_end_probe_growth_counts_and_injected_failures() {
    let case = Case::new();
    let implementation = fs::read_to_string(root().join("std/fs/file.wave")).unwrap();
    let start = implementation.find("pub fun read_to_end(").unwrap();
    let end = implementation[start..].find("\n// Replaces path").unwrap() + start;
    // Exercise the production function with controlled OS/allocation outcomes,
    // rather than relying on nondeterministic host OOM or read failures.
    let body = implementation[start..end]
        .replace("fs_open_read(", "_probe_open(")
        .replace("io_read(", "_probe_read(")
        .replace("io_close(", "_probe_close(")
        .replace("buffer_reserve(", "_probe_reserve(");
    let source = case.0.join("read_mock.wave");
    fs::write(
        &source,
        format!("{}\n{body}", fs::read_to_string(fixture("read_mock.wave")).unwrap()),
    )
    .unwrap();
    for target in [host_target(), "wasm32-unknown-unknown".to_owned()] {
        if !supported(&target)
            || (target.starts_with("wasm")
                && !Command::new("node")
                    .arg("--version")
                    .output()
                    .is_ok_and(|result| result.status.success()))
        {
            continue;
        }
        for opt in ["-O0", "-O2"] {
            run(&case, &source, &target, opt);
        }
    }
}
