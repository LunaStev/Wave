//! Regression coverage for variable declarations and generic type syntax.

use lexer::Lexer;
use parser::ast::{ASTNode, Expression, StatementNode, Visibility, WaveType};
use parser::parse_syntax_only;

fn parse_ok(src: &str) {
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().expect("lex should succeed");
    let parsed = parse_syntax_only(&tokens);
    if let Err(err) = parsed {
        let mut dump = String::new();
        for (idx, t) in tokens.iter().enumerate() {
            dump.push_str(&format!(
                "{:03}: line={} {:?} lexeme=`{}`\n",
                idx, t.line, t.token_type, t.lexeme
            ));
        }
        panic!(
            "parse failed: {:?}\nsource:\n{}\ntokens:\n{}",
            err, src, dump
        );
    }
}

fn parse_nodes(src: &str) -> Vec<ASTNode> {
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize().expect("lex should succeed");
    parse_syntax_only(&tokens).expect("parse should succeed")
}

#[test]
fn parses_var_in_function_body() {
    parse_ok(
        r#"
fun main() {
    var x: i32;
    return;
}
"#,
    );
}

#[test]
fn parses_multigeneric_function_and_types() {
    parse_ok(
        r#"
struct Pair<A, B> {
    first: A;
    second: B;
}

fun make_pair<A, B>(a: A, b: B) -> Pair<A, B> {
    var pair_value: Pair<A, B>;
    return pair_value;
}
"#,
    );
}

#[test]
fn parses_public_declarations_and_import_forms() {
    let nodes = parse_nodes(
        r#"
import("add");
import("add::math");
import("./helpers" as helpers);
import("add")::{sum, Point,};

pub fun sum(a: i32, b: i32) -> i32 { return a + b; }
pub struct Point {}
fun main() {
    var a = add::sum(1, 2);
    var b = sum(1, 2);
    var p = Point();
}
"#,
    );

    let imports = nodes
        .iter()
        .filter_map(|node| match node {
            ASTNode::Statement(StatementNode::Import(import)) => Some(import),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(imports.len(), 4);
    assert_eq!(imports[0].path, "add");
    assert_eq!(imports[1].path, "add::math");
    assert_eq!(imports[2].alias.as_deref(), Some("helpers"));
    assert_eq!(imports[3].selections, ["sum", "Point"]);

    assert!(matches!(
        &nodes[4],
        ASTNode::Function(function) if function.visibility == Visibility::Public
    ));
    assert!(matches!(
        &nodes[5],
        ASTNode::Struct(structure) if structure.visibility == Visibility::Public
    ));

    let ASTNode::Function(main) = &nodes[6] else {
        panic!("expected main function");
    };
    let ASTNode::Variable(first) = &main.body[0] else {
        panic!("expected first variable");
    };
    assert_eq!(first.type_name, WaveType::Infer);
    assert!(matches!(
        first.initial_value.as_ref(),
        Some(Expression::FunctionCall { name, .. }) if name == "add::sum"
    ));
}

#[test]
fn rejects_public_main() {
    let mut lexer = Lexer::new("pub fun main() {}\n");
    let tokens = lexer.tokenize().expect("lex should succeed");
    let error = parse_syntax_only(&tokens).expect_err("pub main must fail");
    assert_eq!(error.message(), "entry function `main` cannot be public");
}

#[test]
fn public_visibility_is_independent_from_c_abi_export() {
    let nodes = parse_nodes(
        "pub export(c) fun shared_value() -> i32 { return 1; }\nexport(c) fun abi_only() -> i32 { return 2; }\n",
    );
    let ASTNode::Function(shared) = &nodes[0] else {
        panic!("expected shared export");
    };
    let ASTNode::Function(abi_only) = &nodes[1] else {
        panic!("expected ABI-only export");
    };
    assert_eq!(shared.visibility, Visibility::Public);
    assert!(shared.export.is_some());
    assert_eq!(abi_only.visibility, Visibility::Private);
    assert!(abi_only.export.is_some());
}
