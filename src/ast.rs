use std::error::Error;
use std::fmt;
use std::fmt::{Formatter, write};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Operator {
    /** BlockComment */
    BC,
    /** LineComment */
    LC,
    /** WhiteSpace */
    WS,
    /** TAB */
    TAB,
    /** NewLine */
    NL,

    FUN,                        // 함수
    VAR,                        // 문자 변수
    COUNT,                      // 숫자 변수
    IF,
    FOR,
    WHILE,
    ELSE,
    SWITCH,
    CASE,
    BREAK,

    CONTINUE,
    DEFAULT,

    ID_ENT,                     // 식별자
    STRING,                     // 문자열 리터럴
    NUMBER(i128),                     // 숫자 리터럴

    AND,                        // '&&'
    OR,                         // '||'
    NOT,                        // '!'

    EQUAL,                      // '='
    GREATER_THAN,               // '>'
    LEES_THAN,                  // '<'
    GREATER_THAN_EQUAL,         // '>='
    LESS_THAN_EQUAL,            // '<='
    EQUAL_EQUAL,                // '=='
    NOT_EQUAL,                  // '!='
    INCREMENT,                  // '++'
    DECREMENT,                  // '--'
    ARROW,                      // '->'

    TRUE,
    FALSE,
    NULL,

    COMMA,                      // '.'
    SEMI,                       // ';'
    COLON,                      // ':'

    MARKS,                      // '''
    DOUBLE_MARKS,               // '"'

    L_BRA,                      // '('
    R_BRA,                      // ')'
    L_C_BRA,                    // '{'
    R_C_BRA,                    // '}'
    L_S_BRA,                    // '['
    R_S_BRA,                    // ']'
    L_A_BRA,                    // '<'
    R_A_BRA,                    // '>'

    PLUS,                       // '+'
    MINUS,                      // '-'
    TIMES,                      // '*'
    SLASH,                      // '/'

    IMPORT,
    RETURN,
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match &self {
            Operator::BC => write!(f, "BlockComment"),
            Operator::LC => write!(f, "LineComment"),
            Operator::WS => write!(f, "WhiteSpace"),
            Operator::TAB => write!(f, "TAB"),
            Operator::NL => write!(f, "NewLine"),

            Operator::FUN => write!(f, "fun"),
            Operator::VAR => write!(f, "var"),
            Operator::COUNT => write!(f, "count"),
            Operator::IF => write!(f, "if"),
            Operator::FOR => write!(f, "for"),
            Operator::WHILE => write!(f, "while"),
            Operator::ELSE => write!(f, "else"),
            Operator::SWITCH => write!(f, "switch"),
            Operator::CASE => write!(f, "case"),
            Operator::BREAK => write!(f, "break"),

            Operator::CONTINUE => write!(f, "continue"),
            Operator::DEFAULT => write!(f, "default"),

            Operator::ID_ENT => write!(f, "id"),
            Operator::STRING => write!(f, "string"),
            Operator::NUMBER(..) => write!(f, "number"),

            Operator::AND => write!(f, "&&"),
            Operator::OR => write!(f, "||"),
            Operator::NOT => write!(f, "!"),

            Operator::EQUAL => write!(f, "="),
            Operator::GREATER_THAN => write!(f, ">"),
            Operator::LEES_THAN => write!(f, "<"),
            Operator::GREATER_THAN_EQUAL => write!(f, ">="),
            Operator::LESS_THAN_EQUAL => write!(f, "<="),
            Operator::EQUAL_EQUAL => write!(f, "=="),
            Operator::NOT_EQUAL => write!(f, "!="),
            Operator::INCREMENT => write!(f, "++"),
            Operator::DECREMENT => write!(f, "--"),
            Operator::ARROW => write!(f, "->"),

            Operator::TRUE => write!(f, "true"),
            Operator::FALSE => write!(f, "false"),
            Operator::NULL => write!(f, "null"),

            Operator::COMMA => write!(f, "."),
            Operator::SEMI => write!(f, ";"),
            Operator::COLON => write!(f, ":"),

            Operator::PLUS => write!(f, "+"),
            Operator::MINUS => write!(f, "-"),
            Operator::TIMES => write!(f, "*"),
            Operator::SLASH => write!(f, "/"),

            Operator::IMPORT => write!(f, "import"),
            Operator::RETURN => write!(f, "return"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Int(i32),

    UnaryExpr {
        op: Operator,
        child: Box<Node>,
    },
    BinaryExpr {
        op: Operator,
        lhs: Box<Node>,
        rhs: Box<Node>,
    },
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match &self {
            Node::Int(n) => write!(f, "{}", n),
            Node::UnaryExpr { op, child }=> write!(f, "{}{}", op, child),
            Node::BinaryExpr { op, lhs, rhs } => write!(f, "{} {} {}", lhs, op, rhs)
        }
    }
}