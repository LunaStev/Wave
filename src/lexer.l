%{
    #include "parser.h"
%}

%%

"var"                   { return VAR; }
"fun"                   { return FUN; }
"if"                    { return IF; }
"else"                  { return ELSE; }
"while"                 { return WHILE; }
"println"               { return PRINTLN; }
"input"                 { return INPUT; }

[0-9]+                  { yylval.ival = atoi(yytext); return NUMBER; }
[a-zA-Z_][a-zA-Z0-9_]*  { yylval.sval = strdup(yytext); return IDENTIFIER; }

[ \t\n]+                { /* ignore whitespace */ }
.                       { printf("Unexpected character: %s\n", yytext); }

%%

int yywrap(void) {
    return 1;
}
