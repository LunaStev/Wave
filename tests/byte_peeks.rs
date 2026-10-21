// SPDX-License-Identifier: MPL-2.0
use std::{fs, path::PathBuf, process::Command};

#[test]
fn fixed_width_and_leb_peeks_preserve_cursor_and_outputs() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let compiler = env!("CARGO_BIN_EXE_wavec");
    let result = Command::new(compiler).args(["print", "default-target"]).output().unwrap();
    assert!(result.status.success());
    let host = String::from_utf8(result.stdout).unwrap().trim().to_owned();
    let directory = std::env::temp_dir().join(format!("wave-byte-peeks-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    for target in
        [host.as_str(), "wasm32-unknown-unknown", "wasm32-wasip1", "wasm64-unknown-unknown"]
    {
        if llvm::codegen::target::target_spec_for_triple(target).is_none() {
            continue;
        }
        if target.starts_with("wasm")
            && !Command::new("node").arg("--version").output().is_ok_and(|r| r.status.success())
        {
            eprintln!("byte peek execution for {target} requires Node");
            continue;
        }
        for opt in ["-O0", "-O2"] {
            let result = Command::new(compiler)
                .current_dir(&directory)
                .arg("--std-root")
                .arg(root.join("std"))
                .arg("build")
                .arg(root.join("tests/fixtures/byte_peeks.wave"))
                .args(["--target", target, opt, "--run", "--out-dir"])
                .arg(directory.join(format!("{target}-{opt}")))
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{target} {opt}: {}\n{}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
        }
    }
    fs::remove_dir_all(directory).unwrap();
}
