// SPDX-License-Identifier: MPL-2.0

use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Directory(PathBuf);

impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "wave-learning-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn compiler() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_wavec"));
    command.current_dir(root()).arg(format!("--std-root={}", root().join("std").display()));
    command
}

fn success(output: Output) {
    assert!(
        output.status.success(),
        "{}\n{}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn each_learning_source_checks_independently_and_private_imports_stay_private() {
    let output =
        Command::new(env!("CARGO_BIN_EXE_wavec")).args(["print", "target-list"]).output().unwrap();
    assert!(output.status.success());
    let targets = String::from_utf8(output.stdout).unwrap();
    let target =
        targets.lines().find(|target| !target.contains("wasm")).unwrap_or("wasm32-unknown-unknown");
    for source in [
        "language/modules/main.wave",
        "language/modules/arithmetic.wave",
        "language/modules/facade.wave",
        "language/generics.wave",
        "language/pointers_arrays.wave",
        "language/target_declarations.wave",
        "basics/sorting_search.wave",
        "platform/c_abi/main.wave",
        "platform/wasm_host/module.wave",
        "platform/freestanding/checksum.wave",
    ] {
        success(
            compiler()
                .args(["check", &format!("examples/{source}"), "--target", target])
                .output()
                .unwrap(),
        );
    }
    let directory = Directory::new();
    std::fs::write(
        directory.0.join("helper.wave"),
        include_str!("../examples/language/modules/arithmetic.wave"),
    )
    .unwrap();
    std::fs::write(
        directory.0.join("main.wave"),
        "import(\"./helper\")::{private_identity}; fun main() {}\n",
    )
    .unwrap();
    let failure = compiler()
        .arg("check")
        .arg(directory.0.join("main.wave"))
        .args(["--target", target])
        .output()
        .unwrap();
    assert!(!failure.status.success());
    assert!(String::from_utf8_lossy(&failure.stderr).contains("private"));
}

#[cfg(all(
    target_os = "linux",
    target_arch = "x86_64",
    any(feature = "llvm-target-x86", feature = "llvm-target-all")
))]
#[test]
fn learning_algorithms_and_both_c_abi_directions_execute() {
    let directory = Directory::new();
    for source in [
        "language/modules/main.wave",
        "language/generics.wave",
        "language/pointers_arrays.wave",
        "language/target_declarations.wave",
        "basics/sorting_search.wave",
    ] {
        for optimization in ["-O0", "-O2"] {
            let executable = directory.0.join("program");
            success(
                compiler()
                    .args(["build", &format!("examples/{source}"), optimization, "-o"])
                    .arg(&executable)
                    .output()
                    .unwrap(),
            );
            success(Command::new(executable).output().unwrap());
        }
    }
    let helper = directory.0.join("helper.o");
    success(
        Command::new("clang")
            .args(["-c", "examples/platform/c_abi/helper.c", "-o"])
            .arg(&helper)
            .current_dir(root())
            .output()
            .unwrap(),
    );
    let executable = directory.0.join("roundtrip");
    success(
        compiler()
            .args(["build", "examples/platform/c_abi/main.wave"])
            .arg(helper)
            .arg("-o")
            .arg(&executable)
            .output()
            .unwrap(),
    );
    success(Command::new(executable).output().unwrap());
}

#[cfg(any(feature = "llvm-target-wasm", feature = "llvm-target-all"))]
#[test]
fn standalone_javascript_host_provides_and_calls_the_wave_abi() {
    let directory = Directory::new();
    let module = directory.0.join("module.wasm");
    success(
        compiler()
            .args([
                "build",
                "examples/platform/wasm_host/module.wave",
                "--target=wasm32-unknown-unknown",
                "-o",
            ])
            .arg(&module)
            .output()
            .unwrap(),
    );
    success(
        Command::new("node")
            .arg(root().join("examples/platform/wasm_host/host.mjs"))
            .arg(module)
            .output()
            .unwrap(),
    );
}

#[cfg(any(feature = "llvm-target-x86", feature = "llvm-target-all"))]
#[test]
fn freestanding_example_is_an_object_with_no_hosted_dependencies() {
    let directory = Directory::new();
    let object = directory.0.join("checksum.o");
    success(
        compiler()
            .args([
                "build",
                "examples/platform/freestanding/checksum.wave",
                "--target=x86_64-unknown-none-elf",
                "--freestanding",
                "--emit=obj",
                "-o",
            ])
            .arg(&object)
            .output()
            .unwrap(),
    );
    assert!(object.is_file());
    let undefined = Command::new("llvm-nm").arg("--undefined-only").arg(object).output().unwrap();
    assert!(undefined.status.success(), "{undefined:?}");
    assert!(undefined.stdout.is_empty(), "{undefined:?}");
}
