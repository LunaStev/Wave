use std::iter::Peekable;
use crate::lexer::{Token, TokenType};
use crate::parser::ast::{BinaryOperator, Expression, FormatPart, Literal};

pub fn parse_format_string(s: &str) -> Vec<FormatPart> {
    let mut parts = Vec::new();
    let mut buffer = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '{' {
            if let Some('}') = chars.peek() {
                chars.next();
                if !buffer.is_empty() {
                    parts.push(FormatPart::Literal(buffer.clone()));
                    buffer.clear();
                }
                parts.push(FormatPart::Placeholder);
            } else {
                buffer.push(c);
            }
        } else {
            buffer.push(c);
        }
    }

    if !buffer.is_empty() {
        parts.push(FormatPart::Literal(buffer));
    }

    parts
}

pub fn parse_expression<'a, T>(tokens: &mut Peekable<T>) -> Option<Expression>
where
    T: Iterator<Item = &'a Token>,
{
    parse_logical_expression(tokens)
}

pub fn parse_logical_expression<'a, T>(tokens: &mut Peekable<T>) -> Option<Expression>
where
    T: Iterator<Item = &'a Token>,
{
    let mut left = parse_relational_expression(tokens)?;

    while let Some(token) = tokens.peek() {
        match token.token_type {
            TokenType::LogicalAnd |
            TokenType::BitwiseAnd |
            TokenType::LogicalOr |
            TokenType::BitwiseOr => {
                let op = match token.token_type {
                    TokenType::LogicalAnd => Operator::LogicalAnd,
                    TokenType::BitwiseAnd => Operator::BitwiseAnd,
                    TokenType::LogicalOr => Operator::LogicalOr,
                    TokenType::BitwiseOr => Operator::BitwiseOr,
                    _ => unreachable!(),
                };
                tokens.next();

                let right = parse_relational_expression(tokens)?;
                left = Expression::BinaryExpression {
                    left: Box::new(left),
                    operator: op,
                    right: Box::new(right),
                };
            }
            _ => break,
        }
    }
    Some(left)
}

pub fn parse_relational_expression<'a, T>(tokens: &mut Peekable<T>) -> Option<Expression>
where
    T: Iterator<Item = &'a Token>,
{
    let mut left = parse_additive_expression(tokens)?;

    while let Some(token) = tokens.peek() {
        match token.token_type {
            TokenType::EqualTwo |
            TokenType::NotEqual |
            TokenType::Rchevr |
            TokenType::Lchevr |
            TokenType::RchevrEq |
            TokenType::LchevrEq => {
                let op = match token.token_type {
                    TokenType::EqualTwo => Operator::Equal,
                    TokenType::NotEqual => Operator::NotEqual,
                    TokenType::Rchevr => Operator::Greater,
                    TokenType::Lchevr => Operator::Less,
                    TokenType::RchevrEq => Operator::GreaterEqual,
                    TokenType::LchevrEq => Operator::LessEqual,
                    _ => unreachable!(),
                };
                tokens.next();

                let right = parse_additive_expression(tokens)?;
                left = Expression::BinaryExpression {
                    left: Box::new(left),
                    operator: op,
                    right: Box::new(right),
                };
            }
            _ => break,
        }
    }
    Some(left)
}


pub fn parse_additive_expression<'a, T>(tokens: &mut Peekable<T>) -> Option<Expression>
where
    T: Iterator<Item = &'a Token>,
{
    let mut left = parse_multiplicative_expression(tokens)?;

    while let Some(token) = tokens.peek() {
        match token.token_type {
            TokenType::Plus | TokenType::Minus => {
                let op = match token.token_type {
                    TokenType::Plus => Operator::Add,
                    TokenType::Minus => Operator::Subtract,
                    _ => unreachable!(),
                };
                tokens.next();

                let right = parse_multiplicative_expression(tokens)?;
                left = Expression::BinaryExpression {
                    left: Box::new(left),
                    operator: op,
                    right: Box::new(right),
                };
            }
            _ => break,
        }
    }
    Some(left)
}

pub fn parse_multiplicative_expression<'a, T>(tokens: &mut Peekable<T>) -> Option<Expression>
where
    T: Iterator<Item = &'a Token>,
{
    let mut left = parse_primary_expression(tokens)?;

    while let Some(token) = tokens.peek() {
        match token.token_type {
            TokenType::Star | TokenType::Div => {
                let op = match token.token_type {
                    TokenType::Star => BinaryOperator::Multiply,
                    TokenType::Div => BinaryOperator::Divide,
                    _ => unreachable!(),
                };
                tokens.next();

                let right = parse_primary_expression(tokens)?;
                left = Expression::BinaryExpression {
                    left: Box::new(left),
                    operator: op,
                    right: Box::new(right),
                };
            }
            _ => break,
        }
    }
    Some(left)
}

pub fn parse_primary_expression<'a, T>(tokens: &mut Peekable<T>) -> Option<Expression>
where
    T: Iterator<Item = &'a Token>,
{
    let token = tokens.peek()?;

    match &token.token_type {
        TokenType::Number(value) => Some(Expression::Literal(Literal::Number(*value as f64))),
        TokenType::Identifier(name) => Some(Expression::Variable(name.clone())),
        TokenType::Lparen => {
            let expr = parse_expression(tokens)?;
            if tokens.peek()?.token_type != TokenType::Rparen {
                println!("Error: Expected closing ')'");
                return None;
            }
            tokens.next();
            Some(Expression::Grouped(Box::new(expr)))
        }
        _ => {
            println!("Error: Expected primary expression");
            None
        }
    }
}