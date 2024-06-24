#![allow(clippy::upper_case_acronyms, clippy::result_large_err)]

use pest::{self, Parser, parses_to};

use crate::ast::{Node, Operator};

#[derive(pest_derive::Parser)]
#[grammar = "grammar.pest"]
struct CalcParser;

pub fn parse(source: &str) -> std::result::Result<Vec<Node>, pest::error::Error<Rule>> {
    let mut ast = vec![];
    let pairs = CalcParser::parse(Rule::Program, source)?;
    for pair in pairs {
        if let Rule::Expr = pair.as_rule() {
            ast.push(build_ast_from_expr(pair));
        }
    } Ok(ast)
}

fn build_ast_from_expr(pair: pest::iterators::Pair<Rule>) -> Node {
    match pair.ad_rule() {
        Rule::Expr => build_ast_from_expr(pair.into_inner().next().unwrap()),
        Rule::UnaryExpr => {
            let mut pair = pair.into_inner();
            let op = pair.next().unwrap();
            let child = pair.next().unwrap();
            let child = build_ast_from_expr(child);
            parse_unary_expr(op, child)
        }

        Rule::BinaryExpr => {
            let mut pair = pair.into_inner();
            let lhspair = pair.next().unwrap();
            let mut lhs = build_ast_from_expr(lhspair);
            let op = pair.next().unwrap();
            let rhspair = pair.next().unwrap();
            let mut rhs = build_ast_from_term(rhspair);
            let mut retval = parse_binary_expr(op, lhs, rhs);

            loop {
                let pair_buf = pair.next();

                if let Some(op) = pair_buf {
                    lhs = retval;
                    rhs = build_ast_from_term(pair.next().unwrap());
                    retval = parse_binary_expr(op, lhs, rhs);
                } else {
                    return retval;
                }
            }
        } unknown => panic!("Unknown expr: {:?}", unknown),
    }
}

fn build_ast_from_term(pair: pest::iterators::Pair<Rule>) -> Node {
    match pair.as_rule() {
        Rule::Int => {
            let istr = pair.as_str();
            let (sign, istr) = match &istr[..1] {
                "-" => (-1, &istr[1..]),
                _ => (1, istr),
            };
            let int: i32 = istr.parse().unwrap();
            Node::Int(sign * int)
        }
        Rule::Expr => build_ast_from_expr(pair),
        unknown => panic!("Unknown term: {:?}", unknown),
    }
}

/**
 * 단항 표현식을 파싱하는 함수
 *
 * @param pair
 *    pest 라이브러리의 Pair 객체, 단항 연산자를 나타냄
 *
 * @param child
 *    단항 연산자의 피연산자를 나타내는 Node 객체
 *
 * @return Node
 *    파싱된 단항 표현식을 나타내는 Node 객체
 */
fn parse_unary_expr(pair: pest::iterators::Pair<Rule>, child: Node) -> Node {
    Node::UnaryExpr {
        op: match pair.as_str() {
            "+" => Operator::PLUS,
            "-" => Operator::MINUS,
            _ => unreachable!(),
        },
        child: Box::new(child),
    }
}

/**
 * 이항 표현식을 파싱하는 함수
 *
 * @param pair
 *    pest 라이브러리의 Pair 객체, 이항 연산자를 나타냄
 *
 * @param lhs
 *    왼쪽 피연산자를 나타내는 Node 객체
 *
 * @param rhs
 *    오른쪽 피연산자를 나타내는 Node 객체
 *
 * @return Node
 *    파싱된 이항 표현식을 나타내는 Node 객체
 */
fn parse_binary_expr(pair: pest::iterators::Pair<Rule>, lhs: Node, rhs: Node) -> Node {
    Node::BinaryExpr {
        op: match pair.as_str() {
            "+" => Operator::PLUS,
            "-" => Operator::MINUS,
            _ => unreachable!(),
        },
        lhs: Box::new(lhs),
        rhs: Box::new(rhs),
    }
}

#[cfg(test)]
mod tests {

}