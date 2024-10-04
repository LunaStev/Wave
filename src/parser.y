%{
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

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
%token FUN VAR IF ELSE WHILE PRINTLN INPUT

// 반환할 타입 지정
%type <ival> expr statement

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
    FUN IDENTIFIER '(' ')' '{' statement_list '}' {
        // 함수 정의
        char code[100];
        sprintf(code, "void %s() {\n%s\n}", $2, $6);
        generate_code(code);
    }
    | VAR IDENTIFIER '=' expr ';' {
        // 변수 선언 및 초기화
        char code[100];
        sprintf(code, "int %s = %d;", $2, $4);
        generate_code(code);
    }
    | PRINTLN expr ';' {
        // 출력 문을 위한 코드 생성
        char code[100];
        sprintf(code, "printf(\"%%d\\n\", %d);", $2);
        generate_code(code);
    }
    | IF expr '{' statement_list '}' ELSE '{' statement_list '}' {
        // 조건문
        char code[200];
        sprintf(code, "if (%d) {\n%s} else {\n%s}", $2, $6, $10);
        generate_code(code);
    }
    | WHILE expr '{' statement_list '}' {
        // 반복문
        char code[200];
        sprintf(code, "while (%d) {\n%s}", $2, $5);
        generate_code(code);
    }
;

expr:
    NUMBER {
        $$ = $1;
    }
    | IDENTIFIER {
        $$ = 0; // 변수는 0으로 초기화된다고 가정
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
void yyerror(const char* s) {
    fprintf(stderr, "Error: %s\n", s);
}

// 코드 생성 함수
void generate_code(const char* code) {
    printf("%s\n", code); // 생성된 코드를 출력
}

int main(void) {
    printf("Wave 언어 프로그램을 입력하세요:\n");
    yyparse();
    return 0;
}
