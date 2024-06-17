use crate::lexer::{Lexer, Token};

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current_token: Token,
}

