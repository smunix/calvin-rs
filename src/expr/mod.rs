//! Expression AST for the Calvin language.
//!
//! This module defines the abstract syntax tree (AST) for Calvin expressions,
//! including literals, variables, function application, pattern matching,
//! records, variants, arrays, and comprehensions.

use crate::types::MonoType;
use std::collections::BTreeMap;
use std::fmt;

/// Source location information for error reporting.
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }

    pub fn dummy() -> Self {
        Span { start: 0, end: 0 }
    }
}

/// A spanned expression node, carrying source location information.
#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Spanned { node, span }
    }

    pub fn dummy(node: T) -> Self {
        Spanned {
            node,
            span: Span::dummy(),
        }
    }
}

/// A literal value in the Calvin language.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Unit,
    Bool(bool),
    Char(char),
    Byte(u8),
    Short(i16),
    Int(i32),
    Long(i64),
    Int128(i128),
    Float(f32),
    Double(f64),
    String(String),
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Literal::Unit => write!(f, "()"),
            Literal::Bool(b) => write!(f, "{}", b),
            Literal::Char(c) => write!(f, "'{}'", c),
            Literal::Byte(b) => write!(f, "{}b", b),
            Literal::Short(s) => write!(f, "{}s", s),
            Literal::Int(i) => write!(f, "{}", i),
            Literal::Long(l) => write!(f, "{}L", l),
            Literal::Int128(i) => write!(f, "{}q", i),
            Literal::Float(v) => write!(f, "{}f", v),
            Literal::Double(v) => write!(f, "{}", v),
            Literal::String(s) => write!(f, "\"{}\"", s),
        }
    }
}

impl Literal {
    /// Get the type of this literal.
    pub fn ty(&self) -> MonoType {
        match self {
            Literal::Unit => MonoType::Unit,
            Literal::Bool(_) => MonoType::Bool,
            Literal::Char(_) => MonoType::Char,
            Literal::Byte(_) => MonoType::Byte,
            Literal::Short(_) => MonoType::Short,
            Literal::Int(_) => MonoType::Int,
            Literal::Long(_) => MonoType::Long,
            Literal::Int128(_) => MonoType::Int128,
            Literal::Float(_) => MonoType::Float,
            Literal::Double(_) => MonoType::Double,
            Literal::String(_) => MonoType::Str,
        }
    }
}

/// An expression in the Calvin language.
#[derive(Debug, Clone)]
pub enum Expr {
    /// A literal value.
    Lit(Literal),

    /// A variable reference.
    Var(String),

    /// A let binding: `let x = e1 in e2`.
    Let(String, Box<Spanned<Expr>>, Box<Spanned<Expr>>),

    /// A recursive let binding: `letrec { x1 = e1, x2 = e2, ... } in body`.
    LetRec(Vec<(String, Spanned<Expr>)>, Box<Spanned<Expr>>),

    /// Function application: `f(a1, a2, ...)`.
    App(Box<Spanned<Expr>>, Vec<Spanned<Expr>>),

    /// Lambda abstraction: `\x y -> body`.
    Lambda(Vec<String>, Box<Spanned<Expr>>),

    /// If-then-else expression.
    If(Box<Spanned<Expr>>, Box<Spanned<Expr>>, Box<Spanned<Expr>>),

    /// Pattern matching: `match e { p1 -> e1, p2 -> e2, ... }`.
    Match(Box<Spanned<Expr>>, Vec<MatchArm>),

    /// Record constructor: `{ x = e1, y = e2, ... }`.
    MkRecord(BTreeMap<String, Spanned<Expr>>),

    /// Record field projection: `e.field`.
    Project(Box<Spanned<Expr>>, String),

    /// Variant constructor: `|Ctor e|`.
    MkVariant(String, Box<Spanned<Expr>>),

    /// Array constructor: `[e1, e2, ...]`.
    MkArray(Vec<Spanned<Expr>>),

    /// Array comprehension: `[e | x <- xs, pred]`.
    Comprehension(Box<ComprehensionExpr>),

    /// Type annotation: `e :: T`.
    Annotate(Box<Spanned<Expr>>, MonoType),

    /// Binary operator application (desugared to App).
    BinOp(String, Box<Spanned<Expr>>, Box<Spanned<Expr>>),

    /// Unary operator application.
    UnaryOp(String, Box<Spanned<Expr>>),

    /// A do-block (sequence of expressions).
    Do(Vec<Spanned<Expr>>),

    /// Pack an existential type.
    Pack(Box<Spanned<Expr>>, MonoType),

    /// Unpack an existential type.
    Unpack(String, String, Box<Spanned<Expr>>, Box<Spanned<Expr>>),
}

/// A match arm: a pattern and its corresponding expression.
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Spanned<Expr>>,
    pub body: Spanned<Expr>,
}

/// A pattern used in match expressions.
#[derive(Debug, Clone)]
pub enum Pattern {
    /// A wildcard pattern: `_`.
    Wildcard,

    /// A variable binding pattern.
    Var(String),

    /// A literal pattern.
    Lit(Literal),

    /// A record pattern: `{ x = p1, y = p2, ... }`.
    Record(BTreeMap<String, Pattern>),

    /// A variant pattern: `|Ctor p|`.
    Variant(String, Box<Pattern>),

    /// An array pattern: `[p1, p2, ...]`.
    Array(Vec<Pattern>),

    /// A tuple pattern: `(p1, p2, ...)`.
    Tuple(Vec<Pattern>),

    /// A constructor pattern with arguments.
    Constructor(String, Vec<Pattern>),

    /// A pattern with a guard.
    Guard(Box<Pattern>, Box<Spanned<Expr>>),

    /// An "as" pattern: `p @ name`.
    As(Box<Pattern>, String),
}

/// A comprehension expression.
#[derive(Debug, Clone)]
pub struct ComprehensionExpr {
    /// The body expression to evaluate for each element.
    pub body: Spanned<Expr>,
    /// The qualifiers (generators and filters).
    pub qualifiers: Vec<Qualifier>,
}

/// A qualifier in a comprehension.
#[derive(Debug, Clone)]
pub enum Qualifier {
    /// A generator: `x <- xs`.
    Generator(Pattern, Spanned<Expr>),
    /// A filter/guard: `pred`.
    Filter(Spanned<Expr>),
    /// A let binding in a comprehension.
    Let(String, Spanned<Expr>),
}

/// A top-level definition in a module.
#[derive(Debug, Clone)]
pub enum Definition {
    /// A value definition: `let name = expr`.
    Value(String, Spanned<Expr>),
    /// A function definition: `let name(args) = expr`.
    Function(String, Vec<String>, Spanned<Expr>),
    /// A type alias: `type Name = Type`.
    TypeAlias(String, MonoType),
    /// A type class definition.
    Class {
        name: String,
        params: Vec<String>,
        supers: Vec<crate::types::Constraint>,
        members: Vec<(String, MonoType)>,
    },
    /// A type class instance.
    Instance {
        class_name: String,
        types: Vec<MonoType>,
        constraints: Vec<crate::types::Constraint>,
        members: Vec<(String, Spanned<Expr>)>,
    },
    /// A data type definition.
    Data {
        name: String,
        params: Vec<String>,
        constructors: Vec<(String, Vec<MonoType>)>,
    },
    /// An import declaration.
    Import(String),
}

/// A module is a collection of definitions.
#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub definitions: Vec<Definition>,
}

impl Module {
    pub fn new(name: &str) -> Self {
        Module {
            name: name.to_string(),
            definitions: Vec::new(),
        }
    }

    pub fn add_definition(&mut self, def: Definition) {
        self.definitions.push(def);
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Lit(lit) => write!(f, "{}", lit),
            Expr::Var(name) => write!(f, "{}", name),
            Expr::Let(name, val, body) => {
                write!(f, "let {} = {} in {}", name, val.node, body.node)
            }
            Expr::LetRec(bindings, body) => {
                write!(f, "letrec {{")?;
                for (i, (name, expr)) in bindings.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, " {} = {}", name, expr.node)?;
                }
                write!(f, " }} in {}", body.node)
            }
            Expr::App(func, args) => {
                write!(f, "{}(", func.node)?;
                let arg_strs: Vec<String> = args.iter().map(|a| format!("{}", a.node)).collect();
                write!(f, "{})", arg_strs.join(", "))
            }
            Expr::Lambda(params, body) => {
                write!(f, "\\{} -> {}", params.join(" "), body.node)
            }
            Expr::If(cond, then_e, else_e) => {
                write!(
                    f,
                    "if {} then {} else {}",
                    cond.node, then_e.node, else_e.node
                )
            }
            Expr::Match(scrutinee, _arms) => {
                write!(f, "match {} {{ ... }}", scrutinee.node)
            }
            Expr::MkRecord(fields) => {
                write!(f, "{{")?;
                let entries: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{} = {}", k, v.node))
                    .collect();
                write!(f, "{}}}", entries.join(", "))
            }
            Expr::Project(expr, field) => write!(f, "{}.{}", expr.node, field),
            Expr::MkVariant(ctor, expr) => write!(f, "|{} {}|", ctor, expr.node),
            Expr::MkArray(elems) => {
                let elem_strs: Vec<String> = elems.iter().map(|e| format!("{}", e.node)).collect();
                write!(f, "[{}]", elem_strs.join(", "))
            }
            Expr::Comprehension(comp) => {
                write!(f, "[{} | ...]", comp.body.node)
            }
            Expr::Annotate(expr, ty) => write!(f, "{} :: {}", expr.node, ty),
            Expr::BinOp(op, lhs, rhs) => {
                write!(f, "({} {} {})", lhs.node, op, rhs.node)
            }
            Expr::UnaryOp(op, expr) => write!(f, "({}{})", op, expr.node),
            Expr::Do(exprs) => {
                write!(f, "do {{")?;
                for e in exprs {
                    write!(f, " {};", e.node)?;
                }
                write!(f, " }}")
            }
            Expr::Pack(expr, ty) => write!(f, "pack({}, {})", expr.node, ty),
            Expr::Unpack(ty_var, val_var, pkg, body) => {
                write!(
                    f,
                    "unpack ({}, {}) = {} in {}",
                    ty_var, val_var, pkg.node, body.node
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_display() {
        assert_eq!(format!("{}", Literal::Int(42)), "42");
        assert_eq!(format!("{}", Literal::Bool(true)), "true");
        assert_eq!(format!("{}", Literal::String("hello".to_string())), "\"hello\"");
    }

    #[test]
    fn test_literal_types() {
        assert_eq!(Literal::Int(42).ty(), MonoType::Int);
        assert_eq!(Literal::Bool(true).ty(), MonoType::Bool);
        assert_eq!(Literal::Double(3.14).ty(), MonoType::Double);
    }

    #[test]
    fn test_expr_display() {
        let expr = Expr::BinOp(
            "+".to_string(),
            Box::new(Spanned::dummy(Expr::Lit(Literal::Int(1)))),
            Box::new(Spanned::dummy(Expr::Lit(Literal::Int(2)))),
        );
        assert_eq!(format!("{}", expr), "(1 + 2)");
    }

    #[test]
    fn test_lambda_display() {
        let expr = Expr::Lambda(
            vec!["x".to_string(), "y".to_string()],
            Box::new(Spanned::dummy(Expr::BinOp(
                "+".to_string(),
                Box::new(Spanned::dummy(Expr::Var("x".to_string()))),
                Box::new(Spanned::dummy(Expr::Var("y".to_string()))),
            ))),
        );
        assert_eq!(format!("{}", expr), "\\x y -> (x + y)");
    }
}
