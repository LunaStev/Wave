// SPDX-License-Identifier: MPL-2.0
use lexer::Lexer;
use parser::{parse_syntax_only, parse_syntax_with_spans};

#[test]
fn parser_depth_budget_is_restored_after_errors_and_counts_mixed_shapes() {
    for expression in [
        format!("{}1{}", "(".repeat(129), ")".repeat(129)),
        format!("{}1{}", "f([(".repeat(43), ")])".repeat(43)),
        format!("{}1", "- ".repeat(129)),
        format!("{}1", "x = ".repeat(129)),
    ] {
        let source = format!("fun main() {{ {expression}; }}");
        let tokens = Lexer::new_with_file(&source, "depth.wave")
            .tokenize()
            .unwrap();
        let error = parse_syntax_with_spans(&tokens).unwrap_err();
        assert!(error.message().contains("maximum of 128"), "{error:?}");
        assert_eq!(error.span().unwrap().file, "depth.wave");
        let tokens = Lexer::new("fun main() { (1); }").tokenize().unwrap();
        assert!(parse_syntax_only(&tokens).is_ok());
    }
}
