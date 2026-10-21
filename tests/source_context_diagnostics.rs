// SPDX-License-Identifier: MPL-2.0
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};
use utils::wson;

struct Case(PathBuf);

impl Case {
    fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("wave-source-context-{name}-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_wavec")).current_dir(&self.0).args(args).output().unwrap()
    }

    fn target(&self) -> String {
        let result = self.run(&["print", "target-list"]);
        assert!(result.status.success());
        String::from_utf8(result.stdout).unwrap().lines().next().unwrap().to_owned()
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn invalid_io_first_arguments_identify_the_consumed_token() {
    let case = Case::new("io");
    let target = case.target();
    for name in ["print", "println", "input"] {
        for argument in ["1", "true", "value"] {
            let suffix = if name == "input" { ", value" } else { "" };
            let source =
                format!("fun main() {{ var value: i32 = 0; {name}({argument}{suffix}); }}");
            fs::write(case.0.join("main.wave"), &source).unwrap();
            let start = source.find(&format!("({argument}{suffix})")).unwrap() + 1;
            let result =
                case.run(&["check", "main.wave", "--target", &target, "--error-format=json"]);
            assert_eq!(result.status.code(), Some(1));
            let text = String::from_utf8(result.stderr).unwrap();
            let json = wson::parse_json(text.trim()).unwrap();
            let error = json.get("error").unwrap();
            let span = error.get("span").unwrap();
            assert_eq!(span.get_u64("start"), Some(start as u64), "{text}");
            assert!(error.get_str("found").unwrap().contains(argument), "{text}");
            let human = case.run(&["check", "main.wave", "--target", &target]);
            assert!(!human.status.success());
            assert!(String::from_utf8_lossy(&human.stderr).contains(&format!(":1:{}", start + 1)));
        }
    }
}

#[test]
fn unconstrained_array_statements_report_source_errors_and_preserve_outputs() {
    let case = Case::new("arrays");
    let target = case.target();
    for expression in ["[1, 2]", "[]", "&[1, 2]", "&[]", "([1, 2])", "(&[])"] {
        let source = format!("fun main() {{\n    {expression};\n}}\n");
        fs::write(case.0.join("main.wave"), &source).unwrap();
        fs::write(case.0.join("main.ll"), "previous IR").unwrap();
        for command in ["check", "build"] {
            for opt in ["-O0", "-O2"] {
                let mut args =
                    vec![command, "main.wave", "--target", &target, opt, "--error-format=json"];
                if command == "build" {
                    args.push("--emit=ir");
                }
                let result = case.run(&args);
                assert_eq!(result.status.code(), Some(1), "{expression}: {result:?}");
                let text = String::from_utf8(result.stderr).unwrap();
                let json = wson::parse_json(text.trim()).unwrap();
                let error = json.get("error").unwrap();
                assert!(
                    error.get_str("message").unwrap().contains("concrete array context"),
                    "{text}"
                );
                assert_ne!(error.get_str("code"), Some("E9001"));
                assert_eq!(error.get("span").unwrap().get_u64("start"), Some(17), "{text}");
                assert_eq!(fs::read_to_string(case.0.join("main.ll")).unwrap(), "previous IR");
            }
        }
        let human = case.run(&["check", "main.wave", "--target", &target]);
        assert!(
            String::from_utf8_lossy(&human.stderr).contains("explicitly typed array destination")
        );
    }
    fs::write(case.0.join("main.wave"), "fun take(a: array<i32, 2>) -> array<i32, 2> { return a; } fun main() { var a: array<i32, 2> = take([1, 2]); var p: ptr<array<i32, 2>> = &[3, 4]; }").unwrap();
    let result = case.run(&["build", "main.wave", "--target", &target, "--emit=ir"]);
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
}
