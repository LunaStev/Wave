%{
#include <stdio.h>
#include <stdlib.h>
%}

%union {
    int num;
}

%token <num> NUMBER

%%
[0-9]+       { yylval = atoi(yytext); return NUMBER; }
"+"          { return '+'; }
"-"          { return '-'; }
"*"          { return '*'; }
"/"          { return '/'; }
\n           { /* Ignore newlines */ }
[ \t]        { /* Ignore whitespace */ }
.            { return yytext[0]; } // 기타 문자 처리
%%

// 오류 핸들러
int yywrap() {
    return 1;
}
