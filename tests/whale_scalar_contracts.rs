// SPDX-License-Identifier: MPL-2.0
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use utils::wson;

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Case(PathBuf);

impl Case {
    fn new(source: &str) -> Self {
        let directory = std::env::temp_dir().join(format!(
            "wave-whale-scalars-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("main.wave"), source).unwrap();
        Self(directory)
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_wavec")).current_dir(&self.0).args(args).output().unwrap()
    }

    fn ir(&self) -> String {
        let result = self.run(&["--whale", "build", "main.wave", "--emit=ir"]);
        assert!(result.status.success(), "{result:?}");
        fs::read_to_string(self.0.join("main.wir")).unwrap()
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn scalar_division_float_special_values_and_uninitialized_locals_emit_verified_ir() {
    let case = Case::new(
        r#"
fun integer(a: i128, b: i128) -> i128 { return a / b; }
fun remainder(a: i128, b: i128) -> i128 { return a % b; }
fun unsigned(a: u64, b: u64) -> u64 { return a / b + a % b; }
fun float32(a: f32, b: f32) -> f32 { return a / b + a % b; }
fun float64(a: i16, b: f64) -> f64 { return a / b + a % b; }
fun negative_zero(a: f64) -> f64 { return a / -0.0; }
fun init(flag: bool) -> i32 { var n: i32; if (flag) { n = 7; } return n; }
fun main() -> i32 { return init(true); }
"#,
    );
    let ir = case.ir();
    for opcode in
        ["sdiv", "srem", "udiv", "urem", "fdiv f32", "frem f32", "fdiv f64", "frem f64", "uninit"]
    {
        assert!(ir.contains(opcode), "{opcode}: {ir}");
    }
    assert!(ir.contains("signed integer division overflow"), "{ir}");
    assert!(ir.contains("integer remainder by zero"), "{ir}");
    assert!(!ir.contains("undef"), "{ir}");
}

#[test]
fn imported_default_calls_preserve_literal_types_order_and_llvm_default_execution() {
    let case = Case::new(
        r#"
import("./defaults.wave")::{pick, pick_byte};
fun main() -> i32 {
    var n: i32 = 0;
    var result: i32 = pick((n = 4));
    if (result != 47 || n != 4 || pick() != 17 || pick_byte() != 255) { return 1; }
    return 0;
}
"#,
    );
    fs::write(
        case.0.join("defaults.wave"),
        "pub fun pick(a: i32 = 1, b: i32 = 7) -> i32 { return a * 10 + b; }\n\
         pub fun pick_byte(n: u8 = 255) -> i32 { return n as i32; }\n",
    )
    .unwrap();
    let ir = case.ir();
    assert!(ir.contains("call"), "{ir}");
    assert!(!case.0.join("main.ll").exists());
    let host = match std::env::consts::OS {
        "macos" => format!("{}-apple-darwin", std::env::consts::ARCH),
        "windows" => format!("{}-pc-windows-msvc", std::env::consts::ARCH),
        "freebsd" => format!("{}-unknown-freebsd", std::env::consts::ARCH),
        _ => format!("{}-unknown-linux-gnu", std::env::consts::ARCH),
    };
    if llvm::codegen::target::target_spec_for_triple(&host).is_some() {
        for opt in ["-O0", "-O2"] {
            // Omission of --whale deliberately checks the established LLVM
            // default backend and its real call/default evaluation contract.
            let result =
                case.run(&["build", "main.wave", "--target", &host, opt, "--emit=ir,bin", "--run"]);
            assert!(result.status.success(), "{result:?}");
        }
        assert!(case.0.join("main.ll").exists());
    }
}

#[test]
fn constant_faults_and_invalid_calls_keep_json_source_errors_and_previous_ir() {
    for source in [
        "const N: i32 = 1 / 0; fun main() -> i32 { return N; }",
        "const N: i32 = 1 % 0; fun main() -> i32 { return N; }",
        "const N: i32 = (-2147483648 as i32) / -1; fun main() -> i32 { return N; }",
        "fun pick(n: i32) -> i32 { return n; } fun main() -> i32 { return pick(); }",
        "fun pick(n: i32 = 7) -> i32 { return n; } fun main() -> i32 { return pick(1, 2); }",
    ] {
        let case = Case::new(source);
        fs::write(case.0.join("main.wir"), "previous verified IR").unwrap();
        let result =
            case.run(&["--whale", "build", "main.wave", "--emit=ir", "--error-format=json"]);
        assert!(!result.status.success(), "{source}");
        let stderr = String::from_utf8_lossy(&result.stderr);
        let diagnostic = wson::parse_json(stderr.trim()).unwrap();
        assert!(diagnostic.get("error").unwrap().get("span").is_some(), "{stderr}");
        assert!(!stderr.contains("internal compiler error"), "{stderr}");
        assert_eq!(fs::read_to_string(case.0.join("main.wir")).unwrap(), "previous verified IR");
    }
}

#[test]
fn shared_defined_results_execute_with_the_default_llvm_backend() {
    let host = match std::env::consts::OS {
        "macos" => format!("{}-apple-darwin", std::env::consts::ARCH),
        "windows" => format!("{}-pc-windows-msvc", std::env::consts::ARCH),
        "freebsd" => format!("{}-unknown-freebsd", std::env::consts::ARCH),
        _ => format!("{}-unknown-linux-gnu", std::env::consts::ARCH),
    };
    if llvm::codegen::target::target_spec_for_triple(&host).is_none() {
        return;
    }
    for source in [
        include_str!("fixtures/whale_scalar_defined.wave"),
        include_str!("fixtures/whale_float_defined.wave"),
    ] {
        let case = Case::new(source);
        case.ir();
        for opt in ["-O0", "-O2"] {
            let result =
                case.run(&["build", "main.wave", "--target", &host, opt, "--emit=ir,bin", "--run"]);
            assert!(result.status.success(), "{result:?}");
        }
    }
}
