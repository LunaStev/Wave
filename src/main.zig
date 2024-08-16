const std = @import("std");
const wave_lexer = @import("lexer.zig");

pub fn main() !void {
    var lexer = wave_lexer.Lexer.init("fun main() {    const a = 10 + 1; }");

    var token = lexer.next_token();
    while (token.kind != wave_lexer.TokenType.ENDOFFILE) {
        std.debug.print("Token: {s} - {s}\n", .{ @tagName(token.kind), token.value });
        token = lexer.next_token();
    }
}
