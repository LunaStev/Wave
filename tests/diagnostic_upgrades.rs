// SPDX-License-Identifier: MPL-2.0
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};
use utils::wson::{self, Value};

static NEXT_CASE: AtomicU64 = AtomicU64::new(0);

struct Case {
    root: PathBuf,
    target: String,
}

impl Case {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "wave-diagnostic-upgrades-{}-{}",
            std::process::id(),
            NEXT_CASE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        let mut case = Self { root, target: String::new() };
        let targets = case.run(&["print", "target-list"]);
        assert!(targets.status.success());
        case.target = String::from_utf8(targets.stdout).unwrap().lines().next().unwrap().into();
        case
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_wavec"))
            .current_dir(&self.root)
            .env("NO_COLOR", "1")
            .args(args)
            .output()
            .unwrap()
    }

    fn error(&self, source: &str, offending: &str, message: &str) -> Value {
        self.error_at(source, source.find(offending).unwrap(), message)
    }

    fn error_at(&self, source: &str, start: usize, message: &str) -> Value {
        fs::write(self.root.join("main.wave"), source).unwrap();
        fs::write(self.root.join("main.ll"), "previous IR").unwrap();
        let prefix = &source[..start];
        let line = prefix.chars().filter(|&c| c == '\n').count() + 1;
        let column = prefix.rsplit('\n').next().unwrap().chars().count() + 1;
        let mut record = Value::Null;
        for command in ["check", "build"] {
            for format in ["json", "human"] {
                let mut args =
                    vec![command, "main.wave", "--target", &self.target, "--error-format", format];
                if command == "build" {
                    args.push("--emit=ir");
                }
                let output = self.run(&args);
                assert_eq!(output.status.code(), Some(1), "{args:?}: {output:?}");
                assert!(output.stdout.is_empty(), "{output:?}");
                let text = String::from_utf8(output.stderr).unwrap();
                assert!(text.contains(message), "{source}\n{text}");
                if format == "json" {
                    let parsed = wson::parse_json(text.trim()).unwrap();
                    let error = parsed.get("error").unwrap();
                    assert_eq!(error.get_u64("line"), Some(line as u64), "{text}");
                    assert_eq!(error.get_u64("column"), Some(column as u64), "{text}");
                    assert_eq!(
                        error.get("span").unwrap().get_u64("start"),
                        Some(start as u64),
                        "{text}"
                    );
                    record = parsed;
                } else {
                    assert!(text.contains(&format!("main.wave:{line}:{column}")), "{text}");
                }
                assert_eq!(fs::read_to_string(self.root.join("main.ll")).unwrap(), "previous IR");
            }
        }
        record
    }

    fn valid(&self, source: &str) {
        fs::write(self.root.join("main.wave"), source).unwrap();
        let output = self.run(&["check", "main.wave", "--target", &self.target]);
        assert!(output.status.success(), "{source}\n{output:?}");
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn keyword_migrations_keep_the_original_token_and_help() {
    let case = Case::new();
    for declaration in ["let value: i32 = 1;", "let mut value: i32 = 1;"] {
        let source = format!("fun main() {{\n    {declaration}\n}}");
        case.error(&source, "let", "use `var`");
    }
    case.error("fn main() {}", "fn", "did you mean `fun`?");
    let result = case.error("other main() {}", "other", "unexpected token at top level");
    assert!(!result.get("error").unwrap().get_str("help").unwrap().contains("did you mean"));
    case.valid("fun main() { var value: i32 = 1; }");
}

#[test]
fn missing_statement_terminators_identify_the_boundary() {
    let case = Case::new();
    // Inferred declarations are not valid Wave syntax; retain the explicit
    // type diagnostic instead of inventing an inferred-declaration grammar.
    case.error("fun main() { var x = 1 }", "=", "requires an explicit type");
    for statement in ["var x: i32 = 1", "return 1", "1 + 2"] {
        let source = format!("fun main() -> i32 {{\n    {statement}\n}}");
        let result = case.error(&source, "}", "expected ';'");
        let Value::Array(expected) = result.get("error").unwrap().get("expected").unwrap() else {
            panic!("expected-token list missing");
        };
        assert!(expected.iter().any(|value| matches!(value, Value::String(text) if text == "';'")));
    }
    for statement in ["break", "continue"] {
        case.error(
            &format!("fun main() {{ while (true) {{ {statement} }} }}"),
            "}",
            "expected ';'",
        );
    }
    let source = "fun main() { var x: i32 = 1";
    case.error_at(source, source.len(), "expected ';'");
    case.valid("fun main() -> i32 { var x: i32 = 1; return x; }");
}

#[test]
fn storage_errors_keep_names_types_and_array_initializers() {
    let case = Case::new();
    for (declaration, offending, message) in [
        ("var 123: i32 = 1;", "123", "identifier"),
        ("var value: = 1;", "=", "type"),
        ("var value: ptr<>;", "ptr", "type"),
    ] {
        case.error(&format!("fun main() {{ {declaration} }}"), offending, message);
    }
    for storage in ["const", "static"] {
        case.error(&format!("{storage} 123: i32 = 1; fun main() {{}}"), "123", "identifier");
        case.error(&format!("{storage} value: = 1; fun main() {{}}"), "=", "type");
        case.error(
            &format!("{storage} value = 1; fun main() {{}}"),
            "=",
            if storage == "const" { "expected ':'" } else { "explicit type" },
        );
    }
    for (initializer, count) in [("[]", 0), ("[1]", 1), ("[1, 2, 3]", 3)] {
        let message = format!("expected 2 elements, found {count}");
        case.error(
            &format!("fun main() {{ var a: array<i32, 2> = {initializer}; }}"),
            initializer,
            &message,
        );
        for storage in ["const", "static"] {
            case.error(
                &format!("{storage} a: array<i32, 2> = {initializer}; fun main() {{}}"),
                initializer,
                &message,
            );
        }
    }
    case.error(
        "fun main() { var a: array<array<i32, 2>, 1> = [[1]]; }",
        "[1]",
        "expected 2 elements, found 1",
    );
    case.valid("const a: array<i32, 2> = [1, 2]; static b: i32 = 3; fun main() { var c: array<array<i32, 2>, 1> = [[1, 2]]; }");
}

#[test]
fn ffi_and_proto_diagnostics_explain_local_failures() {
    let case = Case::new();
    for keyword in ["extern", "export"] {
        for (header, offending, message) in [
            ("123)", "123", "expected '('"),
            ("(123)", "123", "identifier"),
            ("(c, 123)", "123", "symbol string literal"),
            ("(c, \"\\xff\")", "\"\\xff\"", "valid UTF-8"),
            ("(c fun", "fun", "expected ')'"),
        ] {
            case.error(
                &format!("{keyword} {header} fun value(); fun main() {{}}"),
                offending,
                message,
            );
        }
    }
    case.error("export(c) fun value<T>() {} fun main() {}", "fun value", "cannot be generic");
    case.error("export(c) { fun value<T>() {} } fun main() {}", "fun value", "cannot be generic");
    case.error(
        "export(c, \"alias\") { fun value() {} } fun main() {}",
        "\"alias\"",
        "cannot use a single symbol alias",
    );
    case.error("proto 123 {} fun main() {}", "123", "identifier");
    case.error("proto Thing fun main() {}", "fun", "expected '{'");
    case.error("proto Thing { var x: i32; } fun main() {}", "var", "method declaration");
    case.error("proto Thing { fun () {} } fun main() {}", "(", "function name");
    let source = "proto Thing {";
    case.error_at(source, source.len(), "method declaration or '}'");
    case.valid("extern(c) { fun external(i32) -> i32; } export(c) { fun one() -> i32 { return 1; } } struct Thing { value: i32; } proto Thing { fun value(self: ptr<Thing>) -> i32 { return self.value; } } fun main() {}");
}

#[test]
fn io_syntax_and_counts_are_actionable() {
    let case = Case::new();
    for builtin in ["print", "println", "input"] {
        case.error(&format!("fun main() {{ {builtin} 1; }}"), "1", "expected '('");
        case.error(&format!("fun main() {{ {builtin}(1); }}"), "1", "format string literal");
        case.error(
            &format!("fun main() {{ {builtin}(\"{{}} {{}}\"); }}"),
            "\"",
            "expected 2 format arguments, found 0",
        );
        case.error(
            &format!("fun main() {{ {builtin}(\"plain\", 1); }}"),
            "\"",
            "expected 0 format arguments, found 1",
        );
        case.error(&format!("fun main() {{ {builtin}(\"plain\"; }}"), ";", "expected ')'");
        case.error(&format!("fun main() {{ {builtin}(\"plain\") }}"), "}", "expected ';'");
    }
    case.valid("fun main() { var x: i32 = 0; input(\"{}\", x); print(\"plain\"); println(\"{} {x} {:.2}\", x, x, 1.25); }");
}

#[test]
fn placeholder_spans_follow_raw_escapes_and_unicode() {
    let case = Case::new();
    for (literal, offending, message) in [
        ("\"prefix {wat} suffix\"", "{wat}", "unsupported format placeholder"),
        ("\"한글 \\t \\x41 {} {wat}\"", "{wat}", "unsupported format placeholder"),
        ("\"prefix \\x7bwat\\x7d\"", "\\x7b", "unsupported format placeholder"),
        ("\"{:.}\"", "{:.}", "precision"),
        ("\"{:.2147483648}\"", "{:.2147483648}", "precision exceeds"),
        ("\"{\\xff}\"", "{\\xff}", "UTF-8"),
    ] {
        let source = format!("fun main() {{ println({literal}); }}");
        let result = case.error(&source, offending, message);
        let error = result.get("error").unwrap();
        assert!(error.get_str("help").unwrap().contains("{:.N}"));
        let span = error.get("span").unwrap();
        let start = span.get_u64("start").unwrap() as usize;
        let end = span.get_u64("end").unwrap() as usize;
        assert!(!source[start..end].contains('"'));
        assert!(source[start..end].ends_with('}') || source[start..end].ends_with("\\x7d"));
    }
    case.error("fun main() { input(\"text {d}\"); }", "{d}", "only `{}`");
    // Unmatched opening braces have always been literal text, not syntax errors.
    case.valid("fun main() { println(\"literal {unclosed\"); }");
}
