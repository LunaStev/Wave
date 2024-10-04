#include <stdio.h>
#include <stdlib.h>
#include "parser.h"

// yyin을 정의합니다
extern FILE *yyin; // extern을 사용하여 yyin을 정의합니다
extern int yyparse();

int main(int argc, char *argv[]) {
    if (argc > 1) {
        FILE *file = fopen(argv[1], "r");
        if (!file) {
            perror("Failed to open file");
            return EXIT_FAILURE;
        }
        yyin = file; // yyin을 통해 파일에서 입력을 읽도록 설정
    } else {
        printf("Usage: %s <input_file>\n", argv[0]);
        return EXIT_FAILURE;
    }

    printf("Starting the Wave interpreter...\n");
    yyparse();

    return 0;
}
