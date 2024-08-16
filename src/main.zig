const std = @import("std");
const wave_lexer = @import("lexer.zig");
const wave_parser = @import("parser.zig");

// pub fn main() !void {
//     var lexer = wave_lexer.Lexer.init("fun main() {    const a = 10 + 1; }");
//
//     var token = lexer.next_token();
//     while (token.kind != wave_lexer.TokenType.ENDOFFILE) {
//         std.debug.print("Token: {s} - {s}\n", .{ @tagName(token.kind), token.value });
//         token = lexer.next_token();
//     }
// }

pub fn main() !void {
    const lexer = wave_lexer.Lexer.init("import iosys; fun hello() { hello(); var a = 0;} fun main() { var a = 1;}");
    var parser = wave_parser.Parser.init(lexer);

    try parser.parse();
}
