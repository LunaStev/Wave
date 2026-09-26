// SPDX-License-Identifier: MPL-2.0
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Case(PathBuf);
impl Case {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "wave-native-providers-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn compiler(&self) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_wavec"));
        cmd.current_dir(&self.0)
            .arg("--std-root")
            .arg(root().join("std"));
        cmd
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
    root().join("tests/fixtures/native_providers").join(name)
}
fn supported(target: &str) -> bool {
    llvm::codegen::target::target_spec_for_triple(target).is_some()
}
fn checked(cmd: &mut Command, context: &str, dir: &Path) {
    let stdout = dir.join("stdout.log");
    let stderr = dir.join("stderr.log");
    let mut child = cmd
        .stdout(Stdio::from(fs::File::create(&stdout).unwrap()))
        .stderr(Stdio::from(fs::File::create(&stderr).unwrap()))
        .spawn()
        .unwrap_or_else(|e| panic!("{context}: {cmd:?}: {e}"));
    let deadline = Instant::now() + Duration::from_secs(60);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!(
                "{context}: timed out: {cmd:?}\n{}\n{}",
                fs::read_to_string(stdout).unwrap(),
                fs::read_to_string(stderr).unwrap()
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(
        status.success(),
        "{context}: {cmd:?}: {status}\n{}\n{}",
        fs::read_to_string(stdout).unwrap(),
        fs::read_to_string(stderr).unwrap()
    );
}
fn build(case: &Case, source: &Path, target: &str, opt: &str, output: &Path, object: bool) {
    checked(
        case.compiler()
            .arg("build")
            .arg(source)
            .args([
                "--target",
                target,
                opt,
                "--emit",
                if object { "obj" } else { "bin" },
                "-o",
            ])
            .arg(output),
        &format!("{target} {opt} {} build", source.display()),
        &case.0,
    );
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

#[test]
fn loongarch_attribute_aliases_select_identical_declarations() {
    let target = "loongarch64-unknown-linux-gnu";
    if !supported(target) {
        return;
    }
    let case = Case::new();
    let source = case.0.join("alias.wave");
    fs::write(&source, "#[target(arch=\" LoOnG64 \")]\nfun alias() -> i32 { return 7; }\n#[target(arch=\"loongarch64\")]\nfun canonical() -> i32 { return alias(); }\nfun main() -> i32 { return canonical(); }\n").unwrap();
    checked(
        case.compiler()
            .arg("build")
            .arg(source)
            .args(["--target", target, "--emit=check"]),
        target,
        &case.0,
    );
}

#[test]
fn native_provider_fixtures_compile_and_run_on_their_host() {
    let case = Case::new();
    for (source, targets) in [
        (
            "dup2.wave",
            vec![
                "x86_64-unknown-linux-gnu",
                "aarch64-unknown-linux-gnu",
                "riscv64-unknown-linux-gnu",
                "loongarch64-unknown-linux-gnu",
            ],
        ),
        (
            "macos_memory.wave",
            vec!["x86_64-apple-darwin", "aarch64-apple-darwin"],
        ),
        (
            "macos_event.wave",
            vec!["x86_64-apple-darwin", "aarch64-apple-darwin"],
        ),
        (
            "random.wave",
            vec![
                "x86_64-pc-windows-msvc",
                "aarch64-pc-windows-msvc",
                "x86_64-unknown-freebsd",
            ],
        ),
    ] {
        for target in targets {
            if !supported(target) {
                continue;
            }
            for opt in ["-O0", "-O2"] {
                let native = target == host_target();
                let output = case.0.join(if native { "probe.exe" } else { "probe.o" });
                build(&case, &fixture(source), target, opt, &output, !native);
                if native {
                    checked(
                        &mut Command::new(&output),
                        &format!("{target} {opt} {source} run"),
                        &case.0,
                    );
                }
            }
        }
    }
}

// Exact OS failures and historical clock values are injected at the OS ABI
// boundary. The production Wave bodies are copied unchanged except imports
// and the system calling-convention spelling on the Linux test host.
#[cfg(all(
    target_os = "linux",
    target_arch = "x86_64",
    feature = "llvm-target-x86"
))]
#[test]
fn native_provider_os_boundary_failures() {
    let case = Case::new();
    for (name, provider) in [
        ("windows_time", "std/sys/windows/time.wave"),
        ("windows_random", "std/sys/windows/random.wave"),
        ("freebsd_random", "std/sys/freebsd/amd64/random.wave"),
        ("macos_event", "std/sys/macos/event.wave"),
    ] {
        let mut text = fs::read_to_string(root().join(provider))
            .unwrap()
            .replace("extern(system,", "extern(c,");
        if name == "freebsd_random" {
            text = text.replace(
                "import(\"std::sys::freebsd::amd64::syscall\")::{syscall3};",
                "extern(c) fun syscall3(id: i64, buffer: i64, size: i64, flags: i64) -> i64;",
            );
        }
        if name.ends_with("random") {
            text += &fs::read_to_string(root().join("std/random/fill.wave"))
                .unwrap()
                .replace(
                    "import(\"std::sys::random\")::{sys_random_available, sys_random_read};",
                    "",
                );
        }
        text += &fs::read_to_string(fixture(&format!("{name}_mock.wave"))).unwrap();
        let source = case.0.join(format!("{name}.wave"));
        fs::write(&source, text).unwrap();
        let c_object = case.0.join("host.o");
        checked(
            Command::new("clang")
                .arg("-c")
                .arg(fixture(&format!("{name}_mock.c")))
                .args(["-O2", "-o"])
                .arg(&c_object),
            name,
            &case.0,
        );
        for opt in ["-O0", "-O2"] {
            let object = case.0.join("probe.o");
            build(
                &case,
                &source,
                "x86_64-unknown-linux-gnu",
                opt,
                &object,
                true,
            );
            let binary = case.0.join("probe");
            checked(
                Command::new("clang")
                    .arg(&object)
                    .arg(&c_object)
                    .arg("-o")
                    .arg(&binary),
                &format!("{name} {opt} link"),
                &case.0,
            );
            checked(
                &mut Command::new(binary),
                &format!("{name} {opt} run"),
                &case.0,
            );
        }
    }
}

#[test]
fn darwin_bidirectional_c_abi_fixtures() {
    let case = Case::new();
    let required = std::env::var_os("WAVE_RUN_DARWIN_INTEROP_TESTS").is_some();
    if required {
        assert_eq!(
            std::env::consts::OS,
            "macos",
            "Darwin ABI execution requires a native macOS runner"
        );
        assert!(supported(&host_target()), "native LLVM target is required");
    }
    for target in ["x86_64-apple-darwin", "aarch64-apple-darwin"] {
        if !supported(target) {
            continue;
        }
        let mut fixtures = vec![
            "c_abi_edges/interop",
            "native_providers/darwin_variadic",
            "aggregate_registers/pointer_float",
            "aggregate_registers/float_pressure",
            "aggregate_registers/mixed_gp_pressure",
            "aggregate_registers/aligned_i128",
            "aggregate_registers/triple_float",
            "aggregate_registers/hfa_stack",
            "aggregate_registers/aligned_pressure",
        ];
        if target.starts_with("x86_64") {
            fixtures.extend(["x86_64_sysv/interop", "x86_64_sysv_pressure/interop"]);
        } else {
            fixtures.push("aarch64_aapcs64/interop");
        }
        for name in fixtures {
            for opt in ["-O0", "-O2"] {
                let context = format!("{target} {opt} {name}");
                let source = root().join("tests/fixtures").join(name);
                let c_object = case.0.join("c.o");
                checked(
                    Command::new("clang")
                        .arg(format!("--target={target}"))
                        .args([
                            "-ffreestanding",
                            "-fno-builtin",
                            "-fno-stack-protector",
                            opt,
                            "-c",
                        ])
                        .arg(source.with_extension("c"))
                        .arg("-o")
                        .arg(&c_object),
                    &context,
                    &case.0,
                );
                let object = case.0.join("wave.o");
                build(
                    &case,
                    &source.with_extension("wave"),
                    target,
                    opt,
                    &object,
                    true,
                );
                if required && target == host_target() {
                    let binary = case.0.join("abi");
                    checked(
                        Command::new("clang")
                            .arg(&c_object)
                            .arg(&object)
                            .arg("-o")
                            .arg(&binary),
                        &format!("{context} link"),
                        &case.0,
                    );
                    checked(
                        &mut Command::new(binary),
                        &format!("{context} run"),
                        &case.0,
                    );
                }
            }
        }
    }
}
