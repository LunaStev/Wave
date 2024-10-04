%{
#include "parser.h"
%}

%option noyywrap

%union {
    int ival;    // 정수형 값
    char* sval;  // 문자열
}

%token <sval> IDENTIFIER
%token <ival> NUMBER
%token FUN VAR IF ELSE WHILE PRINTLN INPUT

%%

// 패턴 정의
[0-9]+       { yylval.ival = atoi(yytext); return NUMBER; }
[a-zA-Z_][a-zA-Z0-9_]* { yylval.sval = strdup(yytext); return IDENTIFIER; }
"fun"       { return FUN; }
"var"       { return VAR; }
"if"        { return IF; }
"else"      { return ELSE; }
"while"     { return WHILE; }
"println"   { return PRINTLN; }
"input"     { return INPUT; }
"{"         { return '{'; }
"}"         { return '}'; }
"+"         { return '+'; }
"-"         { return '-'; }
"*"         { return '*'; }
"/"         { return '/'; }
">"         { return '>'; }
"<"         { return '<'; }
\n          { /* Ignore newlines */ }
[ \t]       { /* Ignore whitespace */ }
.           { return yytext[0]; } // 기타 문자 처리

%%

// 오류 핸들러
int yywrap() {
    return 1;
}
