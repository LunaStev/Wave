#include <stdio.h>
#include <stdlib.h>
#include "lexer.h"

void print_token(Token token) {
    const char *token_type;

    switch (token.type) {
    case TOKEN_IDENTIFIER: token_type = "IDENTIFIER"; break;
    case TOKEN_NUMBER: token_type = "NUMBER"; break;
    case TOKEN_STRING: token_type = "STRING"; break;
    case TOKEN_FUN: token_type = "FUN"; break;
    case TOKEN_VAR: token_type = "VAR"; break;
    case TOKEN_WHILE: token_type = "WHILE"; break;
    case TOKEN_IF: token_type = "IF"; break;
    case TOKEN_ELSE: token_type = "ELSE"; break;
    case TOKEN_EQUAL: token_type = "EQUAL"; break;
    case TOKEN_EQUAL_EQUAL: token_type = "EQUAL_EQUAL"; break;
    case TOKEN_PLUS: token_type = "PLUS"; break;
    case TOKEN_MINUS: token_type = "MINUS"; break;
    case TOKEN_MUL: token_type = "MUL"; break;
    case TOKEN_SLASH: token_type = "SLASH"; break;
    case TOKEN_PERCENT: token_type = "PERCENT"; break;
    case TOKEN_LESS: token_type = "LESS"; break;
    case TOKEN_GREATER: token_type = "GREATER"; break;
    default: token_type = "UNKNOWN"; break;
    }

    printf("Token Type: %s, Lexeme: %s, Line: %d\n", token_type, token.lexeme, token.line);
}

int main() {
    const char* source = "fun myFunction(var x) { if (x < 10) { return x; } }";

    Lexer lexer;
    init_lexer(&lexer, source);

    Token token;
    do {
        token = scan_token(&lexer);
        print_token(token);
        free(token.lexeme); // 메모리 해제
    } while (token.type != -1);

    return 0;
}
