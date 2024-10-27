mod lexer;
mod parser;
mod ast;
mod type_checker;
mod codegen;

fn main() {
    let source_code = r#"
    fun main() {
        println("hello world");
    }
    "#;

    // 1. 토크나이징 단계
    let tokens = lexer::tokenize(source_code).expect("토크나이징 실패");

    // 2. 파싱 단계
    let ast = parser::parse(&tokens).expect("파싱 실패");

    // 3. 타입 검사 단계
    type_checker::check(&ast).expect("타입 검사 실패");

    // 4. 코드 생성 단계
    codegen::generate(&ast).expect("코드 생성 실패");
}
