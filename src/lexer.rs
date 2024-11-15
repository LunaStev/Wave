use std::str::FromStr;

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
    NULL_COALESCE,          // ??
    CONDITIONAL,            // ?:
    IN,                     // in
    IS,                     // is
    ROL,
    ROR,
    XNAND,
    IDENTIFIER(String),
    STRING(String),
    NUMBER(i64),
    PLUS,                   // +
    MINUS,                  // -
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
    source: &'a str,
    current: usize,
    line: usize,
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
        self.source.chars().nth(self.current - 1).unwrap()
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
            self.source.chars().nth(self.current).unwrap()
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
            '+' => Token {
                token_type: TokenType::PLUS,
                lexeme: "+".to_string(),
                line: self.line,
            },
            '-' => Token {
                token_type: TokenType::MINUS,
                lexeme: "-".to_string(),
                line: self.line,
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
                } else if self.match_next(':') {
                    Token {
                        token_type: TokenType::CONDITIONAL,
                        lexeme: "?:".to_string(),
                        line: self.line,
                    }
                } else {
                    panic!("Unexpected character: {}", c);
                }

            },
            '"' => {
                return Token {
                    token_type: TokenType::STRING(self.string()),
                    lexeme: String::new(), // 필요에 따라 설정
                    line: self.line,
                };
            }
            'a'..='z' | 'A'..='Z' => {
                let identifier = self.identifier();
                let token_type = match identifier.as_str() {
                    "fun" => TokenType::FUN,
                    "var" => TokenType::VAR,
                    "const" => TokenType::CONST,
                    "if" => TokenType::IF,
                    "else" => TokenType::ELSE,
                    "while" => TokenType::WHILE,
                    "for" => TokenType::FOR,
                    "in" => TokenType::IN,
                    "is" => TokenType::IS,
                    "rol" => TokenType::ROL,
                    "ror" => TokenType::ROR,
                    "xnand" => TokenType::XNAND,
                    "import" => TokenType::IMPORT,
                    "return" => TokenType::RETURN,
                    "continue" => TokenType::CONTINUE,
                    "print" => TokenType::PRINT,
                    "println" => TokenType::PRINTLN,
                    _ => TokenType::IDENTIFIER(identifier.clone()),
                };
                Token {
                    token_type,
                    lexeme: identifier,
                    line: self.line,
                }
            }
            '0'..='9' => {
                return Token {
                    token_type: TokenType::NUMBER(self.number()),
                    lexeme: String::new(),
                    line: self.line,
                };
            }
            'p' => {
                if self.match_next('r') {
                    if self.match_next('i') {
                        if self.match_next('n') {
                            if self.match_next('t') {
                                if self.match_next('l') {
                                    Token {
                                        token_type: TokenType::PRINTLN,
                                        lexeme: "println".to_string(),
                                        line:self.line,
                                    }
                                } else {
                                    Token {
                                        token_type: TokenType::PRINT,
                                        lexeme: "print".to_string(),
                                        line: self.line,
                                    }
                                }
                            } else {
                                Token {
                                    token_type: TokenType::PRINT,
                                    lexeme: "print".to_string(),
                                    line: self.line,
                                }
                            }
                        } else {
                            panic!("Unexpected character: {}", c);
                        }
                    } else {
                        panic!("Unexpected character: {}", c);
                    }
                } else {
                    panic!("Unexpected character: {}", c);
                }
            },
            _ => {
                panic!("Unexpected character: {}", c);
            }
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
        let start = self.current - 1;
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
        i64::from_str(number_str).unwrap()
    }
}
