package lunastev.wave.token;

public enum TokenType {
    /* BlockComment */
    BC,
    /* LineComment */
    LC,
    /* WhiteSpace */
    WS,
    /* TAB */
    TAB,
    /* NewLine */
    NL,

    FUN,                        // 함수
    VAR,                        // 문자 변수
    COUNT,                      // 숫자 변수
    IF,
    FOR,
    WHILE,
    ELSE,

    ID_ENT,                     // 식별자
    STRING,                     // 문자열 리터럴
    NUMBER,                     // 숫자 리터럴

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

    public boolean isAuxiliary() {
        return this == BC || this == LC || this == WS || this == TAB || this == NL;
    }
}

