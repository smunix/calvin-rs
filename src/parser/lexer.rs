//! Lexer for the Calvin language.
//!
//! This module provides token definitions and a lexer that can be used
//! for more advanced parsing scenarios. The main parser in `mod.rs` uses
//! character-level parsing directly, but this module provides a token-based
//! alternative.

use std::fmt;

/// Token types for the Calvin language.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    IntLit(i32),
    LongLit(i64),
    ShortLit(i16),
    ByteLit(u8),
    FloatLit(f32),
    DoubleLit(f64),
    StringLit(String),
    CharLit(char),
    BoolLit(bool),
    UnitLit,

    // Identifiers and operators
    Ident(String),
    UpperIdent(String),
    Operator(String),

    // Keywords
    Let,
    In,
    If,
    Then,
    Else,
    Match,
    With,
    Fn,
    Do,
    Type,
    Class,
    Instance,
    Data,
    Import,
    Where,
    Forall,
    Exists,
    Mu,
    LetRec,
    Pack,
    Unpack,

    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Pipe,

    // Punctuation
    Comma,
    Semicolon,
    Dot,
    Colon,
    ColonColon,
    Arrow,
    FatArrow,
    Backslash,
    Equals,
    At,
    Underscore,

    // Special
    Eof,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::IntLit(n) => write!(f, "{}", n),
            Token::LongLit(n) => write!(f, "{}L", n),
            Token::ShortLit(n) => write!(f, "{}s", n),
            Token::ByteLit(n) => write!(f, "{}b", n),
            Token::FloatLit(n) => write!(f, "{}f", n),
            Token::DoubleLit(n) => write!(f, "{}", n),
            Token::StringLit(s) => write!(f, "\"{}\"", s),
            Token::CharLit(c) => write!(f, "'{}'", c),
            Token::BoolLit(b) => write!(f, "{}", b),
            Token::UnitLit => write!(f, "()"),
            Token::Ident(s) => write!(f, "{}", s),
            Token::UpperIdent(s) => write!(f, "{}", s),
            Token::Operator(s) => write!(f, "{}", s),
            Token::Let => write!(f, "let"),
            Token::In => write!(f, "in"),
            Token::If => write!(f, "if"),
            Token::Then => write!(f, "then"),
            Token::Else => write!(f, "else"),
            Token::Match => write!(f, "match"),
            Token::With => write!(f, "with"),
            Token::Fn => write!(f, "fn"),
            Token::Do => write!(f, "do"),
            Token::Type => write!(f, "type"),
            Token::Class => write!(f, "class"),
            Token::Instance => write!(f, "instance"),
            Token::Data => write!(f, "data"),
            Token::Import => write!(f, "import"),
            Token::Where => write!(f, "where"),
            Token::Forall => write!(f, "forall"),
            Token::Exists => write!(f, "exists"),
            Token::Mu => write!(f, "mu"),
            Token::LetRec => write!(f, "letrec"),
            Token::Pack => write!(f, "pack"),
            Token::Unpack => write!(f, "unpack"),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::Pipe => write!(f, "|"),
            Token::Comma => write!(f, ","),
            Token::Semicolon => write!(f, ";"),
            Token::Dot => write!(f, "."),
            Token::Colon => write!(f, ":"),
            Token::ColonColon => write!(f, "::"),
            Token::Arrow => write!(f, "->"),
            Token::FatArrow => write!(f, "=>"),
            Token::Backslash => write!(f, "\\"),
            Token::Equals => write!(f, "="),
            Token::At => write!(f, "@"),
            Token::Underscore => write!(f, "_"),
            Token::Eof => write!(f, "<EOF>"),
        }
    }
}

/// Classify a keyword string into its token type.
pub fn keyword_or_ident(s: &str) -> Token {
    match s {
        "let" => Token::Let,
        "in" => Token::In,
        "if" => Token::If,
        "then" => Token::Then,
        "else" => Token::Else,
        "match" => Token::Match,
        "with" => Token::With,
        "fn" => Token::Fn,
        "true" => Token::BoolLit(true),
        "false" => Token::BoolLit(false),
        "do" => Token::Do,
        "type" => Token::Type,
        "class" => Token::Class,
        "instance" => Token::Instance,
        "data" => Token::Data,
        "import" => Token::Import,
        "where" => Token::Where,
        "forall" => Token::Forall,
        "exists" => Token::Exists,
        "mu" => Token::Mu,
        "letrec" => Token::LetRec,
        "pack" => Token::Pack,
        "unpack" => Token::Unpack,
        _ => {
            if s.chars().next().map_or(false, |c| c.is_uppercase()) {
                Token::UpperIdent(s.to_string())
            } else {
                Token::Ident(s.to_string())
            }
        }
    }
}
