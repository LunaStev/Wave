// This file is part of the Wave language project.
// SPDX-License-Identifier: MPL-2.0

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Case(PathBuf);

impl Case {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "wave-output-plan-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("main.wave"), "fun main() -> i32 { return 0; }\n").unwrap();
        Self(root)
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_wavec"));
        command.current_dir(&self.0).env("NO_COLOR", "1").args(["--target", frontend_target()]);
        command
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, bytes).unwrap();
        path
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn frontend_target() -> &'static str {
    static TARGET: OnceLock<String> = OnceLock::new();
    TARGET.get_or_init(|| {
        let output = Command::new(env!("CARGO_BIN_EXE_wavec"))
            .args(["print", "target-list"])
            .output()
            .unwrap();
        successful(&output);
        String::from_utf8(output.stdout).unwrap().lines().next().unwrap().to_owned()
    })
}

fn successful(output: &Output) {
    assert!(
        output.status.success(),
        "{}\n{}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn collision(output: &Output) {
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let diagnostic = utils::wson::parse_json(stderr.trim()).unwrap();
    let error = diagnostic.get("error").unwrap();
    assert_eq!(error.get_str("kind"), Some("usage"));
    assert!(
        error.get_str("message").unwrap().contains("overlaps another planned artifact"),
        "{stderr}"
    );
}

#[test]
fn colliding_ir_and_object_destinations_fail_before_any_artifact_write() {
    for (emit, filename) in [("ir,bin", "main.ll"), ("obj,bin", "main.o")] {
        for existing in [false, true] {
            let case = Case::new();
            let output = case.0.join("out").join(filename);
            if existing {
                case.write(&format!("out/{filename}"), b"previous artifact");
            }
            collision(
                &case
                    .command()
                    .args(["build", "main.wave", "--emit", emit, "--out-dir", "out", "-o"])
                    .arg(&output)
                    .arg("--error-format=json")
                    .output()
                    .unwrap(),
            );
            if existing {
                assert_eq!(fs::read(&output).unwrap(), b"previous artifact");
                assert_eq!(fs::read_dir(case.0.join("out")).unwrap().count(), 1);
            } else {
                assert!(!case.0.join("out").exists());
            }
            assert!(!case.0.join("target").exists());
        }
    }
}

#[test]
fn normalized_nonexistent_destinations_are_compared_without_creating_parents() {
    let case = Case::new();
    collision(
        &case
            .command()
            .args([
                "build",
                "main.wave",
                "--emit=ir,bin",
                "--out-dir",
                "out",
                "-o",
                "out/missing/.././main.ll",
                "--error-format=json",
            ])
            .output()
            .unwrap(),
    );
    assert!(!case.0.join("out").exists());
    assert!(!case.0.join("target").exists());
}

#[cfg(windows)]
#[test]
fn windows_case_aliases_collide_for_existing_and_nonexistent_artifacts() {
    for (directory, existing) in [("out", false), ("out/missing", false), ("out", true)] {
        let case = Case::new();
        let probe = case.write("out/case-probe", b"case probe");
        if !case.0.join("out/CASE-PROBE").exists() {
            eprintln!("case-insensitive fixture unavailable in this temporary directory");
            continue;
        }
        fs::remove_file(probe).unwrap();
        let artifact = case.0.join(directory).join("main.ll");
        if existing {
            case.write(&format!("{directory}/main.ll"), b"previous artifact");
        }
        let alias = format!("{}/MAIN.LL", directory.to_ascii_uppercase());
        collision(
            &case
                .command()
                .args([
                    "build",
                    "main.wave",
                    "--emit=ir,bin",
                    "--out-dir",
                    directory,
                    "-o",
                    &alias,
                    "--error-format=json",
                ])
                .output()
                .unwrap(),
        );
        if existing {
            assert_eq!(fs::read(artifact).unwrap(), b"previous artifact");
        } else {
            assert!(!artifact.exists());
        }
        assert!(!case.0.join("out/missing").exists());
        assert!(!case.0.join("target").exists());
    }
}

#[cfg(windows)]
#[test]
fn windows_normalized_atomic_basenames_collide_before_publication() {
    let target = [
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "riscv64-unknown-linux-gnu",
        "loongarch64-unknown-linux-gnu",
    ]
    .into_iter()
    .find(|target| llvm::codegen::target::target_spec_for_triple(target).is_some());
    let Some(target) = target else {
        return;
    };
    for suffix in [".", " "] {
        for existing in [false, true] {
            let case = Case::new();
            if existing {
                case.write("out/main.ll", b"previous artifact");
            }
            let result = Command::new(env!("CARGO_BIN_EXE_wavec"))
                .current_dir(&case.0)
                .args([
                    "build",
                    "main.wave",
                    "--target",
                    target,
                    "--emit=ir,bin",
                    "--out-dir",
                    "out",
                    "-o",
                    &format!("out/main.ll{suffix}"),
                    "--dry-run",
                    "--error-format=json",
                ])
                .output()
                .unwrap();
            collision(&result);
            if existing {
                assert_eq!(fs::read(case.0.join("out/main.ll")).unwrap(), b"previous artifact");
            } else {
                assert!(!case.0.join("out").exists());
            }
            assert!(!case.0.join("target").exists());
        }
    }
}

#[cfg(windows)]
#[test]
fn windows_msvc_staging_preserves_distinct_verbatim_basenames() {
    let target = ["x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"]
        .into_iter()
        .find(|target| llvm::codegen::target::target_spec_for_triple(target).is_some());
    let Some(target) = target else {
        return;
    };
    for suffix in [".", " "] {
        let case = Case::new();
        let result = Command::new(env!("CARGO_BIN_EXE_wavec"))
            .current_dir(&case.0)
            .args([
                "build",
                "main.wave",
                "--target",
                target,
                "--emit=ir,bin",
                "--out-dir",
                "out",
                "-o",
                &format!("out/main.ll{suffix}"),
                "--dry-run",
            ])
            .output()
            .unwrap();
        successful(&result);
        assert!(!case.0.join("out").exists());
        assert!(!case.0.join("target").exists());
    }
}

#[cfg(windows)]
#[test]
fn windows_case_sensitive_directories_keep_distinct_planned_basenames() {
    let case = Case::new();
    let directory = case.0.join("sensitive");
    fs::create_dir(&directory).unwrap();
    let enabled = Command::new("fsutil.exe")
        .args(["file", "setCaseSensitiveInfo"])
        .arg(&directory)
        .arg("enable")
        .output();
    if !enabled.is_ok_and(|output| output.status.success()) {
        eprintln!(
            "per-directory case-sensitive flag unavailable; native insensitive checks still run"
        );
        return;
    }
    case.write("sensitive/case-probe", b"case probe");
    assert!(!directory.join("CASE-PROBE").exists());
    successful(
        &case
            .command()
            .args([
                "build",
                "main.wave",
                "--emit=ir,bin",
                "--out-dir",
                "sensitive",
                "-o",
                "sensitive/MAIN.LL",
                "--dry-run",
            ])
            .output()
            .unwrap(),
    );
    assert!(!directory.join("main.ll").exists());
    assert!(!directory.join("MAIN.LL").exists());
    assert!(!case.0.join("target").exists());
}

#[cfg(unix)]
#[test]
fn directory_and_file_symlinks_cannot_redirect_ir_into_the_final_binary() {
    use std::os::unix::fs::symlink;
    use std::path::Path;

    let case = Case::new();
    case.write("out/main.ll", b"previous artifact");
    symlink("out", case.0.join("alias")).unwrap();
    collision(
        &case
            .command()
            .args([
                "build",
                "main.wave",
                "--emit=ir,bin",
                "--out-dir",
                "out",
                "-o",
                "alias/main.ll",
                "--error-format=json",
            ])
            .output()
            .unwrap(),
    );
    assert_eq!(fs::read(case.0.join("out/main.ll")).unwrap(), b"previous artifact");

    for existing in [false, true] {
        let case = Case::new();
        fs::create_dir(case.0.join("out")).unwrap();
        if existing {
            case.write("program", b"previous binary");
        }
        symlink("../program", case.0.join("out/main.ll")).unwrap();
        collision(
            &case
                .command()
                .args([
                    "build",
                    "main.wave",
                    "--emit=ir,bin",
                    "--out-dir",
                    "out",
                    "-o",
                    "program",
                    "--error-format=json",
                ])
                .output()
                .unwrap(),
        );
        assert_eq!(fs::read_link(case.0.join("out/main.ll")).unwrap(), Path::new("../program"));
        if existing {
            assert_eq!(fs::read(case.0.join("program")).unwrap(), b"previous binary");
        } else {
            assert!(!case.0.join("program").exists());
        }
        assert!(!case.0.join("target").exists());
    }
}

#[cfg(unix)]
#[test]
fn parent_components_are_resolved_after_directory_symlinks() {
    let case = Case::new();
    case.write("real/main.ll", b"previous artifact");
    case.write("container/main.ll", b"different lexical destination");
    fs::create_dir(case.0.join("real/deeper")).unwrap();
    std::os::unix::fs::symlink("../real/deeper", case.0.join("container/link")).unwrap();
    collision(
        &case
            .command()
            .args([
                "build",
                "main.wave",
                "--emit=ir,bin",
                "--out-dir",
                "real",
                "-o",
                "container/link/../main.ll",
                "--error-format=json",
            ])
            .output()
            .unwrap(),
    );
    assert_eq!(fs::read(case.0.join("real/main.ll")).unwrap(), b"previous artifact");
    assert_eq!(
        fs::read(case.0.join("container/main.ll")).unwrap(),
        b"different lexical destination"
    );
    assert!(!case.0.join("target").exists());
}

#[cfg(unix)]
#[test]
fn one_symlink_path_cannot_name_two_distinct_requested_artifacts() {
    let case = Case::new();
    case.write("out/target", b"previous symlink target");
    std::os::unix::fs::symlink("target", case.0.join("out/main.ll")).unwrap();
    collision(
        &case
            .command()
            .args([
                "build",
                "main.wave",
                "--emit=ir,bin",
                "--out-dir",
                "out",
                "-o",
                "out/main.ll",
                "--error-format=json",
            ])
            .output()
            .unwrap(),
    );
    assert_eq!(fs::read(case.0.join("out/target")).unwrap(), b"previous symlink target");
    assert!(fs::symlink_metadata(case.0.join("out/main.ll")).unwrap().file_type().is_symlink());
    assert!(!case.0.join("target").exists());
}

#[test]
fn two_direct_artifact_writers_cannot_share_an_existing_hardlink() {
    let case = Case::new();
    let ast = case.write("out/main.ast.json", b"previous direct artifact");
    let ir = case.0.join("out/main.ll");
    fs::hard_link(&ast, &ir).unwrap();
    collision(
        &case
            .command()
            .args([
                "build",
                "main.wave",
                "--emit=ast,ir",
                "--ast-format=json",
                "--out-dir",
                "out",
                "--error-format=json",
            ])
            .output()
            .unwrap(),
    );
    assert_eq!(fs::read(&ast).unwrap(), b"previous direct artifact");
    assert_eq!(fs::read(&ir).unwrap(), b"previous direct artifact");
}

#[test]
fn msvc_companion_destinations_are_checked_against_explicit_emits() {
    for target in ["x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"] {
        if llvm::codegen::target::target_spec_for_triple(target).is_none() {
            continue;
        }
        let case = Case::new();
        let output = case.write("out/main.ll", b"previous artifact");
        // Use a fresh command to avoid specifying the target twice.
        let result = Command::new(env!("CARGO_BIN_EXE_wavec"))
            .current_dir(&case.0)
            .args([
                "build",
                "main.wave",
                "--target",
                target,
                "--emit=ir,bin",
                "--out-dir",
                "out",
                "-o",
                "out/app.exe",
                "-C",
                "link-arg=/PDB:out/main.ll",
                "--error-format=json",
            ])
            .output()
            .unwrap();
        collision(&result);
        assert_eq!(fs::read(output).unwrap(), b"previous artifact");
        assert!(!case.0.join("target").exists());
    }
}

#[test]
fn identity_passthrough_and_multiinput_artifact_names_stay_valid() {
    let case = Case::new();
    let ir = b"define i32 @main() { ret i32 0 }\n";
    case.write("identity.ll", ir);
    successful(
        &case
            .command()
            .args(["build", "identity.ll", "--emit=ir", "--out-dir", "."])
            .output()
            .unwrap(),
    );
    assert_eq!(fs::read(case.0.join("identity.ll")).unwrap(), ir);
    fs::create_dir(case.0.join("identity-out")).unwrap();
    fs::hard_link(case.0.join("identity.ll"), case.0.join("identity-out/identity.ll")).unwrap();
    successful(
        &case
            .command()
            .args(["build", "identity.ll", "--emit=ir", "--out-dir", "identity-out"])
            .output()
            .unwrap(),
    );
    case.write("first/same.wave", b"fun first() -> i32 { return 1; }\n");
    case.write("second/same.wave", b"fun second() -> i32 { return 2; }\n");
    successful(
        &case
            .command()
            .args([
                "build",
                "first/same.wave",
                "second/same.wave",
                "--emit=ir,obj",
                "--out-dir",
                "multi",
            ])
            .output()
            .unwrap(),
    );
    for name in ["same_1.ll", "same_2.ll", "same_1.o", "same_2.o"] {
        assert!(fs::metadata(case.0.join("multi").join(name)).unwrap().len() > 0);
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", feature = "llvm-target-x86"))]
#[test]
fn atomic_publication_separates_existing_hardlinks_and_safe_binary_symlinks() {
    for symlink in [false, true] {
        let case = Case::new();
        let ir = case.write("out/main.ll", b"previous artifact");
        let binary = case.0.join("program");
        if symlink {
            std::os::unix::fs::symlink("out/main.ll", &binary).unwrap();
        } else {
            fs::hard_link(&ir, &binary).unwrap();
        }
        let result = Command::new(env!("CARGO_BIN_EXE_wavec"))
            .current_dir(&case.0)
            .args([
                "build",
                "main.wave",
                "--target=x86_64-unknown-linux-gnu",
                "--emit=ir,bin",
                "--out-dir",
                "out",
                "-o",
                "program",
            ])
            .output()
            .unwrap();
        successful(&result);
        assert!(fs::read_to_string(&ir).unwrap().contains("target triple"));
        assert_eq!(&fs::read(&binary).unwrap()[..4], b"\x7fELF");
        successful(&Command::new(&binary).output().unwrap());
    }
}
