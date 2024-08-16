const std = @import("std");

pub const TokenType = enum {
    FUN, // fun
    VAR, // var
    CONST, // var
    IF, // if
    ELSE, // else
    WHILE, // while
    L_RB, // (
    R_RB, // )
    L_CB, // {
    R_CB, // }
    L_SB, // [
    R_SB, // ]
    SEMI, // ;
    COLON, // :
    DOT, // .
    ADD, // +
    SUB, // -
    MUL, // *
    DIV, // /
    EQUAL, // =
    D_EQUAL, // ==
    COMMA, // ,
    IDENTIFIER, // 함수 이름
    NUMBER, // 숫자
    UNKNOWN, // 없음
    ENDOFFILE,
};

const Token = struct {
    kind: TokenType,
    value: []const u8,
};

pub const Lexer = struct {
    text: []const u8,
    pos: usize,
    current_char: u8,

    pub fn init(text: []const u8) Lexer {
        return Lexer{
            .text = text,
            .pos = 0,
            .current_char = text[0],
        };
    }

    fn advance(self: *Lexer) void {
        self.pos += 1;
        if (self.pos >= self.text.len) {
            self.current_char = 0; // End of file
        } else {
            self.current_char = self.text[self.pos];
        }
    }

    fn skip_whitespace(self: *Lexer) void {
        while (self.current_char == ' ' or self.current_char == '\t' or self.current_char == '\n' or self.current_char == '\r') {
            self.advance();
        }
    }

    fn get_identifier(self: *Lexer) []const u8 {
        const start_pos = self.pos;
        while (self.current_char >= 'a' and self.current_char <= 'z' or
            self.current_char >= 'A' and self.current_char <= 'Z' or
            self.current_char == '_')
        {
            self.advance();
        }
        return self.text[start_pos..self.pos];
    }

    fn get_number(self: *Lexer) []const u8 {
        const start_pos = self.pos;
        while (self.current_char >= '0' and self.current_char <= '9') {
            self.advance();
        }
        return self.text[start_pos..self.pos];
    }

    pub fn next_token(self: *Lexer) Token {
        while (self.current_char != 0) {
            if (self.current_char == ' ' or self.current_char == '\t' or self.current_char == '\n' or self.current_char == '\r') {
                self.skip_whitespace();
                continue;
            }

            if (self.current_char == '(') {
                self.advance();
                return Token{ .kind = TokenType.L_RB, .value = "(" };
            }

            if (self.current_char == ')') {
                self.advance();
                return Token{ .kind = TokenType.R_RB, .value = ")" };
            }

            if (self.current_char == '{') {
                self.advance();
                return Token{ .kind = TokenType.L_CB, .value = "{" };
            }

            if (self.current_char == '}') {
                self.advance();
                return Token{ .kind = TokenType.R_CB, .value = "}" };
            }

            if (self.current_char == '[') {
                self.advance();
                return Token{ .kind = TokenType.L_SB, .value = "[" };
            }

            if (self.current_char == ']') {
                self.advance();
                return Token{ .kind = TokenType.R_SB, .value = "]" };
            }

            if (self.current_char == ';') {
                self.advance();
                return Token{ .kind = TokenType.SEMI, .value = ";" };
            }

            if (self.current_char == ':') {
                self.advance();
                return Token{ .kind = TokenType.COLON, .value = ":" };
            }

            if (self.current_char == '.') {
                self.advance();
                return Token{ .kind = TokenType.DOT, .value = "." };
            }

            if (self.current_char == ',') {
                self.advance();
                return Token{ .kind = TokenType.COMMA, .value = "," };
            }

            if (self.current_char == '+') {
                self.advance();
                return Token{ .kind = TokenType.ADD, .value = "+" };
            }

            if (self.current_char == '-') {
                self.advance();
                return Token{ .kind = TokenType.SUB, .value = "-" };
            }

            if (self.current_char == '*') {
                self.advance();
                return Token{ .kind = TokenType.MUL, .value = "*" };
            }

            if (self.current_char == '/') {
                self.advance();
                return Token{ .kind = TokenType.DIV, .value = "/" };
            }

            if (self.current_char == '=') {
                self.advance();
                if (self.current_char == '=') {
                    self.advance();
                    return Token{ .kind = TokenType.EQUAL, .value = "==" };
                }
                return Token{ .kind = TokenType.EQUAL, .value = "=" };
            }

            if (self.current_char >= 'a' and self.current_char <= 'z' or
                self.current_char >= 'A' and self.current_char <= 'Z' or
                self.current_char == '_')
            {
                const id = self.get_identifier();
                if (std.mem.eql(u8, id, "fun")) return Token{ .kind = TokenType.FUN, .value = id };
                if (std.mem.eql(u8, id, "var")) return Token{ .kind = TokenType.VAR, .value = id };
                if (std.mem.eql(u8, id, "if")) return Token{ .kind = TokenType.IF, .value = id };
                if (std.mem.eql(u8, id, "else")) return Token{ .kind = TokenType.ELSE, .value = id };
                if (std.mem.eql(u8, id, "while")) return Token{ .kind = TokenType.WHILE, .value = id };
                if (std.mem.eql(u8, id, "const")) return Token{ .kind = TokenType.CONST, .value = id };
                return Token{ .kind = TokenType.IDENTIFIER, .value = id };
            }

            if (self.current_char >= '0' and self.current_char <= '9') {
                return Token{ .kind = TokenType.NUMBER, .value = self.get_number() };
            }

            // Handle unknown characters
            self.advance();
            return Token{ .kind = TokenType.UNKNOWN, .value = "Unknown character" };
        }

        return Token{ .kind = TokenType.ENDOFFILE, .value = "" };
    }
};
