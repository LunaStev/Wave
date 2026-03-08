use lexer::Lexer;
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
