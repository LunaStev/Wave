//
// Created by HSC on 2024-09-12.
//

#include "lexer.h"

int lexer(char *input) {
    if (input[0] == '\0') {
        return 0;
    } else {
        return 1;
    }

    if (TOKEN_IF) {
        return TOKEN_IF;
    } else if (TOKEN_ELSE) {
        return TOKEN_ELSE;
    } else if (TOKEN_WHILE) {
        return TOKEN_WHILE;
    }
}