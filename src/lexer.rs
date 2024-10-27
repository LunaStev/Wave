#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Fun,
    Identifier(String),
    LParen,
    RParen,
    LBrace,
    RBrace,
    Println,
    StringLiteral(String),
    Semicolon,
    // 더 많은 토큰들...
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&c) = chars.peek() {
        match c {
            '(' => { tokens.push(Token::LParen); chars.next(); },
            ')' => { tokens.push(Token::RParen); chars.next(); },
            '{' => { tokens.push(Token::LBrace); chars.next(); },
            '}' => { tokens.push(Token::RBrace); chars.next(); },
            '"' => {
                chars.next(); // 처음 따옴표
                let literal: String = chars.by_ref().take_while(|&c| c != '"').collect();
                tokens.push(Token::StringLiteral(literal));
                chars.next(); // 끝 따옴표
            },
            _ if c.is_alphabetic() => {
                let ident: String = chars.by_ref().take_while(|c| c.is_alphanumeric()).collect();
                match ident.as_str() {
                    "fun" => tokens.push(Token::Fun),
                    "println" => tokens.push(Token::Println),
                    _ => tokens.push(Token::Identifier(ident)),
                }
            },
            _ => { chars.next(); }
        }
    }
    Ok(tokens)
}
