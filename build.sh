#!/bin/bash

# 색상 정의
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # 색상 초기화

# 구분선
LINE="---------------------------------------------"

# Wave/build 디렉토리로 이동
cd build || { echo -e "${RED}Build directory does not exist.${NC}"; exit 1; }

# 현재 디렉토리 확인
echo -e "\n${LINE}"
echo -e "${BLUE}Current directory: $(pwd)${NC}"
echo -e "${LINE}\n"

# 기존 파일 제거
echo -e "${YELLOW}Removing existing files...${NC}"
rm -rf *
echo -e "${LINE}\n"

# CMake 실행
echo -e "${GREEN}Running CMake...${NC}"
cmake ..
echo -e "${LINE}\n"

# 빌드 실행
echo -e "${GREEN}Building project...${NC}"
make
echo -e "${LINE}\n"

echo -e "${GREEN}Build complete.${NC}"
echo -e "${LINE}\n"
