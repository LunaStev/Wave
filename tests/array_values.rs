// SPDX-License-Identifier: MPL-2.0
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Case(PathBuf);

impl Case {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "wave-array-values-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("main.wave"), SOURCE).unwrap();
        Self(root)
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_wavec")).current_dir(&self.0).args(args).output().unwrap()
    }

    fn target(&self) -> String {
        let output = self.run(&["print", "target-list"]);
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8(output.stdout).unwrap().lines().next().unwrap().to_owned()
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const SOURCE: &str = r#"
static calls: i32 = 0;
static order: i32 = 0;
fun made() -> array<i32, 2> {
    calls += 1;
    return [calls, 20];
}
fun total(values: array<i32, 2>) -> i32 { return values[0] + values[1]; }
fun changed(values: array<i32, 2>) -> i32 {
    values[0] = 77;
    return values[0];
}
struct Record { values: array<i32, 2>; }
struct Empty { values: array<u8, 0>; }
struct Receiver {
    fun sum(self: Receiver, values: array<i32, 2>) -> i32 { return total(values); }
}
fun receiver() -> Receiver {
    order = order * 10 + 1;
    return Receiver {};
}
fun argument() -> array<i32, 2> {
    order = order * 10 + 2;
    return [3, 4];
}
fun main() -> i32 {
    var source: array<i32, 2> = [11, 22];
    var copy: array<i32, 2> = (source);
    copy[0] = 99;
    if (source[0] != 11 || copy[0] != 99) { return 1; }
    var record: Record = Record { values: source };
    record.values[1] = 55;
    if (source[1] != 22 || record.values[1] != 55) { return 2; }
    var nested: array<array<i32, 2>, 2> = [source, (source)];
    var row: array<i32, 2> = nested[0];
    nested[0][0] = 66;
    row[1] = 88;
    if (source[0] != 11 || nested[1][0] != 11 || row[0] != 11
        || nested[0][1] != 22 || row[1] != 88) { return 3; }
    var view: ptr<i32> = &source[0];
    view[1] = 23;
    if (source[1] != 23 || copy[1] != 22) { return 4; }
    var assigned: array<i32, 2>;
    assigned = source;
    assigned[1] = 1;
    if (source[1] != 23 || total(source) != 34 || changed(source) != 77
        || source[0] != 11) { return 5; }
    var object: Receiver = Receiver {};
    if (source.total() != 34 || object.sum(source) != 34) { return 6; }
    var produced: array<i32, 2> = made();
    var produced_record: Record = Record { values: made() };
    var produced_nested: array<array<i32, 2>, 2> = [made(), source];
    if (produced[0] != 1 || produced_record.values[0] != 2
        || produced_nested[0][0] != 3 || made().total() != 24 || calls != 4) { return 7; }
    if (receiver().sum(argument()) != 7 || order != 12) { return 8; }
    var empty: array<u8, 0> = [];
    var empty_copy: array<u8, 0> = (empty);
    var empty_record: Empty = Empty { values: empty_copy };
    var empty_nested: array<array<u8, 0>, 2> = [empty, empty_record.values];
    return 0;
}
"#;

#[test]
fn array_value_boundaries_emit_verified_ir_at_o0_and_o2() {
    let case = Case::new();
    let target = case.target();
    for optimization in ["-O0", "-O2"] {
        let output =
            case.run(&["build", "main.wave", "--emit=ir", "--target", &target, optimization]);
        assert!(
            output.status.success(),
            "{target} {optimization}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(case.0.join("main.ll").is_file());
    }
}

#[test]
#[cfg(all(target_os = "linux", target_arch = "x86_64", feature = "llvm-target-x86"))]
fn array_value_boundaries_copy_storage_and_evaluate_producers_once() {
    let case = Case::new();
    for optimization in ["-O0", "-O2"] {
        let output = case.run(&["build", "main.wave", "--run", optimization]);
        assert!(
            output.status.success(),
            "{optimization}: status {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
