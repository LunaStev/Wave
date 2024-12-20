use std::fmt;
use std::str::FromStr;


#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IntegerType {
    I4,
    I8,
    I16,
    I32,
    I64,
    I128,
    I256,
    I512,
    I1024,
    I2048,
    I4096,
    I8192,
    I16384,
    I32768,
    U4,
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
    U512,
    U1024,
    U2048,
    U4096,
    U8192,
    U16384,
    U32768,
    ISZ,
    USZ,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FloatType {
    F32,
    F64,
    F128,
    F256,
    F512,
    F1024,
    F2048,
    F4096,
    F8192,
    F16384,
    F32768,
}

impl fmt::Display for IntegerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            IntegerType::I4 => "i4",
            IntegerType::I8 => "i8",
            IntegerType::I16 => "i16",
            IntegerType::I32 => "i32",
            IntegerType::I64 => "i64",
            IntegerType::I128 => "i128",
            IntegerType::I256 => "i256",
            IntegerType::I512 => "i512",
            IntegerType::I1024 => "i1024",
            IntegerType::I2048 => "i2048",
            IntegerType::I4096 => "i4096",
            IntegerType::I8192 => "i8192",
            IntegerType::I16384 => "i16384",
            IntegerType::I32768 => "i32768",
            IntegerType::U4 => "u4",
            IntegerType::U8 => "u8",
            IntegerType::U16 => "u16",
            IntegerType::U32 => "u32",
            IntegerType::U64 => "u64",
            IntegerType::U128 => "u128",
            IntegerType::U256 => "u256",
            IntegerType::U512 => "u512",
            IntegerType::U1024 => "u1024",
            IntegerType::U2048 => "u2048",
            IntegerType::U4096 => "u4096",
            IntegerType::U8192 => "u8192",
            IntegerType::U16384 => "u16384",
            IntegerType::U32768 => "u32768",
            IntegerType::ISZ => "isz",
            IntegerType::USZ => "usz",
        };
        write!(f, "{}", name)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum TokenType {
    FUN,
    VAR,
    CONST,
    IF,
    ELSE,
    WHILE,
    FOR,
    IMPORT,
    RETURN,
    CONTINUE,
    INPUT,
    PRINT,
    PRINTLN,
    LOGICAL_AND,            // &&
    BITWISE_AND,            // &
    LOGICAL_OR,             // ||
    BITWISE_OR,             // |
    NOT_EQUAL,              // !=
    XOR,                    // ^
    XNOR,                   // ~^
    BITWISE_NOT,            // ~
    NAND,                   // !&
    NOR,                    // !|
    NOT,                    // !
    CONDITION,              // ?
    NULL_COALESCE,          // ??
    CONDITIONAL,            // ?:
    IN,                     // in
    IS,                     // is
    ROL,
    ROR,
    XNAND,
    TYPE_INT(IntegerType),
    TYPE_FLOAT(FloatType),
    TYPE_STRING,
    IDENTIFIER(String),
    STRING(String),
    NUMBER(i64),
    PLUS,                   // +
    INCREMENT,              // ++
    MINUS,                  // -
    DECREMENT,              // --
    STAR,                   // *
    DIV,                    // /
    EQUAL,                  // =
    EQUAL_TWO,              // ==
    COMMA,                  // ,
    SEMICOLON,              // ;
    COLON,                  // :
    LCHEVR,                 // <
    LCHEVR_EQ,              // <=
    RCHEVR,                 // >
    RCHEVR_EQ,              // >=
    LPAREN,                 // (
    RPAREN,                 // )
    LBRACE,                 // {
    RBRACE,                 // }
    LBRACK,                 // [
    RBRACK,                 // ]
    EOF,                    // End of file
}

#[derive(Debug)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,
    pub line: usize,
}

impl Default for Token {
    fn default() -> Self {
        Token {
            token_type: TokenType::EOF, // Set default token type to EOF
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

            if token.token_type == TokenType::EOF {
                tokens.push(token);
                break;
            }
            tokens.push(token);
        }
        tokens
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        if self.is_at_end() {
            return Token {
                token_type: TokenType::EOF,
                lexeme: String::new(),
                line: self.line,
            };
        }

        let c = self.advance();
        match c {
            'i' | 'u' | 'f' => {
                let type_prefix = c;
                let start = self.current - 1;

                // Collect all numeric characters
                let mut number_str = String::new();
                while !self.is_at_end() && self.peek().is_numeric() {
                    number_str.push(self.advance());
                }

                if !number_str.is_empty() {
                    let type_str = format!("{}{}", type_prefix, number_str);

                    // Handle integer types (i and u prefixes)
                    if type_prefix == 'i' || type_prefix == 'u' {
                        match type_str.as_str() {
                            // Signed integer types
                            "isz" => return self.create_int_token(IntegerType::ISZ, type_str),
                            "i4" => return self.create_int_token(IntegerType::I4, type_str),
                            "i8" => return self.create_int_token(IntegerType::I8, type_str),
                            "i16" => return self.create_int_token(IntegerType::I16, type_str),
                            "i32" => return self.create_int_token(IntegerType::I32, type_str),
                            "i64" => return self.create_int_token(IntegerType::I64, type_str),
                            "i128" => return self.create_int_token(IntegerType::I128, type_str),
                            "i256" => return self.create_int_token(IntegerType::I256, type_str),
                            "i512" => return self.create_int_token(IntegerType::I512, type_str),
                            "i1024" => return self.create_int_token(IntegerType::I1024, type_str),
                            "i2048" => return self.create_int_token(IntegerType::I2048, type_str),
                            "i4096" => return self.create_int_token(IntegerType::I4096, type_str),
                            "i8192" => return self.create_int_token(IntegerType::I8192, type_str),
                            "i16384" => return self.create_int_token(IntegerType::I16384, type_str),
                            "i32768" => return self.create_int_token(IntegerType::I32768, type_str),

                            // Unsigned integer types
                            "usz" => return self.create_int_token(IntegerType::USZ, type_str),
                            "u4" => return self.create_int_token(IntegerType::U4, type_str),
                            "u8" => return self.create_int_token(IntegerType::U8, type_str),
                            "u16" => return self.create_int_token(IntegerType::U16, type_str),
                            "u32" => return self.create_int_token(IntegerType::U32, type_str),
                            "u64" => return self.create_int_token(IntegerType::U64, type_str),
                            "u128" => return self.create_int_token(IntegerType::U128, type_str),
                            "u256" => return self.create_int_token(IntegerType::U256, type_str),
                            "u512" => return self.create_int_token(IntegerType::U512, type_str),
                            "u1024" => return self.create_int_token(IntegerType::U1024, type_str),
                            "u2048" => return self.create_int_token(IntegerType::U2048, type_str),
                            "u4096" => return self.create_int_token(IntegerType::U4096, type_str),
                            "u8192" => return self.create_int_token(IntegerType::U8192, type_str),
                            "u16384" => return self.create_int_token(IntegerType::U16384, type_str),
                            "u32768" => return self.create_int_token(IntegerType::U32768, type_str),

                            _ => {
                                self.current = start;
                                let identifier = self.identifier();
                                return self.create_identifier_token(identifier);
                            }
                        }
                    }
                    // Handle float types
                    else if type_prefix == 'f' {
                        match type_str.as_str() {
                            "f32" => return self.create_float_token(FloatType::F32, type_str),
                            "f64" => return self.create_float_token(FloatType::F64, type_str),
                            "f128" => return self.create_float_token(FloatType::F128, type_str),
                            "f256" => return self.create_float_token(FloatType::F256, type_str),
                            "f512" => return self.create_float_token(FloatType::F512, type_str),
                            "f1024" => return self.create_float_token(FloatType::F1024, type_str),
                            "f2048" => return self.create_float_token(FloatType::F2048, type_str),
                            "f4096" => return self.create_float_token(FloatType::F4096, type_str),
                            "f8192" => return self.create_float_token(FloatType::F8192, type_str),
                            "f16384" => return self.create_float_token(FloatType::F16384, type_str),
                            "f32768" => return self.create_float_token(FloatType::F32768, type_str),

                            _ => {
                                self.current = start;
                                let identifier = self.identifier();
                                return self.create_identifier_token(identifier);
                            }
                        }
                    }
                }

                // If we get here, treat as identifier
                self.current = start;
                let identifier = self.identifier();
                return self.create_identifier_token(identifier);
            },
            's' => {
                let start = self.current - 1;
                let remaining = &self.source[start..];

                if remaining.starts_with("str") {
                    for _ in 0..("str".len() - 1) {
                        self.advance();
                    }

                    return Token {
                        token_type: TokenType::TYPE_STRING,
                        lexeme: "str".to_string(),
                        line: self.line,
                    };
                } else {
                    self.current = start;
                    let identifier = self.identifier();
                    return Token {
                        token_type: TokenType::IDENTIFIER(identifier.clone()),
                        lexeme: identifier,
                        line: self.line,
                    };
                }
            },
            '+' => {
                if self.match_next('+') {
                    Token {
                        token_type: TokenType::INCREMENT,
                        lexeme: "++".to_string(),
                        line: self.line,
                    }
                } else {
                    Token {
                        token_type: TokenType::PLUS,
                        lexeme: "+".to_string(),
                        line: self.line,
                    }
                }
            },
            '-' => {
                if self.match_next('-') {
                    Token {
                        token_type: TokenType::DECREMENT,
                        lexeme: "--".to_string(),
                        line: self.line,
                    }
                } else {
                    Token {
                        token_type: TokenType::MINUS,
                        lexeme: "-".to_string(),
                        line: self.line,
                    }
                }
            },
            '*' => Token {
                token_type: TokenType::STAR,
                lexeme: "*".to_string(),
                line: self.line,
            },
            '/' => Token {
                token_type: TokenType::DIV,
                lexeme: "/".to_string(),
                line: self.line,
            },
            ';' => Token {
                token_type: TokenType::SEMICOLON,
                lexeme: ";".to_string(),
                line: self.line,
            },
            ':' => Token {
                token_type: TokenType::COLON,
                lexeme: ":".to_string(),
                line: self.line,
            },
            '<' => {
                if self.match_next('=') {
                    Token {
                        token_type: TokenType::LCHEVR_EQ,
                        lexeme: "<=".to_string(),
                        line: self.line,
                    }
                } else {
                    Token {
                        token_type: TokenType::LCHEVR,
                        lexeme: "<".to_string(),
                        line: self.line,
                    }
                }

            },
            '>' => {
                if self.match_next('=') {
                    Token {
                        token_type: TokenType::RCHEVR_EQ,
                        lexeme: ">=".to_string(),
                        line: self.line,
                    }
                } else {
                    Token {
                        token_type: TokenType::RCHEVR,
                        lexeme: ">".to_string(),
                        line: self.line,
                    }
                }

            },
            '(' => Token {
                token_type: TokenType::LPAREN,
                lexeme: "(".to_string(),
                line: self.line,
            },
            ')' => Token {
                token_type: TokenType::RPAREN,
                lexeme: ")".to_string(),
                line: self.line,
            },
            '{' => Token {
                token_type: TokenType::LBRACE,
                lexeme: "{".to_string(),
                line: self.line,
            },
            '}' => Token {
                token_type: TokenType::RBRACE,
                lexeme: "}".to_string(),
                line: self.line,
            },
            '[' => Token {
                token_type: TokenType::LBRACK,
                lexeme: "[".to_string(),
                line: self.line,
            },
            ']' => Token {
                token_type: TokenType::RBRACK,
                lexeme: "]".to_string(),
                line: self.line,
            },
            '=' => {
                if self.match_next('=') {
                    Token {
                        token_type: TokenType::EQUAL_TWO,
                        lexeme: "==".to_string(),
                        line: self.line,
                    }
                } else {
                    Token {
                        token_type: TokenType::EQUAL,
                        lexeme: "=".to_string(),
                        line: self.line,
                    }
                }
            },
            '&' => {
                if self.match_next('&') {
                    Token {
                        token_type: TokenType::LOGICAL_AND,
                        lexeme: "&&".to_string(),
                        line: self.line,
                    }
                } else {
                    Token {
                        token_type: TokenType::BITWISE_AND,
                        lexeme: "&".to_string(),
                        line: self.line,
                    }
                }
            },
            '|' => {
                if self.match_next('|') {
                    Token {
                        token_type: TokenType::LOGICAL_OR,
                        lexeme: "||".to_string(),
                        line: self.line,
                    }
                } else {
                    Token {
                        token_type: TokenType::BITWISE_OR,
                        lexeme: "|".to_string(),
                        line: self.line,
                    }
                }
            },
            '!' => {
                if self.match_next('=') {
                    Token {
                        token_type: TokenType::NOT_EQUAL,
                        lexeme: "!=".to_string(),
                        line: self.line,
                    }
                } else if self.match_next('&') {
                    Token {
                        token_type: TokenType::NAND,
                        lexeme: "!&".to_string(),
                        line: self.line,
                    }
                } else if self.match_next('|') {
                    Token {
                        token_type: TokenType::NOR,
                        lexeme: "!|".to_string(),
                        line: self.line,
                    }
                } else {
                    Token {
                        token_type: TokenType::NOT,
                        lexeme: "!".to_string(),
                        line: self.line,
                    }
                }
            },
            '^' => {
                Token {
                    token_type: TokenType::XOR,
                    lexeme: "^".to_string(),
                    line: self.line,
                }
            },
            '~' => {
                if self.match_next('^') {
                    Token {
                        token_type: TokenType::XNOR,
                        lexeme: "~^".to_string(),
                        line: self.line,
                    }
                } else {
                    Token {
                        token_type: TokenType::BITWISE_NOT,
                        lexeme: "~".to_string(),
                        line: self.line,
                    }
                }
            },
            '?' => {
                if self.match_next('?') {
                    Token {
                        token_type: TokenType::NULL_COALESCE,
                        lexeme: "??".to_string(),
                        line: self.line,
                    }
                } else {
                    Token {
                        token_type: TokenType::CONDITION,
                        lexeme: "?".to_string(),
                        line: self.line,
                    }
                }

            },
            ':' => Token {
                token_type: TokenType::COLON,
                lexeme: ":".to_string(),
                line: self.line,
            },
            ',' => Token {
                token_type: TokenType::COMMA,
                lexeme: ",".to_string(),
                line: self.line,
            },
            '"' => {
                return Token {
                    token_type: TokenType::STRING(self.string()),
                    lexeme: String::new(), // 필요에 따라 설정
                    line: self.line,
                };
            },
            'a'..='z' | 'A'..='Z' => {
                let identifier = self.identifier();
                match identifier.as_str() {
                    "fun" => Token {
                        token_type: TokenType::FUN,
                        lexeme: "fun".to_string(),
                        line: self.line,
                    },
                    "var" => Token {
                        token_type: TokenType::VAR,
                        lexeme: "var".to_string(),
                        line: self.line,
                    },
                    "const" => Token {
                        token_type: TokenType::CONST,
                        lexeme: "const".to_string(),
                        line: self.line,
                    },
                    "if" => Token {
                        token_type: TokenType::IF,
                        lexeme: "if".to_string(),
                        line: self.line,
                    },
                    "else" => Token {
                        token_type: TokenType::ELSE,
                        lexeme: "else".to_string(),
                        line: self.line,
                    },
                    "while" => Token {
                        token_type: TokenType::WHILE,
                        lexeme: "while".to_string(),
                        line: self.line,
                    },
                    "for" => Token {
                        token_type: TokenType::FOR,
                        lexeme: "for".to_string(),
                        line: self.line,
                    },
                    "in" => Token {
                        token_type: TokenType::IN,
                        lexeme: "in".to_string(),
                        line: self.line,
                    },
                    "is" => Token {
                        token_type: TokenType::IS,
                        lexeme: "is".to_string(),
                        line: self.line,
                    },
                    "rol" => Token {
                        token_type: TokenType::ROL,
                        lexeme: "rol".to_string(),
                        line: self.line,
                    },
                    "ror" => Token {
                        token_type: TokenType::ROR,
                        lexeme: "ror".to_string(),
                        line: self.line,
                    },
                    "xnand" => Token {
                        token_type: TokenType::XNAND,
                        lexeme: "xnand".to_string(),
                        line: self.line,
                    },
                    "import" => Token {
                        token_type: TokenType::IMPORT,
                        lexeme: "import".to_string(),
                        line: self.line,
                    },
                    "return" => Token {
                        token_type: TokenType::RETURN,
                        lexeme: "return".to_string(),
                        line: self.line,
                    },
                    "continue" => Token {
                        token_type: TokenType::CONTINUE,
                        lexeme: "continue".to_string(),
                        line: self.line,
                    },
                    "print" => Token {
                        token_type: TokenType::PRINT,
                        lexeme: "print".to_string(),
                        line: self.line,
                    },
                    "input" => Token {
                        token_type: TokenType::INPUT,
                        lexeme: "input".to_string(),
                        line: self.line,
                    },
                    "println" => Token {
                        token_type: TokenType::PRINTLN,
                        lexeme: "println".to_string(),
                        line: self.line,
                    },
                    _ => {
                        Token {
                            token_type: TokenType::IDENTIFIER(identifier.clone()),
                            lexeme: identifier,
                            line: self.line,
                        }
                    }
                }
            },
            '0'..='9' => {
                return Token {
                    token_type: TokenType::NUMBER(self.number()),
                    lexeme: String::new(),
                    line: self.line,
                };
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
            token_type: TokenType::TYPE_INT(int_type),
            lexeme,
            line: self.line,
        }
    }

    fn create_float_token(&self, float_type: FloatType, lexeme: String) -> Token {
        Token {
            token_type: TokenType::TYPE_FLOAT(float_type),
            lexeme,
            line: self.line,
        }
    }

    fn create_identifier_token(&self, identifier: String) -> Token {
        Token {
            token_type: TokenType::IDENTIFIER(identifier.clone()),
            lexeme: identifier,
            line: self.line,
        }
    }

    // 문자열 리터럴 처리 함수 추가
    fn string(&mut self) -> String {
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
        match i64::from_str(number_str) {
            Ok(num) => num,
            Err(_) => 0,
        }
    }
}
