use crate::lexer::token::*;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,
    pub line: usize,
}

impl Token {
    pub fn new(token_type: TokenType, lexeme: String, line: usize) -> Self {
        Token {
            token_type,
            lexeme,
            line,
        }
    }
}

impl Default for Token {
    fn default() -> Self {
        Token {
            token_type: TokenType::Eof, // Set default token type to EOF
            lexeme: String::new(),      // The default lexeme is an empty string
            line: 0,                    // Default line number is 0
        }
    }
}

#[derive(Debug)]
pub struct Lexer<'a> {
    pub(crate) source: &'a str,
    pub(crate) current: usize,
    pub(crate) line: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Lexer<'a> {
        Lexer {
            source,
            current: 0,
            line: 1,
        }
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    fn advance(&mut self) -> char {
        self.current += 1;
        self.source.chars().nth(self.current - 1).unwrap_or('\0')
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            let c = self.peek();
            match c {
                ' ' | '\r' | '\t' => {
                    self.advance();
                }
                '\n' => {
                    self.line += 1;
                    self.advance();
                }
                _ => break,
            }
        }
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.source.chars().nth(self.current).unwrap_or('\0')
        }
    }

    fn match_next(&mut self, expected: char) -> bool {
        if self.is_at_end() {
            return false;
        }
        if self.peek() != expected {
            return false;
        }
        self.advance();
        true
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token();

            if token.token_type == TokenType::Eof {
                tokens.push(token);
                break;
            }
            tokens.push(token);
        }
        tokens
    }

    /*
    pub fn consume(&mut self) {
        if let Some(current_char) = self.source.chars().nth(self.current) {
            if current_char == '\n' {
                self.line += 1;
            }
            println!("Consuming character: {}, at position: {}", current_char, self.current);
            self.current += 1;
        }
    }

    pub fn consume_n(&mut self, n: usize) {
        for _ in 0..n {
            self.consume();
        }
        println!("Consumed {} characters, current position: {}", n, self.current);
    }
     */

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        if self.is_at_end() {
            return Token {
                token_type: TokenType::Eof,
                lexeme: String::new(),
                line: self.line,
            };
        }

        let c = self.advance();

        match c {
            '+' => {
                if self.match_next('+') {
                    Token {
                        token_type: TokenType::Increment,
                        lexeme: "++".to_string(),
                        line: self.line,
                    }
                } else {
                    Token {
                        token_type: TokenType::Plus,
                        lexeme: "+".to_string(),
                        line: self.line,
                    }
                }
            },
            '-' => {
                if self.match_next('-') {
                    Token {
                        token_type: TokenType::Decrement,
                        lexeme: "--".to_string(),
                        line: self.line,
                    }
                } else {
                    Token {
                        token_type: TokenType::Minus,
                        lexeme: "-".to_string(),
                        line: self.line,
                    }
                }
            },
            '*' => {
                Token {
                    token_type: TokenType::Star,
                    lexeme: "*".to_string(),
                    line: self.line,
                }
            } ,
            '.' => {
                Token {
                    token_type: TokenType::Dot,
                    lexeme: ".".to_string(),
                    line: self.line,
                }
            },
            '/' => {
                Token {
                    token_type: TokenType::Div,
                    lexeme: "/".to_string(),
                    line: self.line,
                }
            },
            ';' => {
                Token {
                    token_type: TokenType::SemiColon,
                    lexeme: ";".to_string(),
                    line: self.line,
                }
            },
            ':' => {
                Token {
                    token_type: TokenType::Colon,
                    lexeme: ":".to_string(),
                    line: self.line,
                }
            },
            '<' => {
                if self.match_next('=') {
                    Token {
                        token_type: TokenType::LchevrEq,
                        lexeme: "<=".to_string(),
                        line: self.line,
                    }
                } else {
                    Token {
                        token_type: TokenType::Lchevr,
                        lexeme: "<".to_string(),
                        line: self.line,
                    }
                }

            },
            '>' => {
                if self.match_next('=') {
                    Token {
                        token_type: TokenType::RchevrEq,
                        lexeme: ">=".to_string(),
                        line: self.line,
                    }
                } else {
                    Token {
                        token_type: TokenType::Rchevr,
                        lexeme: ">".to_string(),
                        line: self.line,
                    }
                }

            },
            '(' => {
                Token {
                    token_type: TokenType::Lparen,
                    lexeme: "(".to_string(),
                    line: self.line,
                }
            },
            ')' => {
                Token {
                    token_type: TokenType::Rparen,
                    lexeme: ")".to_string(),
                    line: self.line,
                }
            },
            '{' => {
                Token {
                    token_type: TokenType::Lbrace,
                    lexeme: "{".to_string(),
                    line: self.line,
                }
            },
            '}' => {
                Token {
                    token_type: TokenType::Rbrace,
                    lexeme: "}".to_string(),
                    line: self.line,
                }
            },
            '[' => {
                Token {
                    token_type: TokenType::Lbrack,
                    lexeme: "[".to_string(),
                    line: self.line,
                }
            },
            ']' => {
                Token {
                    token_type: TokenType::Rbrack,
                    lexeme: "]".to_string(),
                    line: self.line,
                }
            },
            '=' => {
                if self.match_next('=') {
                    Token {
                        token_type: TokenType::EqualTwo,
                        lexeme: "==".to_string(),
                        line: self.line,
                    }
                } else {
                    Token {
                        token_type: TokenType::Equal,
                        lexeme: "=".to_string(),
                        line: self.line,
                    }
                }
            },
            '&' => {
                if self.match_next('&') {
                    Token {
                        token_type: TokenType::LogicalAnd,
                        lexeme: "&&".to_string(),
                        line: self.line,
                    }
                } else {
                    Token {
                        token_type: TokenType::BitwiseAnd,
                        lexeme: "&".to_string(),
                        line: self.line,
                    }
                }
            },
            '|' => {
                if self.match_next('|') {
                    Token {
                        token_type: TokenType::LogicalOr,
                        lexeme: "||".to_string(),
                        line: self.line,
                    }
                } else {
                    Token {
                        token_type: TokenType::BitwiseOr,
                        lexeme: "|".to_string(),
                        line: self.line,
                    }
                }
            },
            '!' => {
                if self.match_next('=') {
                    Token {
                        token_type: TokenType::NotEqual,
                        lexeme: "!=".to_string(),
                        line: self.line,
                    }
                } else if self.match_next('&') {
                    Token {
                        token_type: TokenType::Nand,
                        lexeme: "!&".to_string(),
                        line: self.line,
                    }
                } else if self.match_next('|') {
                    Token {
                        token_type: TokenType::Nor,
                        lexeme: "!|".to_string(),
                        line: self.line,
                    }
                } else {
                    Token {
                        token_type: TokenType::Not,
                        lexeme: "!".to_string(),
                        line: self.line,
                    }
                }
            },
            '^' => {
                Token {
                    token_type: TokenType::Xor,
                    lexeme: "^".to_string(),
                    line: self.line,
                }
            },
            '~' => {
                if self.match_next('^') {
                    Token {
                        token_type: TokenType::Xnor,
                        lexeme: "~^".to_string(),
                        line: self.line,
                    }
                } else {
                    Token {
                        token_type: TokenType::BitwiseNot,
                        lexeme: "~".to_string(),
                        line: self.line,
                    }
                }
            },
            '?' => {
                if self.match_next('?') {
                    Token {
                        token_type: TokenType::NullCoalesce,
                        lexeme: "??".to_string(),
                        line: self.line,
                    }
                } else {
                     Token {
                        token_type: TokenType::Condition,
                        lexeme: "?".to_string(),
                        line: self.line,
                    }
                }
            },
            ',' => {
                Token {
                    token_type: TokenType::Comma,
                    lexeme: ",".to_string(),
                    line: self.line,
                }
            },
            '"' => {
                let string_value = self.string();
                Token {
                    token_type: TokenType::String(string_value.clone()),
                    lexeme: format!("\"{}\"", string_value),
                    line: self.line,
                }
            },
            'a'..='z' | 'A'..='Z' => {
                let identifier = self.identifier();
                match identifier.as_str() {
                    "fun" => {
                        Token {
                            token_type: TokenType::Fun,
                            lexeme: "fun".to_string(),
                            line: self.line,
                        }
                    },
                    "var" => {
                        Token {
                            token_type: TokenType::Var,
                            lexeme: "var".to_string(),
                            line: self.line,
                        }
                    },
                    "imm" => {
                        Token {
                            token_type: TokenType::Imm,
                            lexeme: "imm".to_string(),
                            line: self.line,
                        }
                    }
                    "const" => {
                        Token {
                            token_type: TokenType::Const,
                            lexeme: "const".to_string(),
                            line: self.line,
                        }
                    },
                    "if" => {
                        Token {
                            token_type: TokenType::If,
                            lexeme: "if".to_string(),
                            line: self.line,
                        }
                    },
                    "else" => {
                        Token {
                            token_type: TokenType::Else,
                            lexeme: "else".to_string(),
                            line: self.line,
                        }
                    },
                    "while" => {
                        Token {
                            token_type: TokenType::While,
                            lexeme: "while".to_string(),
                            line: self.line,
                        }
                    },
                    "for" => {
                        Token {
                            token_type: TokenType::For,
                            lexeme: "for".to_string(),
                            line: self.line,
                        }
                    },
                    "module" => {
                        Token {
                            token_type: TokenType::Module,
                            lexeme: "module".to_string(),
                            line: self.line,
                        }
                    },
                    "class" => {
                        Token {
                            token_type: TokenType::Class,
                            lexeme: "class".to_string(),
                            line: self.line,
                        }
                    },
                    "in" => {
                        Token {
                            token_type: TokenType::In,
                            lexeme: "in".to_string(),
                            line: self.line,
                        }
                    },
                    "is" => {
                        Token {
                            token_type: TokenType::Is,
                            lexeme: "is".to_string(),
                            line: self.line,
                        }
                    },
                    "rol" => {
                        Token {
                            token_type: TokenType::Rol,
                            lexeme: "rol".to_string(),
                            line: self.line,
                        }
                    },
                    "ror" => {
                        Token {
                            token_type: TokenType::Ror,
                            lexeme: "ror".to_string(),
                            line: self.line,
                        }
                    },
                    "xnand" => {
                        Token {
                            token_type: TokenType::Xnand,
                            lexeme: "xnand".to_string(),
                            line: self.line,
                        }
                    },
                    "import" => {
                        Token {
                            token_type: TokenType::Import,
                            lexeme: "import".to_string(),
                            line: self.line,
                        }
                    },
                    "return" => {
                        Token {
                            token_type: TokenType::Return,
                            lexeme: "return".to_string(),
                            line: self.line,
                        }
                    },
                    "continue" => {
                        Token {
                            token_type: TokenType::Continue,
                            lexeme: "continue".to_string(),
                            line: self.line,
                        }
                    },
                    "print" => {
                        Token {
                            token_type: TokenType::Print,
                            lexeme: "print".to_string(),
                            line: self.line,
                        }
                    },
                    "input" => {
                        Token {
                            token_type: TokenType::Input,
                            lexeme: "input".to_string(),
                            line: self.line,
                        }
                    },
                    "println" => {
                        Token {
                            token_type: TokenType::Println,
                            lexeme: "println".to_string(),
                            line: self.line,
                        }
                    },
                    "match" => {
                        Token {
                            token_type: TokenType::Match,
                            lexeme: "match".to_string(),
                            line: self.line,
                        }
                    },
                    "char" => {
                        Token {
                            token_type: TokenType::TypeChar,
                            lexeme: "char".to_string(),
                            line: self.line,
                        }
                    },
                    "byte" => {
                        Token {
                            token_type: TokenType::TypeByte,
                            lexeme: "byte".to_string(),
                            line: self.line,
                        }
                    },
                    "ptr" => {
                        Token {
                            token_type: TokenType::TypePointer(Box::new(TokenType::Fun)),
                            lexeme: "ptr".to_string(),
                            line: self.line,
                        }
                    },
                    "array" => {
                        Token {
                            token_type: TokenType::TypeArray(Box::new(TokenType::Fun), 0),
                            lexeme: "array".to_string(),
                            line: self.line,
                        }
                    },
                    "isz" => {
                        Token {
                            token_type: TokenType::TokenTypeInt(IntegerType::ISZ),
                            lexeme: "isz".to_string(),
                            line: self.line,
                        }
                    },
                    "i8" => {
                        Token {
                            token_type: TokenType::TokenTypeInt(IntegerType::I8),
                            lexeme: "i8".to_string(),
                            line: self.line,
                        }
                    },
                    "i16" => {
                        Token {
                            token_type: TokenType::TokenTypeInt(IntegerType::I16),
                            lexeme: "i16".to_string(),
                            line: self.line,
                        }
                    },
                    "i32" => {
                        Token {
                            token_type: TokenType::TokenTypeInt(IntegerType::I32),
                            lexeme: "i32".to_string(),
                            line: self.line,
                        }
                    },
                    "i64" => {
                        Token {
                            token_type: TokenType::TokenTypeInt(IntegerType::I64),
                            lexeme: "i64".to_string(),
                            line: self.line,
                        }
                    },
                    "i128" => {
                        Token {
                            token_type: TokenType::TokenTypeInt(IntegerType::I128),
                            lexeme: "i128".to_string(),
                            line: self.line,
                        }
                    },
                    "i256" => {
                        Token {
                            token_type: TokenType::TokenTypeInt(IntegerType::I256),
                            lexeme: "i256".to_string(),
                            line: self.line,
                        }
                    },
                    "i512" => {
                       Token {
                            token_type: TokenType::TokenTypeInt(IntegerType::I512),
                            lexeme: "i512".to_string(),
                            line: self.line,
                        }
                    },
                    "i1024" => {
                        Token {
                            token_type: TokenType::TokenTypeInt(IntegerType::I1024),
                            lexeme: "i1024".to_string(),
                            line: self.line,
                        }
                    },
                    "usz" => Token {
                        token_type: TokenType::TokenTypeUint(UnsignedIntegerType::USZ),
                        lexeme: "usz".to_string(),
                        line: self.line,
                    },
                    "u8" => Token {
                        token_type: TokenType::TokenTypeUint(UnsignedIntegerType::U8),
                        lexeme: "u8".to_string(),
                        line: self.line,
                    },
                    "u16" => Token {
                        token_type: TokenType::TokenTypeUint(UnsignedIntegerType::U16),
                        lexeme: "u16".to_string(),
                        line: self.line,
                    },
                    "u32" => Token {
                        token_type: TokenType::TokenTypeUint(UnsignedIntegerType::U32),
                        lexeme: "u32".to_string(),
                        line: self.line,
                    },
                    "u64" => Token {
                        token_type: TokenType::TokenTypeUint(UnsignedIntegerType::U64),
                        lexeme: "u64".to_string(),
                        line: self.line,
                    },
                    "u128" => Token {
                        token_type: TokenType::TokenTypeUint(UnsignedIntegerType::U128),
                        lexeme: "u128".to_string(),
                        line: self.line,
                    },
                    "u256" => Token {
                        token_type: TokenType::TokenTypeUint(UnsignedIntegerType::U256),
                        lexeme: "u256".to_string(),
                        line: self.line,
                    },
                    "u512" => Token {
                        token_type: TokenType::TokenTypeUint(UnsignedIntegerType::U512),
                        lexeme: "u512".to_string(),
                        line: self.line,
                    },
                    "u1024" => Token {
                        token_type: TokenType::TokenTypeUint(UnsignedIntegerType::U1024),
                        lexeme: "u1024".to_string(),
                        line: self.line,
                    },
                    "f32" => Token {
                        token_type: TokenType::TokenTypeFloat(FloatType::F32),
                        lexeme: "f32".to_string(),
                        line: self.line,
                    },
                    "f64" => Token {
                        token_type: TokenType::TokenTypeFloat(FloatType::F64),
                        lexeme: "f64".to_string(),
                        line: self.line,
                    },
                    "f128" => Token {
                        token_type: TokenType::TokenTypeFloat(FloatType::F128),
                        lexeme: "f128".to_string(),
                        line: self.line,
                    },
                    "f256" => Token {
                        token_type: TokenType::TokenTypeFloat(FloatType::F256),
                        lexeme: "f256".to_string(),
                        line: self.line,
                    },
                    "f512" => Token {
                        token_type: TokenType::TokenTypeFloat(FloatType::F512),
                        lexeme: "f512".to_string(),
                        line: self.line,
                    },
                    "f1024" => Token {
                        token_type: TokenType::TokenTypeFloat(FloatType::F1024),
                        lexeme: "f1024".to_string(),
                        line: self.line,
                    },
                    "str" => {
                        Token {
                            token_type: TokenType::TypeString,
                            lexeme: "str".to_string(),
                            line: self.line,
                        }
                    },
                    "break" => {
                        Token {
                            token_type: TokenType::Break,
                            lexeme: "break".to_string(),
                            line: self.line,
                        }
                    },
                    _ => {
                        Token {
                            token_type: TokenType::Identifier(identifier.clone()),
                            lexeme: identifier,
                            line: self.line,
                        }
                    }
                }
            },
            '0'..='9' => {
                let mut num_str = self.number().to_string(); // Converting Numbers to Strings
                if self.peek() == '.' { // If the following characters are dots, handle mistakes
                    num_str.push('.'); // Add a dot
                    self.advance(); // turning over a mole
                    // deal with numbers that can follow a mistake
                    while self.peek().is_digit(10) {
                        num_str.push(self.advance()); // Keep adding numbers
                    }
                }

                // Safe handling of errors in accidental parsing
                let token_type = match num_str.parse::<f64>() {
                    Ok(n) => {
                        if n.fract() == 0.0 {
                            TokenType::Number(n as i64)
                        } else {
                            TokenType::Float(n)
                        }
                    }
                    Err(_) => TokenType::Float(0.0),
                };

                Token {
                    token_type,
                    lexeme: num_str, // Save real string to lexeme
                    line: self.line,
                }
            },
            _ => {
                eprintln!("[eprintln] Unexpected character: {}", c);
                panic!("[panic] Unexpected character: {}", c);
            }
        }
    }

    // Helper methods to create tokens
    fn create_int_token(&self, int_type: IntegerType, lexeme: String) -> Token {
        Token {
            token_type: TokenType::TokenTypeInt(int_type),
            lexeme,
            line: self.line,
        }
    }

    fn create_float_token(&self, float_type: FloatType, lexeme: String) -> Token {
        Token {
            token_type: TokenType::TokenTypeFloat(float_type),
            lexeme,
            line: self.line,
        }
    }

    fn create_identifier_token(&self, identifier: String) -> Token {
        Token {
            token_type: TokenType::Identifier(identifier.clone()),
            lexeme: identifier,
            line: self.line,
        }
    }

    // Add string literal processing function
    fn string(&mut self) -> String {
        if self.peek() == '"' {
            self.advance();
        }

        let mut string_literal = String::new();

        while !self.is_at_end() && self.peek() != '"' {
            string_literal.push(self.advance());
        }

        if self.is_at_end() {
            panic!("Unterminated string.");
        }

        self.advance(); // closing quote

        string_literal
    }

    fn identifier(&mut self) -> String {
        let start = if self.current > 0 {
            self.current - 1
        } else {
            0
        };

        while !self.is_at_end() && self.peek().is_alphanumeric() {
            self.advance();
        }

        self.source[start..self.current].to_string()
    }

    fn number(&mut self) -> i64 {
        let start = self.current - 1;
        while !self.is_at_end() && self.peek().is_numeric() {
            self.advance();
        }

        let number_str = &self.source[start..self.current];
        i64::from_str(number_str).unwrap_or_else(|_| 0)
    }
}
