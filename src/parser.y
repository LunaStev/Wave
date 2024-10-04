%{
    #include <stdio.h>
    #include <stdlib.h>
    #include "parser.h"
%}

%union {
    int ival;    // 정수형 값
    char* sval;  // 문자열
}

%token <sval> IDENTIFIER
%token <ival> NUMBER
%token VAR FUN IF ELSE WHILE PRINTLN INPUT
%type <ival> expr

%%

// 파서 규칙 정의
program:
    statement_list
;

statement_list:
    statement_list statement
    | /* empty */
;

statement:
    VAR IDENTIFIER '=' expr ';' {
        printf("Variable declaration: %s = %d\n", $2, $4);
    }
    | PRINTLN expr ';' {
        printf("Print statement: %d\n", $2);
    }
    | IF expr '{' statement_list '}' ELSE '{' statement_list '}' {
        if ($2) {
            printf("Condition true\n");
        } else {
            printf("Condition false\n");
        }
    }
;

expr:
    NUMBER {
        $$ = $1;
    }
    | IDENTIFIER {
        $$ = 0; // Assume variable has value 0 for simplicity
    }
    | expr '>' expr {
        $$ = $1 > $3;
    }
    | expr '<' expr {
        $$ = $1 < $3;
    }
;

%%

int yyerror(char *s) {
    fprintf(stderr, "Error: %s\n", s);
    return 0;
}
