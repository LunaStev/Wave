#[derive(Debug, PartiaEq, Eq, Clone)]
pub enum Token {
    /** BlockComment */
    BC,
    /** LineComment */
    LC,
    /** WhiteSpace */
    WS,
    /** TAB */
    TAB,
    /** NewLine */
    NL,

    FUN,                        // 함수
    VAR,                        // 문자 변수
    COUNT,                      // 숫자 변수
    IF,
    FOR,
    WHILE,
    ELSE,
    SWITCH,
    CASE,
    BREAK,

    CONTINUE,
    DEFAULT,

    ID_ENT,                     // 식별자
    STRING,                     // 문자열 리터럴
    NUMBER(i128),                     // 숫자 리터럴

    AND,                        // '&&'
    OR,                         // '||'
    NOT,                        // '!'

    EQUAL,                      // '='
    GREATER_THAN,               // '>'
    LEES_THAN,                  // '<'
    GREATER_THAN_EQUAL,         // '>='
    LESS_THAN_EQUAL,            // '<='
    EQUAL_EQUAL,                // '=='
    NOT_EQUAL,                  // '!='
    INCREMENT,                  // '++'
    DECREMENT,                  // '--'
    ARROW,                      // '->'

    TRUE,
    FALSE,
    NULL,

    COMMA,                      // '.'
    SEMI,                       // ';'
    COLON,                      // ':'

    MARKS,                      // '''
    DOUBLE_MARKS,               // '"'

    L_BRA,                      // '('
    R_BRA,                      // ')'
    L_C_BRA,                    // '{'
    R_C_BRA,                    // '}'
    L_S_BRA,                    // '['
    R_S_BRA,                    // ']'
    L_A_BRA,                    // '<'
    R_A_BRA,                    // '>'

    PLUS,                       // '+'
    MINUS,                      // '-'
    TIMES,                      // '*'
    SLASH,                      // '/'

    IMPORT,
    RETURN,
}

pub struct Lexer<'a> {
    input: &'a str,
    pos: usize,
    current_char: Option<char>
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self { }
    fn advance(&mut self) { }
    fn skip_whitespace(&mut self) { }
    pub fn next_token(&mut self) -> Token { }

}