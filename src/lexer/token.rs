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
    ISZ,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum UnsignedIntegerType {
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
            IntegerType::ISZ => "isz",
        };
        write!(f, "{}", name)
    }
}

impl fmt::Display for UnsignedIntegerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            UnsignedIntegerType::U4 => "u4",
            UnsignedIntegerType::U8 => "u8",
            UnsignedIntegerType::U16 => "u16",
            UnsignedIntegerType::U32 => "u32",
            UnsignedIntegerType::U64 => "u64",
            UnsignedIntegerType::U128 => "u128",
            UnsignedIntegerType::U256 => "u256",
            UnsignedIntegerType::U512 => "u512",
            UnsignedIntegerType::U1024 => "u1024",
            UnsignedIntegerType::U2048 => "u2048",
            UnsignedIntegerType::U4096 => "u4096",
            UnsignedIntegerType::U8192 => "u8192",
            UnsignedIntegerType::U16384 => "u16384",
            UnsignedIntegerType::U32768 => "u32768",
            UnsignedIntegerType::USZ => "usz",
        };
        write!(f, "{}", name)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum TokenType {
    Fun,
    Var,
    Imm,
    Const,
    If,
    Else,
    While,
    For,
    Import,
    Return,
    Continue,
    Input,
    Print,
    Println,
    Module,
    Class,
    Match,
    LogicalAnd,            // &&
    BitwiseAnd,            // &
    LogicalOr,             // ||
    BitwiseOr,             // |
    NotEqual,              // !=
    Xor,                    // ^
    Xnor,                   // ~^
    BitwiseNot,            // ~
    Nand,                   // !&
    Nor,                    // !|
    Not,                    // !
    Condition,              // ?
    NullCoalesce,          // ??
    Conditional,            // ?:
    In,                     // in
    Is,                     // is
    Char,
    Byte,
    Ptr,
    Array,
    Rol,
    Ror,
    Xnand,
    TypeInt(IntegerType),
    TypeUint(UnsignedIntegerType),
    TypeFloat(FloatType),
    TypeString,
    Identifier(String),
    String(String),
    Number(i64),
    Float(f64),
    Plus,                   // +
    Increment,              // ++
    Minus,                  // -
    Decrement,              // --
    Star,                   // *
    Div,                    // /
    Equal,                  // =
    EqualTwo,              // ==
    Comma,                  // ,
    Dot,                    // .
    SemiColon,              // ;
    Colon,                  // :
    Lchevr,                 // <
    LchevrEq,              // <=
    Rchevr,                 // >
    RchevrEq,              // >=
    Lparen,                 // (
    Rparen,                 // )
    Lbrace,                 // {
    Rbrace,                 // }
    Lbrack,                 // [
    Rbrack,                 // ]
    Eof,                    // End of file
    Error,
}