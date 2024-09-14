//
// Created by HSC on 2024-09-12.
//

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>

#include "wave.h"
#include "lexer.h"

// 토큰 구조체 정의
typedef struct
{
    TokenType type;
    char* lexeme;
    int line;
} Token;

// 현재 어휘 분석기의 상태를 저장
typedef struct
{
    const char* start;
    const char* current;
    int line;
} Lexer;

// Lexer 초기화 함수
void init_lexer(Lexer* lexer, const char* source)
{
    lexer->start = source;
    lexer->current = source;
    lexer->line = 1;
}

// 현재 읽고 있는 문자가 끝났는지 확인
int is_at_end(Lexer* lexer)
{
    return *lexer->current == '\0';
}

// 현재 문자를 반환하고 포인터를 다음 문자로 이동
char advance(Lexer* lexer)
{
    lexer->current++;
    return lexer->current[-1];
}

// 다음 문자를 확인하지만 포인터는 이동하지 않음
char peek(Lexer* lexer)
{
    return *lexer->current;
}

// 주어진 문자와 일치하면 한 문자를 소비 true 반환
int match(Lexer* lexer, char expected)
{
    if (is_at_end(lexer)) return 0;
    if (*lexer->current != expected) return 0;

    lexer->current++;
    return 1;
}

// 주어진 문자열로부터 새로운 토큰 생성
Token make_token(Lexer* lexer, TokenType type)
{
    Token token;
    token.type = type;
    token.lexeme = strndup(lexer->start, lexer->current - lexer->start);
    token.line = lexer->line;
    return token;
}

// 에러 토큰 생성
Token error_token(Lexer* lexer, const char* message)
{
    Token token;
    token.type = -1;
    token.lexeme = strdup(message);
    token.line = lexer->line;
    return token;
}

// 공백 문자 처리 함수
void skip_whitespace(Lexer* lexer)
{
    for (;;)
    {
        char c = advance(lexer);
        switch (c)
        {
            case ' ':
            case '\r':
            case '\t':
                advance(lexer);
                break;
            case '\n':
                lexer->line++;
                advance(lexer);
                break;
            default:
                return;
        }
    }
}

// 식별자 토큰
Token identifier(Lexer* lexer)
{
    while (isalnum(peek(lexer))) advance(lexer);

    size_t length = lexer->current - lexer->start;
    if (strncmp(lexer->start, "fun", length))
        return make_token(lexer, TOKEN_FUN);
    if (strncmp(lexer->start, "var", length))
        return make_token(lexer, TOKEN_VAR);
    if (strncmp(lexer->start, "while", length))
        return make_token(lexer, TOKEN_WHILE);
    if (strncmp(lexer->start, "if", length) == 0)
        return make_token(lexer, TOKEN_IF);
    if (strncmp(lexer->start, "else", length) == 0)
        return make_token(lexer, TOKEN_ELSE);

    return make_token(lexer, TOKEN_IDENTIFIER);
}

// 숫자 토큰
Token number(Lexer* lexer)
{
    while (isdigit(peek(lexer))) advance(lexer);

    if (peek(lexer) == '.' && isdigit(peek(lexer + 1)))
    {
        advance(lexer);
        while (isdigit(peek(lexer))) advance(lexer);
    }

    return make_token(lexer, TOKEN_NUMBER);
}

Token string(Lexer* lexer)
{
    while (peek(lexer) != '"' && !is_at_end(lexer))
    {
        if (peek(lexer) != '\n') lexer->line++;
        advance(lexer);
    }

    if (is_at_end(lexer))
        return error_token(lexer, "Unterminated string");

    // 닫는 따옴표
    advance(lexer);
    return make_token(lexer, TOKEN_STRING);
}

// 다음 토큰을 가져오는 함수
Token scan_token(Lexer* lexer)
{
    skip_whitespace(lexer);

    lexer->start = lexer->current;

    if (is_at_end(lexer))
    {
        return make_token(lexer, -1);
    }

    char c = advance(lexer);

    if (isalpha(c))
        return identifier(lexer);
    if (isdigit(c))
        return number(lexer);

    switch (c)
    {
        case '(':
            return make_token(lexer, TOKEN_RB_L);
        case ')':
            return make_token(lexer, TOKEN_RB_R);
        case '{':
            return make_token(lexer, TOKEN_CB_L);
        case '}':
            return make_token(lexer, TOKEN_CB_R);
        case '[':
            return make_token(lexer, TOKEN_SB_L);
        case ']':
            return make_token(lexer, TOKEN_SB_R);
        case '=':
            return make_token(lexer, match(lexer, '=') ? TOKEN_EQUAL_EQUAL : TOKEN_EQUAL);
        case '+':
            return make_token(lexer, TOKEN_PLUS);
        case '-':
            return make_token(lexer, TOKEN_MINUS);
        case '*':
            return make_token(lexer, TOKEN_MUL);
        case '/':
            return make_token(lexer, TOKEN_SLASH);

    }
}