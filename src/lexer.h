//
// Created by HSC on 2024-09-12.
//

#ifndef LEXER_H
#define LEXER_H

typedef enum TokenType {
    TOKEN_IDENTIFIER,   // 식별자 (변수, 함수 등 이름)
    TOKEN_NUMBER,       // 숫자 (정수 또는 실수)
    TOKEN_STRING,       // 문자열 (텍스트)

    TOKEN_FUN,          // fun
    TOKEN_VAR,          // var
    TOKEN_WHILE,        // while
    TOKEN_IF,           // if
    TOKEN_ELSE,         // else

    TOKEN_EQUAL,        // 등호 (=), 할당 연산자
    TOKEN_EQUAL_EQUAL,  // 이중 등호 (==), 비교 연산자

    TOKEN_PLUS,         // 더하기 기호 (+), 덧셈 연산
    TOKEN_MINUS,        // 빼기 기호 (-), 뺄셈 연산
    TOKEN_MUL,         // 별표 (*), 곱셈 연산자
    TOKEN_SLASH,        // 슬래시 (/), 나눗셈 연산자

    TOKEN_PERCENT,      // 퍼센트 기호 (%), 나머지 연산자
    TOKEN_LESS,         // 작음(<), 비교 연산자
    TOKEN_GREATER,      // 큼(>), 비교 연산자


    TOKEN_COLON,        // 콜론 (:), 레이블이나 타입 명시 등
    TOKEN_SEMICOLON,    // 세미콜론 (;)

    TOKEN_COMMA,        // 쉼표 (,)
    TOKEN_DOT,          // 점 (.)

} TokenType;


#endif //LEXER_H
