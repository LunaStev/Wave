use std::fmt;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IntegerType {
    I4,
    I8,
    I16,
    I32,
    I64,
    I128,
    I256,
    I512,
    I1024,
    I2048,
    I4096,
    I8192,
    I16384,
    I32768,
    U4,
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
    U512,
    U1024,
    U2048,
    U4096,
    U8192,
    U16384,
    U32768,
    ISZ,
    USZ,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FloatType {
    F32,
    F64,
    F128,
    F256,
    F512,
    F1024,
    F2048,
    F4096,
    F8192,
    F16384,
    F32768,
}

impl fmt::Display for IntegerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            IntegerType::I4 => "i4",
            IntegerType::I8 => "i8",
            IntegerType::I16 => "i16",
            IntegerType::I32 => "i32",
            IntegerType::I64 => "i64",
            IntegerType::I128 => "i128",
            IntegerType::I256 => "i256",
            IntegerType::I512 => "i512",
            IntegerType::I1024 => "i1024",
            IntegerType::I2048 => "i2048",
            IntegerType::I4096 => "i4096",
            IntegerType::I8192 => "i8192",
            IntegerType::I16384 => "i16384",
            IntegerType::I32768 => "i32768",
            IntegerType::U4 => "u4",
            IntegerType::U8 => "u8",
            IntegerType::U16 => "u16",
            IntegerType::U32 => "u32",
            IntegerType::U64 => "u64",
            IntegerType::U128 => "u128",
            IntegerType::U256 => "u256",
            IntegerType::U512 => "u512",
            IntegerType::U1024 => "u1024",
            IntegerType::U2048 => "u2048",
            IntegerType::U4096 => "u4096",
            IntegerType::U8192 => "u8192",
            IntegerType::U16384 => "u16384",
            IntegerType::U32768 => "u32768",
            IntegerType::ISZ => "isz",
            IntegerType::USZ => "usz",
        };
        write!(f, "{}", name)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum TokenType {
    FUN,
    VAR,
    IMM,
    CONST,
    IF,
    ELSE,
    WHILE,
    FOR,
    IMPORT,
    RETURN,
    CONTINUE,
    INPUT,
    PRINT,
    PRINTLN,
    LogicalAnd,            // &&
    BitwiseAnd,            // &
    LogicalOr,             // ||
    BitwiseOr,             // |
    NotEqual,              // !=
    XOR,                    // ^
    XNOR,                   // ~^
    BitwiseNot,            // ~
    NAND,                   // !&
    NOR,                    // !|
    NOT,                    // !
    CONDITION,              // ?
    NullCoalesce,          // ??
    CONDITIONAL,            // ?:
    IN,                     // in
    IS,                     // is
    CHAR,
    BYTE,
    PTR,
    ARRAY,
    ROL,
    ROR,
    XNAND,
    TypeInt(IntegerType),
    TypeFloat(FloatType),
    TypeString,
    IDENTIFIER(String),
    STRING(String),
    NUMBER(f64),
    PLUS,                   // +
    INCREMENT,              // ++
    MINUS,                  // -
    DECREMENT,              // --
    STAR,                   // *
    DIV,                    // /
    EQUAL,                  // =
    EqualTwo,              // ==
    COMMA,                  // ,
    DOT,                    // .
    SEMICOLON,              // ;
    COLON,                  // :
    LCHEVR,                 // <
    LchevrEq,              // <=
    RCHEVR,                 // >
    RchevrEq,              // >=
    LPAREN,                 // (
    RPAREN,                 // )
    LBRACE,                 // {
    RBRACE,                 // }
    LBRACK,                 // [
    RBRACK,                 // ]
    EOF,                    // End of file
    ERROR,
}