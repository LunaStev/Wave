#ifndef WAVE_H
#define WAVE_H

// Define token types for Lex & Yacc
typedef union {
    int ival;
    char *sval;
} YYSTYPE;

#define YYSTYPE_IS_DECLARED 1

void yyerror(const char *s);
int yylex();

#endif