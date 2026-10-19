// SPDX-License-Identifier: MPL-2.0
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Case(PathBuf);

impl Case {
    fn new(source: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "wave-rvalue-index-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("main.wave"), source).unwrap();
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
static selected: ptr<i32> = null;
fun make() -> array<i32, 2> {
    calls += 1;
    order = order * 10 + 1;
    return [11, 22];
}
fun pick(values: ptr<i32>) -> ptr<i32> {
    calls += 1;
    order = order * 10 + 1;
    return values;
}
fun next_index() -> u8 {
    order = order * 10 + 2;
    return 1;
}
fun change_selected(next: ptr<i32>) -> i32 {
    selected = next;
    return 0;
}
struct Record { values: array<i32, 2>; }
fun make_record() -> Record {
    calls += 1;
    order = order * 10 + 1;
    return Record { values: [31, 32] };
}
fun record_pointer(value: ptr<Record>) -> ptr<Record> {
    calls += 1;
    order = order * 10 + 1;
    return value;
}
fun row_pointer(value: ptr<array<i32, 2>>) -> ptr<array<i32, 2>> {
    calls += 1;
    order = order * 10 + 1;
    return value;
}
fun word() -> str {
    calls += 1;
    order = order * 10 + 1;
    return "xy";
}
fun flags() -> array<bool, 2> { return [true, false]; }
fun flag_pointer(value: ptr<array<bool, 2>>) -> ptr<array<bool, 2>> { return value; }
struct Maker {
    fun values(self: Maker) -> array<i32, 2> { return make(); }
}
fun main() -> i32 {
    if (make()[next_index()] != 22 || calls != 1 || order != 12) { return 1; }
    var values: array<i32, 2> = [11, 22];
    calls = 0; order = 0;
    if (pick(&values[0])[next_index()] != 22 || calls != 1 || order != 12) { return 2; }
    calls = 0; order = 0;
    if ((pick(&values[0]) + 1)[0] != 22 || calls != 1 || order != 1) { return 3; }
    calls = 0; order = 0;
    pick(&values[0])[next_index()] = 44;
    if (values[1] != 44 || calls != 1 || order != 12) { return 4; }
    values[1] = 22;
    calls = 0; order = 0;
    if (make_record().values[next_index()] != 32 || calls != 1 || order != 12) { return 5; }
    var record: Record = Record { values: [31, 32] };
    calls = 0; order = 0;
    record_pointer(&record).values[next_index()] = 77;
    if (record.values[1] != 77 || calls != 1 || order != 12) { return 6; }
    var matrix: array<array<i32, 2>, 2> = [[11, 22], [33, 44]];
    calls = 0; order = 0;
    if (row_pointer(&matrix[0])[next_index()] != 22 || calls != 1 || order != 12) { return 7; }
    calls = 0; order = 0;
    if ((row_pointer(&matrix[0]) + 1)[0] != 33 || calls != 1 || order != 1) { return 8; }
    var whole: ptr<array<array<i32, 2>, 2>> = &matrix;
    whole[1][0] = 35;
    if (matrix[1][0] != 35 || whole[0][1] != 22) { return 9; }
    var assigned: array<i32, 2> = [0, 0];
    order = 0;
    var assigned_index: i32 = (assigned = values)[next_index()];
    if (assigned_index != 22 || assigned[0] != 11 || order != 2) { return 10; }
    selected = &values[0];
    if (selected[change_selected(&values[1])] != 11 || deref selected != 22) { return 11; }
    calls = 0; order = 0;
    if (word()[next_index()] != 'y' || calls != 1 || order != 12) { return 12; }
    calls = 0; order = 0;
    var address: ptr<i32> = &(pick(&values[0])[next_index()]);
    deref address = 66;
    if (values[1] != 66 || calls != 1 || order != 12) { return 13; }
    calls = 0; order = 0;
    var temporary: ptr<i32> = &(make()[0]);
    var other: array<i32, 2> = make();
    if (deref temporary != 11 || other[1] != 22 || calls != 2) { return 14; }
    calls = 0; order = 0;
    var maker: Maker = Maker {};
    if (maker.values()[next_index()] != 22 || calls != 1 || order != 12) { return 15; }
    var index: i32 = 0;
    while (index < 1000) {
        if (make()[1] != 22) { return 16; }
        index += 1;
    }
    var bool_source: array<bool, 2> = [true, false];
    var bool_assigned: array<bool, 2> = [false, true];
    var selected_bool: bool = (bool_assigned = bool_source)[0];
    if (!selected_bool || !flags()[0] || flags()[1] || !flag_pointer(&bool_source)[0]) {
        return 17;
    }
    bool_assigned[0] = false;
    if (!bool_source[0] || bool_assigned[0]) { return 18; }
    var bool_nested: array<array<bool, 2>, 2> = [[true, false], [false, true]];
    var bool_copy: array<array<bool, 2>, 2>;
    var selected_row: array<bool, 2> = (bool_copy = bool_nested)[1];
    var nested_bool: bool = (bool_copy = bool_nested)[0][0];
    if (selected_row[0] || !selected_row[1] || !nested_bool) { return 19; }
    bool_copy[0][0] = false;
    if (!bool_nested[0][0] || bool_copy[0][0]) { return 20; }
    var empty_bool: array<bool, 0> = [];
    var empty_assigned: array<bool, 0>;
    empty_assigned = empty_bool;
    return 0;
}
"#;

#[test]
fn computed_index_bases_emit_verified_ir_at_o0_and_o2() {
    let case = Case::new(SOURCE);
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
fn computed_index_bases_preserve_values_addresses_strides_and_evaluation_order() {
    let case = Case::new(SOURCE);
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

#[test]
fn invalid_computed_index_is_a_source_error_and_preserves_previous_output() {
    let case = Case::new(
        "fun make() -> array<i32, 2> { return [11, 22]; }\nfun main() -> i32 { return make()[1.5]; }\n",
    );
    fs::write(case.0.join("main.ll"), "previous verified IR").unwrap();
    let output = case.run(&[
        "build",
        "main.wave",
        "--emit=ir",
        "--target",
        &case.target(),
        "--error-format=json",
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("main.wave"), "{stderr}");
    assert!(!stderr.contains("E9001"), "{stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");
    assert_eq!(fs::read_to_string(case.0.join("main.ll")).unwrap(), "previous verified IR");
}
