//! Runtime values for the Calvin evaluator.
//!
//! This module defines the `Value` type, which represents the result of
//! evaluating a Calvin expression at runtime.

use crate::expr::{Literal, Spanned, Expr};
use std::collections::BTreeMap;
use std::fmt;

/// A runtime value in the Calvin language.
#[derive(Debug, Clone)]
pub enum Value {
    /// The unit value.
    Unit,
    /// A boolean value.
    Bool(bool),
    /// A character value.
    Char(char),
    /// An 8-bit unsigned integer.
    Byte(u8),
    /// A 16-bit signed integer.
    Short(i16),
    /// A 32-bit signed integer.
    Int(i32),
    /// A 64-bit signed integer.
    Long(i64),
    /// A 128-bit signed integer.
    Int128(i128),
    /// A 32-bit floating-point number.
    Float(f32),
    /// A 64-bit floating-point number.
    Double(f64),
    /// A string value.
    String(String),
    /// An array of values.
    Array(Vec<Value>),
    /// A record (named tuple) of values.
    Record(BTreeMap<String, Value>),
    /// A variant (tagged union) value.
    Variant(String, Box<Value>),
    /// A closure (function value with captured environment).
    Closure {
        params: Vec<String>,
        body: Spanned<Expr>,
        env: super::Env,
    },
    /// A built-in function reference.
    BuiltinFn(String),
}

impl Value {
    /// Create a value from a literal.
    pub fn from_literal(lit: &Literal) -> Value {
        match lit {
            Literal::Unit => Value::Unit,
            Literal::Bool(b) => Value::Bool(*b),
            Literal::Char(c) => Value::Char(*c),
            Literal::Byte(b) => Value::Byte(*b),
            Literal::Short(s) => Value::Short(*s),
            Literal::Int(i) => Value::Int(*i),
            Literal::Long(l) => Value::Long(*l),
            Literal::Int128(i) => Value::Int128(*i),
            Literal::Float(f) => Value::Float(*f),
            Literal::Double(d) => Value::Double(*d),
            Literal::String(s) => Value::String(s.clone()),
        }
    }

    /// Check if this value is truthy.
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Unit => false,
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Long(n) => *n != 0,
            Value::Double(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Array(arr) => !arr.is_empty(),
            _ => true,
        }
    }

    /// Get the type name of this value (for error messages).
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Unit => "()",
            Value::Bool(_) => "bool",
            Value::Char(_) => "char",
            Value::Byte(_) => "byte",
            Value::Short(_) => "short",
            Value::Int(_) => "int",
            Value::Long(_) => "long",
            Value::Int128(_) => "int128",
            Value::Float(_) => "float",
            Value::Double(_) => "double",
            Value::String(_) => "str",
            Value::Array(_) => "array",
            Value::Record(_) => "record",
            Value::Variant(_, _) => "variant",
            Value::Closure { .. } => "function",
            Value::BuiltinFn(_) => "builtin",
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Unit => write!(f, "()"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Char(c) => write!(f, "'{}'", c),
            Value::Byte(b) => write!(f, "{}b", b),
            Value::Short(s) => write!(f, "{}s", s),
            Value::Int(i) => write!(f, "{}", i),
            Value::Long(l) => write!(f, "{}L", l),
            Value::Int128(i) => write!(f, "{}q", i),
            Value::Float(v) => write!(f, "{}f", v),
            Value::Double(v) => {
                if v.fract() == 0.0 {
                    write!(f, "{:.1}", v)
                } else {
                    write!(f, "{}", v)
                }
            }
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Array(elems) => {
                write!(f, "[")?;
                for (i, elem) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, "]")
            }
            Value::Record(fields) => {
                write!(f, "{{")?;
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{} = {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Variant(ctor, val) => write!(f, "|{} {}|", ctor, val),
            Value::Closure { params, .. } => {
                write!(f, "<fn({})>", params.join(", "))
            }
            Value::BuiltinFn(name) => write!(f, "<builtin:{}>", name),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Unit, Value::Unit) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::Byte(a), Value::Byte(b)) => a == b,
            (Value::Short(a), Value::Short(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Long(a), Value::Long(b)) => a == b,
            (Value::Int128(a), Value::Int128(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Double(a), Value::Double(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => a == b,
            (Value::Record(a), Value::Record(b)) => a == b,
            (Value::Variant(ca, va), Value::Variant(cb, vb)) => ca == cb && va == vb,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_display() {
        assert_eq!(format!("{}", Value::Int(42)), "42");
        assert_eq!(format!("{}", Value::Bool(true)), "true");
        assert_eq!(format!("{}", Value::String("hello".to_string())), "\"hello\"");
        assert_eq!(format!("{}", Value::Unit), "()");
    }

    #[test]
    fn test_value_equality() {
        assert_eq!(Value::Int(42), Value::Int(42));
        assert_ne!(Value::Int(42), Value::Int(43));
        assert_ne!(Value::Int(42), Value::Bool(true));
    }

    #[test]
    fn test_value_from_literal() {
        assert_eq!(Value::from_literal(&Literal::Int(42)), Value::Int(42));
        assert_eq!(Value::from_literal(&Literal::Bool(true)), Value::Bool(true));
    }

    #[test]
    fn test_is_truthy() {
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Bool(false).is_truthy());
        assert!(Value::Int(1).is_truthy());
        assert!(!Value::Int(0).is_truthy());
        assert!(!Value::Unit.is_truthy());
    }

    #[test]
    fn test_type_name() {
        assert_eq!(Value::Int(42).type_name(), "int");
        assert_eq!(Value::Bool(true).type_name(), "bool");
        assert_eq!(Value::String("hi".to_string()).type_name(), "str");
    }
}
