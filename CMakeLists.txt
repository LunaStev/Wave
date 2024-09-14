cmake_minimum_required(VERSION 3.29)
project(Wave C)
set(CMAKE_C_STANDARD 11)
add_executable(Wave src/main.c
        src/lexer.c
        src/lexer.h
        src/wave.h)
