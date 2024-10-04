%{
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "parser.h"

// 함수 선언
void generate_code(const char* code);
extern int yylex(void);
void yyerror(const char *s);
%}

%union {
    int ival;    // 정수형 값
    char* sval;  // 문자열
}

// 토큰과 문법 요소의 타입 선언
%token <sval> IDENTIFIER
%token <ival> NUMBER
%token VAR FUN IF ELSE WHILE PRINTLN INPUT

// 반환할 타입 지정
%type <ival> expr
%type <sval> statement

%%

// 프로그램의 구조 정의
program:
    statement_list
;

statement_list:
    statement_list statement
    | /* empty */
;

statement:
    VAR IDENTIFIER '=' expr ';' {
        // 변수 선언을 위한 코드 생성
        char code[100];
        sprintf(code, "int %s = %d;", $2, $4);
        generate_code(code);
    }
    | PRINTLN expr ';' {
        // 출력 문을 위한 코드 생성
        char code[100];
        sprintf(code, "printf(\"%%d\", %d);", $2);
        generate_code(code);
    }
    | IF expr '{' statement_list '}' ELSE '{' statement_list '}' {
        // 조건문을 위한 코드 생성
    }
;

expr:
    NUMBER {
        $$ = $1;
    }
    | IDENTIFIER {
        $$ = 0; // 변수가 0이라고 가정
    }
    | expr '>' expr {
        $$ = $1 > $3;
    }
    | expr '<' expr {
        $$ = $1 < $3;
    }
;

%%

// 오류 처리
int yyerror(const char* s) {
    fprintf(stderr, "Error: %s\n", s);
    return 0;
}

// 코드 생성 함수
void generate_code(const char* code) {
    printf("%s\n", code); // 생성된 코드를 출력
}

int main(void) {
    yyparse();
    return 0;
}
