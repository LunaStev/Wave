#include <iostream>
#include <sstream>
#include <string>
#include <unordered_map>
#include <functional>

class WaveCompiler {
public:
    void compile(const std::string& code) {
        std::istringstream stream(code);
        std::string token;
        bool inMainFunction = false; // main 함수 내부에 있는지 여부

        while (stream >> token) {
            if (token == "fun") {
                std::string functionName;
                stream >> functionName; // 함수 이름 읽기
                std::string openParen;
                stream >> openParen; // '(' 읽기

                // main 함수의 시작
                if (functionName == "main()") {
                    inMainFunction = true; // main 함수 내부로 진입
                    std::string line;
                    while (std::getline(stream, line)) {
                        if (line == "}") {
                            inMainFunction = false; // main 함수 끝
                            break;
                        }
                        processLine(line); // 각 줄 처리
                    }
                }
            }
        }
    }

private:
    std::unordered_map<std::string, int> variables; // 변수 저장

    void processLine(const std::string& line) {
        std::istringstream lineStream(line);
        std::string token;

        if (line.find("println") != std::string::npos) {
            // println 처리
            size_t start = line.find('(') + 1;
            size_t end = line.find(')', start);
            std::string msg = line.substr(start, end - start);
            formatAndPrint(msg); // 형식화하여 출력
        } else if (line.find("var") != std::string::npos) {
            // var 처리
            std::string varType, varName, equals;
            int value;
            lineStream >> token >> varName >> equals >> value; // var a = 10 형식
            variables[varName] = value; // 변수 저장
        } else {
            std::cerr << "Unknown command: " << line << std::endl; // 알 수 없는 명령어
        }
    }

    void formatAndPrint(const std::string& msg) {
        std::string formattedMsg = msg;
        size_t pos = formattedMsg.find("{d}");
        while (pos != std::string::npos) {
            std::string varName = formattedMsg.substr(pos + 3, formattedMsg.find(' ', pos + 3) - (pos + 3));
            if (variables.find(varName) != variables.end()) {
                formattedMsg.replace(pos, varName.length() + 4, std::to_string(variables[varName])); // 변수 값을 대체
            } else {
                break; // 변수 이름이 유효하지 않으면 종료
            }
            pos = formattedMsg.find("{d}", pos + 1);
        }
        std::cout << formattedMsg << std::endl; // 최종 메시지 출력
    }
};

int main() {
    WaveCompiler compiler;

    std::string waveCode = R"(
        import("iosys"); // 표준 입출력 라이브러리
        fun main() {
            var a: i32 = 10; // 정수형 변수 a 선언
            var b: i32 = 20; // 정수형 변수 b 선언

            println("Hello, Wave!"); // 문자열 출력
            println("a + b = {d}", a + b); // a와 b의 합 출력

            var ptr: *i32 = &a; // a의 주소를 ptr에 저장
            *ptr = 30; // 포인터를 통해 a의 값을 변경
            println("Updated a = {d}", a); // 업데이트된 a 출력
        }
    )";

    compiler.compile(waveCode); // Wave 코드 실행

    return 0;
}
