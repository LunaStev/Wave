use pest_derive::Parser;
use crate::ast::Expr;
use crate::ast::Stmt;

#[derive(Parser)]
#[grammar = "wave.pest"]
struct MyLangParser;

pub fn parse_source(source: &str) -> Result<Vec<Stmt>, String> {
    let parsed = MyLangParser::parse(Rule::program, source)
        .map_err(|e| format!("Parse error: {}", e))?;

    // 여기에 AST로 변환하는 코드 추가
    // 예시로 프로그램을 표현하는 Vec<Stmt> 반환
    Ok(vec![])
}