const std = @import("std");
const wave_lexer = @import("lexer.zig");

pub const ParseError = error{
    UnexpectedToken,
    OutOfMemory,
    InvalidCharacter,
};

pub const Parser = struct {

};
