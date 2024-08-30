cmake_minimum_required(VERSION 3.29)
project(Wave)

set(CMAKE_CXX_STANDARD 26)

find_package(LLVM REQUIRED CONFIG)

include_directories(&{LLVM_INCLUDE_DIRS})
set(CMAKE_CXX_FLAGS "&{CMAKE_CXX_FLAGS} &{LLVM_CXX_FLAGS}")

add_executable(Wave src/main.cpp)

llvm_map_components_to_libnames(llvm_libs support core irreader)
target_link_libraries(Wave &{llvm_libs})