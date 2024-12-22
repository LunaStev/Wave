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
    DOT,                    // .
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
            '.' => Token {
                token_type: TokenType::DOT,
                lexeme: ".".to_string(),
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
                    "isz" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::ISZ),
                        lexeme: "isz".to_string(),
                        line: self.line,
                    },
                    "i4" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::I4),
                        lexeme: "i4".to_string(),
                        line: self.line,
                    },
                    "i8" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::I8),
                        lexeme: "i8".to_string(),
                        line: self.line,
                    },
                    "i16" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::I16),
                        lexeme: "i16".to_string(),
                        line: self.line,
                    },
                    "i32" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::I32),
                        lexeme: "i32".to_string(),
                        line: self.line,
                    },
                    "i64" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::I64),
                        lexeme: "i64".to_string(),
                        line: self.line,
                    },
                    "i128" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::I128),
                        lexeme: "i128".to_string(),
                        line: self.line,
                    },
                    "i256" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::I256),
                        lexeme: "i256".to_string(),
                        line: self.line,
                    },
                    "i512" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::I512),
                        lexeme: "i512".to_string(),
                        line: self.line,
                    },
                    "i1024" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::I1024),
                        lexeme: "i1024".to_string(),
                        line: self.line,
                    },
                    "i2048" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::I2048),
                        lexeme: "i2048".to_string(),
                        line: self.line,
                    },
                    "i4096" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::I4096),
                        lexeme: "i4096".to_string(),
                        line: self.line,
                    },
                    "i8192" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::I8192),
                        lexeme: "i8192".to_string(),
                        line: self.line,
                    },
                    "i16384" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::I16384),
                        lexeme: "i16384".to_string(),
                        line: self.line,
                    },
                    "i32768" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::I32768),
                        lexeme: "i32768".to_string(),
                        line: self.line,
                    },
                    "usz" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::USZ),
                        lexeme: "usz".to_string(),
                        line: self.line,
                    },
                    "u4" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::U4),
                        lexeme: "u4".to_string(),
                        line: self.line,
                    },
                    "u8" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::U8),
                        lexeme: "u8".to_string(),
                        line: self.line,
                    },
                    "u16" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::U16),
                        lexeme: "u16".to_string(),
                        line: self.line,
                    },
                    "u32" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::U32),
                        lexeme: "u32".to_string(),
                        line: self.line,
                    },
                    "u64" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::U64),
                        lexeme: "u64".to_string(),
                        line: self.line,
                    },
                    "u128" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::U128),
                        lexeme: "u128".to_string(),
                        line: self.line,
                    },
                    "u256" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::U256),
                        lexeme: "u256".to_string(),
                        line: self.line,
                    },
                    "u512" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::U512),
                        lexeme: "u512".to_string(),
                        line: self.line,
                    },
                    "u1024" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::U1024),
                        lexeme: "u1024".to_string(),
                        line: self.line,
                    },
                    "u2048" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::U2048),
                        lexeme: "u2048".to_string(),
                        line: self.line,
                    },
                    "u4096" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::U4096),
                        lexeme: "u4096".to_string(),
                        line: self.line,
                    },
                    "u8192" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::U8192),
                        lexeme: "u8192".to_string(),
                        line: self.line,
                    },
                    "u16384" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::U16384),
                        lexeme: "u16384".to_string(),
                        line: self.line,
                    },
                    "u32768" => Token {
                        token_type: TokenType::TYPE_INT(IntegerType::U32768),
                        lexeme: "u32768".to_string(),
                        line: self.line,
                    },
                    "f32" => Token {
                        token_type: TokenType::TYPE_FLOAT(FloatType::F32),
                        lexeme: "f32".to_string(),
                        line: self.line,
                    },
                    "f64" => Token {
                        token_type: TokenType::TYPE_FLOAT(FloatType::F64),
                        lexeme: "f64".to_string(),
                        line: self.line,
                    },
                    "f128" => Token {
                        token_type: TokenType::TYPE_FLOAT(FloatType::F128),
                        lexeme: "f128".to_string(),
                        line: self.line,
                    },
                    "f256" => Token {
                        token_type: TokenType::TYPE_FLOAT(FloatType::F256),
                        lexeme: "f256".to_string(),
                        line: self.line,
                    },
                    "f512" => Token {
                        token_type: TokenType::TYPE_FLOAT(FloatType::F512),
                        lexeme: "f512".to_string(),
                        line: self.line,
                    },
                    "f1024" => Token {
                        token_type: TokenType::TYPE_FLOAT(FloatType::F1024),
                        lexeme: "f1024".to_string(),
                        line: self.line,
                    },
                    "f2048" => Token {
                        token_type: TokenType::TYPE_FLOAT(FloatType::F2048),
                        lexeme: "f2048".to_string(),
                        line: self.line,
                    },
                    "f4096" => Token {
                        token_type: TokenType::TYPE_FLOAT(FloatType::F4096),
                        lexeme: "f4096".to_string(),
                        line: self.line,
                    },
                    "f8192" => Token {
                        token_type: TokenType::TYPE_FLOAT(FloatType::F8192),
                        lexeme: "f8192".to_string(),
                        line: self.line,
                    },
                    "f16384" => Token {
                        token_type: TokenType::TYPE_FLOAT(FloatType::F16384),
                        lexeme: "f16384".to_string(),
                        line: self.line,
                    },
                    "f32768" => Token {
                        token_type: TokenType::TYPE_FLOAT(FloatType::F32768),
                        lexeme: "f32768".to_string(),
                        line: self.line,
                    },
                    "str" => Token {
                        token_type: TokenType::TYPE_STRING,
                        lexeme: "str".to_string(),
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
