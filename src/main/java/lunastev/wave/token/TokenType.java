package lunastev.wave.token;

public enum TokenType {
    FUN,            // 함수
    VAR,            // 문자 변수
    COUNT,          // 숫자 변수
    IF,
    FOR,
    WHILE,
    ELSE,

    TRUE,
    FALSE,
    NULL,

    COMMA,          // '.'
    SEMI,           // ';'
    COLON,          // ':'

    MARKS,          // '''
    DOUBLE_MARKS,   // '"'

    L_BRA,       // '('
    R_BRA,       // ')'
    L_C_BRA,     // '{'
    R_C_BRA,     // '}'
    L_S_BRA,     // '['
    R_S_BRA,     // ']'
    L_A_BRA,     // '<'
    R_A_BRA,     // '>'

    PLUS,           // '+'
    MINUS,          // '-'
    TIMES,          // '*'
    SLASH,          // '/'

    IMPORT,
    RETURN,
}

