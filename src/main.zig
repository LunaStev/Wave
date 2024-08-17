const std = @import("std");
const wave_lexer = @import("lexer.zig");
const wave_parser = @import("parser.zig");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    const input = "fun main() { var x = 10; if (x > 5) { while (x > 0) { x = x - 1; } } }";
    var lexer = wave_lexer.Lexer.init(input);
    var parser = wave_parser.Parser.init(&lexer, allocator);

    const ast = parser.parse() catch |err| {
        std.debug.print("Error during parsing: {}\n", .{err});
        return;
    };
    defer {
        for (ast) |*node| {
            node.deinit();
        }
        allocator.free(ast);
    }

    std.debug.print("Parsed {} nodes\n", .{ast.len});
}
