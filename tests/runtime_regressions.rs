//! Native standard-library regressions with bounded process-tree execution.
#![cfg(target_os = "linux")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct FixtureDirectory(PathBuf);

impl Drop for FixtureDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let destination = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), destination).unwrap();
        }
    }
}

fn run_native_fixture(name: &str) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let compiler = PathBuf::from(env!("CARGO_BIN_EXE_wavec"));
    let target = Command::new(&compiler)
        .args(["print", "default-target"])
        .output()
        .unwrap();
    assert!(target.status.success());
    let host = String::from_utf8(target.stdout).unwrap().trim().to_owned();
    if llvm::codegen::target::target_spec_for_triple(&host).is_none() {
        eprintln!("native runtime fixture skipped: LLVM target {host} is disabled");
        return;
    }
    let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
    let directory = FixtureDirectory(
        std::env::temp_dir().join(format!("wave-runtime-{}-{sequence}", std::process::id())),
    );
    fs::create_dir_all(&directory.0).unwrap();
    let home = directory.0.join("home");
    copy_tree(&root.join("std"), &home.join(".wave/lib/wave/std"));
    let executable = directory.0.join("fixture");
    let output = Command::new(&compiler)
        .env("HOME", &home)
        .args(["build", "--target", &host])
        .arg(root.join("tests/fixtures").join(name))
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{name} compile: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // The fixture forks children: bound and reap the whole tree on failure.
    let output = Command::new("python3")
        .current_dir(&root)
        .args([
            "-c",
            "import sys; from tools.process_tree import run_process; r = run_process([sys.argv[1]], timeout=20); raise SystemExit(r.returncode)",
        ])
        .arg(executable)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{name} runtime: {}\n{}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn child_standard_streams_preserve_shared_and_cyclic_descriptors() {
    run_native_fixture("process/descriptor_remapping.wave");
}
