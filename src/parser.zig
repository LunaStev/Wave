const std = @import("std");
const wave_lexer = @import("lexer.zig");

pub const Parser = struct {
    lexer: wave_lexer.Lexer,
    current_token: wave_lexer.Token,

    // 생성자 함수
    pub fn init(lexer: wave_lexer.Lexer) Parser {
        const p = Parser{ .lexer = lexer, .current_token = lexer.next_token() };
        return p;
    }

    // 토큰을 소비하고 다음 토큰으로 이동하는 메서드
    fn eat(self: *Parser, token_type: wave_lexer.TokenType) !void {
        if (self.current_token.kind != token_type) {
            return error.UnexpectedToken;
        }
        self.current_token = self.lexer.next_token();
    }

    pub fn parse_function(self: *Parser) !void {
        self.eat(wave_lexer.TokenType.FUN) catch return;
        std.debug.print("Parsing function: {s}\n", .{self.current_token.value});
        self.eat(wave_lexer.TokenType.VAR) catch return;
        self.eat(wave_lexer.TokenType.CONST) catch return;
        self.eat(wave_lexer.TokenType.IF) catch return;
        self.eat(wave_lexer.TokenType.ELSE) catch return;
        self.eat(wave_lexer.TokenType.WHILE) catch return;
        self.eat(wave_lexer.TokenType.L_RB) catch return;
        self.eat(wave_lexer.TokenType.R_RB) catch return;
        self.eat(wave_lexer.TokenType.L_CB) catch return;
        self.eat(wave_lexer.TokenType.R_CB) catch return;
        self.eat(wave_lexer.TokenType.L_SB) catch return;
        self.eat(wave_lexer.TokenType.R_SB) catch return;
        self.eat(wave_lexer.TokenType.SEMI) catch return;
        self.eat(wave_lexer.TokenType.COLON) catch return;
        self.eat(wave_lexer.TokenType.DOT) catch return;
        self.eat(wave_lexer.TokenType.COMMA) catch return;
        self.eat(wave_lexer.TokenType.ADD) catch return;
        self.eat(wave_lexer.TokenType.SUB) catch return;
        self.eat(wave_lexer.TokenType.MUL) catch return;
        self.eat(wave_lexer.TokenType.DIV) catch return;
        self.eat(wave_lexer.TokenType.EQUAL) catch return;
        self.eat(wave_lexer.TokenType.D_EQUAL) catch return;
        self.eat(wave_lexer.TokenType.IMPORT) catch return;
        self.eat(wave_lexer.TokenType.IDENTIFIER) catch return;
        self.eat(wave_lexer.TokenType.NUMBER) catch return;
        self.eat(wave_lexer.TokenType.UNKNOWN) catch return;
        self.eat(wave_lexer.TokenType.ENDOFFILE) catch return;
    }

    pub fn parse(self: *Parser) !void {
        while (self.current_token.kind != wave_lexer.TokenType.ENDOFFILE) {
            switch (self.current_token.kind) {
                wave_lexer.TokenType.FUN => |_| {
                    self.parse_function() catch return;
                },
                else => {
                    std.debug.print("Unhandled token: {s} - {s}\n", .{ @tagName(self.current_token.kind), self.current_token.value });
                    self.current_token = self.lexer.next_token();
                },
            }
        }
    }
};
