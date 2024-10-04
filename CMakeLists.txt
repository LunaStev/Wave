cmake_minimum_required(VERSION 3.22)
project(Wave C)
set(CMAKE_C_STANDARD 11)

find_package(FLEX REQUIRED)
find_package(BISON REQUIRED)

BISON_TARGET(
        Parser
        ${CMAKE_SOURCE_DIR}/src/parser.y
        ${CMAKE_BINARY_DIR}/parser.c
        DEFINES_FILE ${CMAKE_BINARY_DIR}/parser.h  # parser.tab.h의 경로를 지정합니다.
        COMPILE_FLAGS "-d"  # Bison에게 헤더 파일을 생성하도록 지시합니다.
)
FLEX_TARGET(Lexer src/lexer.l  ${CMAKE_CURRENT_BINARY_DIR}/lexer.c)
ADD_FLEX_BISON_DEPENDENCY(Lexer Parser)

add_executable(Wave ${BISON_Parser_OUTPUTS} ${FLEX_Lexer_OUTPUTS} src/main.c
        src/parser.h)
