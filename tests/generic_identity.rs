// This file is part of the Wave language project.
// SPDX-License-Identifier: MPL-2.0

use hir::TypedProgram;
use lexer::Lexer;
use parser::ast::ASTNode;
use parser::generics::monomorphize_generics;

fn lower(source: &str) -> TypedProgram {
    let tokens = Lexer::new(source).tokenize().unwrap();
    let syntax = parser::parse_syntax_only(&tokens).unwrap();
    let syntax = monomorphize_generics(syntax).unwrap();
    TypedProgram::lower(syntax).unwrap()
}

fn identity_source(reverse: bool) -> String {
    let mut calls = [
        "var pointer_width: u64 = width<ptr<i32>>();",
        "var named_pointer_width: u64 = width<p_i32>();",
        "var array_width: u64 = width<array<i32, 2>>();",
        "var named_array_width: u64 = width<a2_i32>();",
        "var first_unicode_width: u64 = width<가>();",
        "var second_unicode_width: u64 = width<나>();",
        "var nested_width: u64 = width<Box<i64>>();",
        "var named_nested_width: u64 = width<Box_g_i64>();",
        "var variant_width: u64 = width<Box<Choice<i64>>>();",
        "var named_variant_width: u64 = width<Box_g_Choice_i64_>();",
    ];
    if reverse {
        calls.reverse();
    }
    let calls = calls.join("\n");
    format!(
        r#"
struct p_i32 {{ first: i64; second: i64; }}
struct a2_i32 {{ first: i64; second: i64; third: i64; }}
struct 가 {{ value: i64; }}
struct 나 {{ first: i64; second: i64; third: i64; }}
struct Box<T> {{ value: T; }}
struct Box_g_i64 {{ first: i64; second: i64; }}
variant Choice<T> {{ Value(T) }}
struct Box_g_Choice_i64_ {{ first: i64; second: i64; third: i64; fourth: i64; }}
fun width<T>() -> u64 {{ return __wave_size_of<T>(); }}
fun identity<T>(value: T) -> T {{ return value; }}
fun main() -> i32 {{
    {calls}
    if (pointer_width != __wave_size_of<ptr<i32>>() ||
        named_pointer_width != __wave_size_of<p_i32>() ||
        array_width != __wave_size_of<array<i32, 2>>() ||
        named_array_width != __wave_size_of<a2_i32>() ||
        first_unicode_width != __wave_size_of<가>() ||
        second_unicode_width != __wave_size_of<나>() ||
        nested_width != __wave_size_of<Box<i64>>() ||
        named_nested_width != __wave_size_of<Box_g_i64>>() ||
        variant_width != __wave_size_of<Box<Choice<i64>>>() ||
        named_variant_width != __wave_size_of<Box_g_Choice_i64_>>()) {{
        return 1;
    }}
    var value: i32 = 7;
    var pointer: ptr<i32> = identity<ptr<i32>>(&value);
    var named: p_i32 = identity<p_i32>(p_i32 {{ first: 11, second: 13 }});
    var first: 가 = identity<가>(가 {{ value: 17 }});
    var second: 나 = identity<나>(나 {{ first: 19, second: 23, third: 29 }});
    if (deref pointer != 7 || named.second != 13 ||
        first.value != 17 || second.third != 29) {{
        return 2;
    }}
    println("generic identities checked");
    return 0;
}}
"#
    )
}

#[test]
fn colliding_source_names_and_nested_applications_keep_distinct_typed_instances() {
    for reverse in [false, true] {
        let program = lower(&identity_source(reverse));
        let widths = program
            .syntax()
            .iter()
            .filter(|node| {
                matches!(node, ASTNode::Function(function) if function.name.starts_with("width$g$"))
            })
            .count();
        assert_eq!(widths, 10);
    }
}

#[test]
fn repeated_recursive_specializations_remain_deterministic_and_deduplicated() {
    let source = r#"
struct p_i32 { value: i64; }
struct Node<T> { value: T; next: ptr<Node<T>>; }
fun repeat<T>(value: T, remaining: i32) -> T {
    if (remaining == 0) { return value; }
    return repeat<T>(value, remaining - 1);
}
fun main() {
    var pointer: ptr<i32> = null;
    var named: p_i32 = p_i32 { value: 7 };
    repeat<ptr<i32>>(pointer, 2);
    repeat<p_i32>(named, 2);
    repeat<ptr<i32>>(pointer, 3);
    var pointer_node: Node<ptr<i32>>;
    var named_node: Node<p_i32>;
    var another_pointer_node: Node<ptr<i32>>;
}
"#;
    let instance_names = |program: &TypedProgram| {
        program
            .syntax()
            .iter()
            .filter_map(|node| match node {
                ASTNode::Function(function) if function.name.starts_with("repeat$g$") => {
                    Some(function.name.clone())
                },
                ASTNode::Struct(structure) if structure.name.starts_with("Node$g$") => {
                    Some(structure.name.clone())
                },
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let names = instance_names(&lower(source));
    assert_eq!(names.len(), 4);
    assert_eq!(names, instance_names(&lower(source)));

    for argument in ["ptr<T>", "Box<T>"] {
        let expanding = format!(
            "struct Box<T> {{ value: T; }} \
             fun grow<T>() {{ grow<{argument}>(); }} fun main() {{ grow<i32>(); }}"
        );
        let tokens = Lexer::new(&expanding).tokenize().unwrap();
        let syntax = parser::parse_syntax_only(&tokens).unwrap();
        let error = monomorphize_generics(syntax).unwrap_err();
        assert!(error.contains("generic instantiation depth exceeded"), "{error}");
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", feature = "llvm-target-x86"))]
#[test]
fn generic_identity_results_are_independent_of_specialization_order_at_o0_and_o2() {
    use std::{fs, process::Command};

    let root = std::env::temp_dir().join(format!("wave-generic-identity-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    for reverse in [false, true] {
        let source = root.join(if reverse { "reverse.wave" } else { "forward.wave" });
        fs::write(&source, identity_source(reverse)).unwrap();
        for optimization in ["-O0", "-O2"] {
            let binary = root.join(format!("generic-{}-{optimization}", u8::from(reverse)));
            let output = Command::new(env!("CARGO_BIN_EXE_wavec"))
                .current_dir(&root)
                .arg("build")
                .arg(&source)
                .args([optimization, "--target=x86_64-unknown-linux-gnu", "--run", "-o"])
                .arg(binary)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(String::from_utf8(output.stdout).unwrap(), "generic identities checked\n");
        }
    }
    fs::remove_dir_all(root).unwrap();
}
