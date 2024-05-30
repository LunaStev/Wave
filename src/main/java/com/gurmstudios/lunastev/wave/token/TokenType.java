package com.gurmstudios.lunastev.wave.token;

public class TokenType {
    public enum Token {
        FUN,    // 함수
        VAR,    // 문자 변수
        COUNT,  // 숫자 변수
        IF,
        FOR,
        WHILE,
        ELSE,

        PLUS,   // '+'
        MINUS,  // '-'
        TIMES,  // '*'
        SLASH,  // '/'

        IMPORT,
    }
}
