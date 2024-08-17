const std = @import("std");
const wave_lexer = @import("lexer.zig");

pub const AstNode = union(enum) {
    FunctionDecl: struct {
        name: []const u8,
        body: []AstNode,
    },
    VarDecl: struct {
        name: []const u8,
        init_expr: ?*AstNode,
    },
    IfStmt: struct {
        condition: *AstNode,
        then_branch: []AstNode,
        else_branch: ?[]AstNode,
    },
    WhileStmt: struct {
        condition: *AstNode,
        body: []AstNode,
    },
    BinaryExpr: struct {
        left: *AstNode,
        op: wave_lexer.TokenType,
        right: *AstNode,
    },
    NumberLiteral: f64,
    Identifier: []const u8,
};

pub const ParseError = error{
    UnexpectedToken,
    OutOfMemory,
    InvalidCharacter,
};

pub const Parser = struct {
    lexer: *wave_lexer.Lexer,
    current_token: wave_lexer.Token,
    allocator: std.mem.Allocator,

    pub fn init(lexer: *wave_lexer.Lexer, allocator: std.mem.Allocator) Parser {
        var parser = Parser{
            .lexer = lexer,
            .current_token = undefined,
            .allocator = allocator,
        };
        parser.current_token = parser.lexer.next_token();
        return parser;
    }

    fn eat(self: *Parser, token_type: wave_lexer.TokenType) ParseError!void {
        if (self.current_token.kind == token_type) {
            self.current_token = self.lexer.next_token();
        } else {
            return ParseError.UnexpectedToken;
        }
    }

    pub fn parse(self: *Parser) ParseError![]AstNode {
        var nodes = std.ArrayList(AstNode).init(self.allocator);
        errdefer {
            for (nodes.items) |*node| {
                node.deinit();
            }
            nodes.deinit();
        }

        while (self.current_token.kind != .ENDOFFILE) {
            const node = try self.parse_statement();
            try nodes.append(node);
        }

        return nodes.toOwnedSlice();
    }

    fn parse_statement(self: *Parser) ParseError!AstNode {
        return switch (self.current_token.kind) {
            .FUN => try self.parse_function(),
            .VAR => try self.parse_var_decl(),
            .IF => try self.parse_if_stmt(),
            .WHILE => try self.parse_while_stmt(),
            else => ParseError.UnexpectedToken,
        };
    }

    fn parse_function(self: *Parser) ParseError!AstNode {
        try self.eat(.FUN);
        const name = self.current_token.value;
        try self.eat(.IDENTIFIER);
        try self.eat(.L_RB);
        try self.eat(.R_RB);
        try self.eat(.L_CB);

        var body = std.ArrayList(AstNode).init(self.allocator);
        errdefer {
            for (body.items) |*node| {
                node.deinit();
            }
            body.deinit();
        }

        while (self.current_token.kind != .R_CB) {
            const stmt = try self.parse_statement();
            try body.append(stmt);
        }
        try self.eat(.R_CB);

        return AstNode{
            .kind = .{ .FunctionDecl = .{ .name = name, .body = try body.toOwnedSlice() } },
            .allocator = self.allocator,
        };
    }

    fn parse_var_decl(self: *Parser) ParseError!AstNode {
        try self.eat(.VAR);
        const name = self.current_token.value;
        try self.eat(.IDENTIFIER);

        var init_expr: ?*AstNode = null;
        if (self.current_token.kind == .EQUAL) {
            try self.eat(.EQUAL);
            const expr = try self.allocator.create(AstNode);
            expr.* = try self.parse_expression();
            init_expr = expr;
        }

        try self.eat(.SEMI);
        return AstNode{ .VarDecl = .{ .name = name, .init_expr = init_expr } };
    }

    fn parse_if_stmt(self: *Parser) !AstNode {
        try self.eat(.IF);
        try self.eat(.L_RB);
        const condition = try self.allocator.create(AstNode);
        condition.* = try self.parse_expression();
        try self.eat(.R_RB);
        try self.eat(.L_CB);

        var then_branch = std.ArrayList(AstNode).init(self.allocator);
        defer then_branch.deinit();

        while (self.current_token.kind != .R_CB) {
            const stmt = try self.parse_statement();
            try then_branch.append(stmt);
        }
        try self.eat(.R_CB);

        var else_branch: ?[]AstNode = null;
        if (self.current_token.kind == .ELSE) {
            try self.eat(.ELSE);
            try self.eat(.L_CB);
            var else_stmts = std.ArrayList(AstNode).init(self.allocator);
            defer else_stmts.deinit();

            while (self.current_token.kind != .R_CB) {
                const stmt = try self.parse_statement();
                try else_stmts.append(stmt);
            }
            try self.eat(.R_CB);
            else_branch = try else_stmts.toOwnedSlice();
        }

        return AstNode{ .IfStmt = .{
            .condition = condition,
            .then_branch = try then_branch.toOwnedSlice(),
            .else_branch = else_branch,
        } };
    }

    fn parse_while_stmt(self: *Parser) !AstNode {
        try self.eat(.WHILE);
        try self.eat(.L_RB);
        const condition = try self.allocator.create(AstNode);
        condition.* = try self.parse_expression();
        try self.eat(.R_RB);
        try self.eat(.L_CB);

        var body = std.ArrayList(AstNode).init(self.allocator);
        defer body.deinit();

        while (self.current_token.kind != .R_CB) {
            const stmt = try self.parse_statement();
            try body.append(stmt);
        }
        try self.eat(.R_CB);

        return AstNode{ .WhileStmt = .{
            .condition = condition,
            .body = try body.toOwnedSlice(),
        } };
    }

    fn parse_expression(self: *Parser) ParseError!AstNode {
        return switch (self.current_token.kind) {
            .NUMBER => blk: {
                const value = std.fmt.parseFloat(f64, self.current_token.value) catch {
                    return ParseError.InvalidCharacter;
                };
                try self.eat(.NUMBER);
                break :blk AstNode{ .NumberLiteral = value };
            },
            .IDENTIFIER => blk: {
                const name = self.current_token.value;
                try self.eat(.IDENTIFIER);
                break :blk AstNode{ .Identifier = name };
            },
            else => ParseError.UnexpectedToken,
        };
    }
};
