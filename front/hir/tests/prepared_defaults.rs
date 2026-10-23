// SPDX-License-Identifier: MPL-2.0
use hir::{conversions::ConversionKind, TypedProgram};
use parser::ast::{ASTNode, Expression, StatementNode, WaveType};

fn syntax(source: &str, file: &str) -> Vec<ASTNode> {
    let tokens = lexer::Lexer::new_with_file(source, file).tokenize().unwrap();
    parser::parse_syntax_with_spans(&tokens).unwrap()
}

fn function_mut<'a>(nodes: &'a mut [ASTNode], name: &str) -> &'a mut parser::ast::FunctionNode {
    for node in nodes {
        let mut node = node;
        while let ASTNode::Located { value, .. } = node {
            node = value;
        }
        if let ASTNode::Function(function) = node {
            if function.name == name {
                return function;
            }
        }
    }
    panic!("missing function {name}");
}

fn default_expression(source: &str) -> Expression {
    let mut nodes =
        syntax(&format!("fun expression() -> i32 {{ return {source}; }}"), "default.wave");
    let function = function_mut(&mut nodes, "expression");
    let ASTNode::Statement(StatementNode::Return(Some(expression))) = function.body[0].unspanned()
    else {
        panic!("return expression expected");
    };
    expression.clone()
}

#[test]
fn every_call_owns_typed_converted_defaults_with_declaration_provenance() {
    let mut nodes = syntax(
        "fun pick(n: i64 = 7) -> i64 { return n; } fun probe() -> i64 { return pick() + pick(); }",
        "calls.wave",
    );
    // Source syntax remains literal-only. The HIR boundary also accepts
    // semantically valid generated expressions, retaining each cast in order.
    function_mut(&mut nodes, "pick").parameters[0].initial_value =
        Some(default_expression("(257 as u8) as i32"));
    let program = TypedProgram::lower(nodes).unwrap();
    program.verify_conversions().unwrap();
    let mut ids = Vec::new();
    parser::ast::visit::walk_nodes(program.syntax(), &mut |expression| {
        if let Expression::FunctionCall { name, args, .. } = expression {
            if name == "pick" {
                assert_eq!(args.len(), 1);
                let id = program.expression_id(&args[0]).unwrap();
                ids.push(id);
                let fact = program.numeric_expression(id).unwrap();
                assert_eq!(fact.result_type, WaveType::Int(64));
                assert_eq!(fact.conversions.last().unwrap().kind, ConversionKind::SignExtend);
                let span = program.expression_span(id).unwrap();
                assert_eq!(span.file, "default.wave");
                assert!(span
                    .expansion
                    .iter()
                    .any(|note| note.contains("default argument for pick")));
                let mut cast_types = Vec::new();
                parser::ast::visit::walk_expression(&args[0], &mut |child| {
                    if let Expression::Cast { target_type, .. } = child {
                        cast_types.push(target_type.clone());
                    }
                });
                assert_eq!(cast_types, vec![WaveType::Int(32), WaveType::Uint(8)]);
            }
        }
    });
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1]);
}

#[test]
fn nested_generated_default_calls_are_fully_prepared_before_semantic_analysis() {
    let mut nodes = syntax(
        "fun inner(n: i32 = 9) -> i32 { return n; } fun outer(n: i32 = 1) -> i32 { return n; } fun probe() -> i32 { return outer(); }",
        "nested.wave",
    );
    function_mut(&mut nodes, "outer").parameters[0].initial_value =
        Some(default_expression("inner()"));
    let program = TypedProgram::lower(nodes).unwrap();
    program.verify_conversions().unwrap();
    parser::ast::visit::walk_nodes(program.syntax(), &mut |expression| {
        if let Expression::FunctionCall { args, .. } = expression {
            assert_eq!(args.len(), 1);
            assert!(program.numeric_expression_of(&args[0]).is_some());
        }
    });
}

#[test]
fn existing_arity_and_default_diagnostics_remain_frontend_errors() {
    for (source, expected) in [
        ("fun pick(n: i32) -> i32 { return n; } fun probe() -> i32 { return pick(); }", "argument"),
        (
            "fun pick(n: i32 = 7) -> i32 { return n; } fun probe() -> i32 { return pick(1, 2); }",
            "argument",
        ),
        ("fun pick(n: i32 = \"bad\") -> i32 { return n; }", "default"),
        ("fun pick(n: i32 = 1, x: i32) -> i32 { return n; }", "required parameter"),
    ] {
        let error = TypedProgram::lower(syntax(source, "invalid.wave")).unwrap_err();
        assert!(error.diagnostic().message.contains(expected), "{error:?}");
        assert_eq!(error.diagnostic().code, "E3001");
    }
}

#[test]
fn recursive_generated_defaults_reuse_the_existing_expression_depth_limit() {
    let mut nodes = syntax("fun pick(n: i32 = 1) -> i32 { return n; }", "cycle.wave");
    function_mut(&mut nodes, "pick").parameters[0].initial_value =
        Some(default_expression("pick()"));
    let error = TypedProgram::lower(nodes).unwrap_err();
    assert!(error.diagnostic().message.contains("maximum of 128"), "{error:?}");
    assert_eq!(error.diagnostic().span.as_ref().unwrap().file, "default.wave");
}
