// This file is part of the Wave language project.
// Copyright (c) 2024–2026 Wave Foundation and contributors
// SPDX-License-Identifier: MPL-2.0

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_CASE: AtomicU64 = AtomicU64::new(0);

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let out = destination.join(entry.file_name());
        if path.is_dir() {
            copy_tree(&path, &out);
        } else {
            fs::copy(path, out).unwrap();
        }
    }
}

struct Case {
    root: PathBuf,
    home: PathBuf,
}
impl Case {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "wave-boundary-{name}-{}-{}",
            std::process::id(),
            NEXT_CASE.fetch_add(1, Ordering::Relaxed)
        ));
        let home = root.join("home");
        copy_tree(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("std"),
            &home.join(".wave/lib/wave/std"),
        );
        Self { root, home }
    }
    fn command(&self) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_wavec"));
        cmd.env("HOME", &self.home)
            .env("NO_COLOR", "1")
            .current_dir(&self.root);
        cmd
    }
    fn run(&self, source: &Path) -> String {
        for opt in ["-O0", "-O2"] {
            success(
                self.command()
                    .arg("build")
                    .arg(source)
                    .arg(opt)
                    .arg("--run")
                    .arg("--emit=ir,obj,bin")
                    .arg("--out-dir")
                    .arg(self.root.join(opt))
                    .output()
                    .unwrap(),
            );
        }
        fs::read_to_string(
            self.root
                .join("-O0")
                .join(source.file_stem().unwrap())
                .with_extension("ll"),
        )
        .unwrap()
    }
}
impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn success(output: Output) {
    assert!(
        output.status.success(),
        "status {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
fn source(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}
fn shared(number: u32) -> String {
    Case::new(&number.to_string()).run(&source(&format!("tests/cases/shared/test{number}.wave")))
}

#[test]
fn contextual_integer_arithmetic_preserves_width_and_signedness() {
    let ir = shared(128);
    assert!(ir.contains("icmp ule i64 %load_value, 4294967264"), "{ir}");
    assert!(!ir.contains("icmp ule i64 %load_value, -32"), "{ir}");
}

#[test]
fn oversized_integer_operands_report_source_errors() {
    let case = Case::new("integer-errors");
    for (index, body) in [
        "4294967296 - 32;",
        "var value: i32 = 4294967296 - 32;",
        "var value: u8 = 256 - 1;",
        "var value: u64 = 0; if (value < (18446744073709551616 - 1)) { return 1; }",
        "var value: i64 = 0; if ((9223372036854775808 - 1) > value) { return 1; }",
        "var value: f64 = 4294967296 / 2;",
    ]
    .iter()
    .enumerate()
    {
        let path = case.root.join(format!("invalid{index}.wave"));
        fs::write(&path, format!("fun main() -> i32 {{ {body} return 0; }}")).unwrap();
        let output = case.command().arg("check").arg(&path).output().unwrap();
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{body}");
        assert!(
            error.contains("does not fit")
                && error.contains("E3001")
                && error.contains(&format!("invalid{index}.wave")),
            "{error}"
        );
    }
}
