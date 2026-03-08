//! The Calvin type system.
//!
//! This module defines the core types used throughout the Calvin language,
//! including monomorphic types, polymorphic type schemes, qualified types
//! with constraints, type environments, and type inference via unification.

pub mod env;
pub mod infer;
pub mod subst;

use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A globally unique counter for generating fresh type variables.
static TVAR_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Generate a fresh type variable name.
pub fn fresh_tvar() -> String {
    let n = TVAR_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("t{}", n)
}

/// Reset the type variable counter (useful for tests).
pub fn reset_tvar_counter() {
    TVAR_COUNTER.store(0, Ordering::SeqCst);
}

/// Represents a monomorphic type in the Calvin language.
#[derive(Debug, Clone, PartialEq)]
pub enum MonoType {
    /// The unit type, representing the absence of a meaningful value.
    Unit,
    /// A boolean type.
    Bool,
    /// A character type (Unicode scalar value).
    Char,
    /// An 8-bit unsigned integer.
    Byte,
    /// A 16-bit signed integer.
    Short,
    /// A 32-bit signed integer.
    Int,
    /// A 64-bit signed integer.
    Long,
    /// A 128-bit signed integer.
    Int128,
    /// A 32-bit floating-point number.
    Float,
    /// A 64-bit floating-point number.
    Double,
    /// A string type (array of characters).
    Str,
    /// A type variable, used during type inference.
    TVar(String),
    /// A generic type variable bound by a polytype.
    TGen(usize),
    /// A type constructor application (e.g., `List Int`).
    TApp(Box<MonoType>, Vec<MonoType>),
    /// A record type with named fields, ordered by field name.
    Record(BTreeMap<String, MonoType>),
    /// A variant (tagged union) type with named constructors.
    Variant(BTreeMap<String, MonoType>),
    /// A fixed-size array type.
    FixedArray(Box<MonoType>, usize),
    /// A dynamically-sized array type.
    Array(Box<MonoType>),
    /// An existential type: `exists a. T`.
    Exists(String, Box<MonoType>),
    /// A recursive type: `mu a. T`.
    Recursive(String, Box<MonoType>),
    /// A function type: `(A, B, ...) -> R`.
    Func(Vec<MonoType>, Box<MonoType>),
    /// A named type alias reference.
    Alias(String, Box<MonoType>),
}

impl MonoType {
    /// Returns `true` if this type is a primitive numeric type.
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            MonoType::Byte
                | MonoType::Short
                | MonoType::Int
                | MonoType::Long
                | MonoType::Int128
                | MonoType::Float
                | MonoType::Double
        )
    }

    /// Returns `true` if this type is a primitive type.
    pub fn is_primitive(&self) -> bool {
        matches!(
            self,
            MonoType::Unit
                | MonoType::Bool
                | MonoType::Char
                | MonoType::Byte
                | MonoType::Short
                | MonoType::Int
                | MonoType::Long
                | MonoType::Int128
                | MonoType::Float
                | MonoType::Double
                | MonoType::Str
        )
    }

    /// Returns the memory size of this type in bytes, if known statically.
    pub fn size_of(&self) -> Option<usize> {
        match self {
            MonoType::Unit => Some(0),
            MonoType::Bool => Some(1),
            MonoType::Char => Some(4),
            MonoType::Byte => Some(1),
            MonoType::Short => Some(2),
            MonoType::Int => Some(4),
            MonoType::Long => Some(8),
            MonoType::Int128 => Some(16),
            MonoType::Float => Some(4),
            MonoType::Double => Some(8),
            MonoType::FixedArray(elem, n) => elem.size_of().map(|s| s * n),
            _ => None,
        }
    }

    /// Collect all free type variables in this type.
    pub fn free_vars(&self) -> Vec<String> {
        match self {
            MonoType::TVar(name) => vec![name.clone()],
            MonoType::TApp(f, args) => {
                let mut vars = f.free_vars();
                vars.extend(args.iter().flat_map(|a| a.free_vars()));
                vars.sort();
                vars.dedup();
                vars
            }
            MonoType::Record(fields) => {
                let mut vars: Vec<String> = fields.values().flat_map(|t| t.free_vars()).collect();
                vars.sort();
                vars.dedup();
                vars
            }
            MonoType::Variant(ctors) => {
                let mut vars: Vec<String> = ctors.values().flat_map(|t| t.free_vars()).collect();
                vars.sort();
                vars.dedup();
                vars
            }
            MonoType::FixedArray(elem, _) | MonoType::Array(elem) => elem.free_vars(),
            MonoType::Exists(bound, body) => body
                .free_vars()
                .into_iter()
                .filter(|v| v != bound)
                .collect(),
            MonoType::Recursive(bound, body) => body
                .free_vars()
                .into_iter()
                .filter(|v| v != bound)
                .collect(),
            MonoType::Func(params, ret) => {
                let mut vars: Vec<String> = params.iter().flat_map(|p| p.free_vars()).collect();
                vars.extend(ret.free_vars());
                vars.sort();
                vars.dedup();
                vars
            }
            MonoType::Alias(_, inner) => inner.free_vars(),
            _ => vec![],
        }
    }

    /// Apply a substitution to this type.
    pub fn apply_subst(&self, subst: &subst::Substitution) -> MonoType {
        match self {
            MonoType::TVar(name) => subst
                .lookup(name)
                .cloned()
                .unwrap_or_else(|| self.clone()),
            MonoType::TGen(n) => subst
                .lookup_gen(*n)
                .cloned()
                .unwrap_or_else(|| self.clone()),
            MonoType::TApp(f, args) => MonoType::TApp(
                Box::new(f.apply_subst(subst)),
                args.iter().map(|a| a.apply_subst(subst)).collect(),
            ),
            MonoType::Record(fields) => MonoType::Record(
                fields
                    .iter()
                    .map(|(k, v)| (k.clone(), v.apply_subst(subst)))
                    .collect(),
            ),
            MonoType::Variant(ctors) => MonoType::Variant(
                ctors
                    .iter()
                    .map(|(k, v)| (k.clone(), v.apply_subst(subst)))
                    .collect(),
            ),
            MonoType::FixedArray(elem, n) => {
                MonoType::FixedArray(Box::new(elem.apply_subst(subst)), *n)
            }
            MonoType::Array(elem) => MonoType::Array(Box::new(elem.apply_subst(subst))),
            MonoType::Exists(bound, body) => {
                let filtered = subst.without(bound);
                MonoType::Exists(bound.clone(), Box::new(body.apply_subst(&filtered)))
            }
            MonoType::Recursive(bound, body) => {
                let filtered = subst.without(bound);
                MonoType::Recursive(bound.clone(), Box::new(body.apply_subst(&filtered)))
            }
            MonoType::Func(params, ret) => MonoType::Func(
                params.iter().map(|p| p.apply_subst(subst)).collect(),
                Box::new(ret.apply_subst(subst)),
            ),
            MonoType::Alias(name, inner) => {
                MonoType::Alias(name.clone(), Box::new(inner.apply_subst(subst)))
            }
            other => other.clone(),
        }
    }
}

impl fmt::Display for MonoType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MonoType::Unit => write!(f, "()"),
            MonoType::Bool => write!(f, "bool"),
            MonoType::Char => write!(f, "char"),
            MonoType::Byte => write!(f, "byte"),
            MonoType::Short => write!(f, "short"),
            MonoType::Int => write!(f, "int"),
            MonoType::Long => write!(f, "long"),
            MonoType::Int128 => write!(f, "int128"),
            MonoType::Float => write!(f, "float"),
            MonoType::Double => write!(f, "double"),
            MonoType::Str => write!(f, "str"),
            MonoType::TVar(name) => write!(f, "{}", name),
            MonoType::TGen(n) => write!(f, "'{}", n),
            MonoType::TApp(func, args) => {
                write!(f, "({}", func)?;
                for arg in args {
                    write!(f, " {}", arg)?;
                }
                write!(f, ")")
            }
            MonoType::Record(fields) => {
                write!(f, "{{")?;
                let entries: Vec<String> =
                    fields.iter().map(|(k, v)| format!("{}: {}", k, v)).collect();
                write!(f, "{}", entries.join(", "))?;
                write!(f, "}}")
            }
            MonoType::Variant(ctors) => {
                let entries: Vec<String> =
                    ctors.iter().map(|(k, v)| format!("{} {}", k, v)).collect();
                write!(f, "({})", entries.join(" | "))
            }
            MonoType::FixedArray(elem, n) => write!(f, "[{}; {}]", elem, n),
            MonoType::Array(elem) => write!(f, "[{}]", elem),
            MonoType::Exists(var, body) => write!(f, "exists {}. {}", var, body),
            MonoType::Recursive(var, body) => write!(f, "mu {}. {}", var, body),
            MonoType::Func(params, ret) => {
                let ps: Vec<String> = params.iter().map(|p| format!("{}", p)).collect();
                write!(f, "({}) -> {}", ps.join(", "), ret)
            }
            MonoType::Alias(name, _) => write!(f, "{}", name),
        }
    }
}

/// A constraint on types, used in qualified types.
#[derive(Debug, Clone, PartialEq)]
pub struct Constraint {
    /// The name of the type class or predicate.
    pub name: String,
    /// The types involved in the constraint.
    pub types: Vec<MonoType>,
}

impl fmt::Display for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        for t in &self.types {
            write!(f, " {}", t)?;
        }
        Ok(())
    }
}

/// A qualified type: a monotype with a set of constraints.
#[derive(Debug, Clone, PartialEq)]
pub struct QualType {
    /// The constraints that must be satisfied.
    pub constraints: Vec<Constraint>,
    /// The underlying monomorphic type.
    pub mono: MonoType,
}

impl QualType {
    /// Create a new qualified type with no constraints.
    pub fn unqualified(mono: MonoType) -> Self {
        QualType {
            constraints: vec![],
            mono,
        }
    }

    /// Create a new qualified type with constraints.
    pub fn qualified(constraints: Vec<Constraint>, mono: MonoType) -> Self {
        QualType { constraints, mono }
    }
}

impl fmt::Display for QualType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.constraints.is_empty() {
            let cs: Vec<String> = self.constraints.iter().map(|c| format!("{}", c)).collect();
            write!(f, "({}) => ", cs.join(", "))?;
        }
        write!(f, "{}", self.mono)
    }
}

/// A polymorphic type scheme: universally quantified type variables
/// over a qualified type.
#[derive(Debug, Clone, PartialEq)]
pub struct PolyType {
    /// The number of universally quantified type variables.
    pub vars: usize,
    /// The qualified type body.
    pub body: QualType,
}

impl PolyType {
    /// Create a monomorphic polytype (no quantified variables).
    pub fn mono(ty: MonoType) -> Self {
        PolyType {
            vars: 0,
            body: QualType::unqualified(ty),
        }
    }

    /// Create a polytype from a qualified type.
    pub fn from_qual(vars: usize, qt: QualType) -> Self {
        PolyType { vars, body: qt }
    }

    /// Instantiate this polytype with fresh type variables.
    pub fn instantiate(&self) -> QualType {
        let fresh: Vec<MonoType> = (0..self.vars).map(|_| MonoType::TVar(fresh_tvar())).collect();
        let subst = subst::Substitution::from_tgens(&fresh);
        QualType {
            constraints: self
                .body
                .constraints
                .iter()
                .map(|c| Constraint {
                    name: c.name.clone(),
                    types: c.types.iter().map(|t| t.apply_subst(&subst)).collect(),
                })
                .collect(),
            mono: self.body.mono.apply_subst(&subst),
        }
    }
}

impl fmt::Display for PolyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.vars > 0 {
            write!(f, "forall")?;
            for i in 0..self.vars {
                write!(f, " '{}", i)?;
            }
            write!(f, ". ")?;
        }
        write!(f, "{}", self.body)
    }
}

/// A type class definition.
#[derive(Debug, Clone)]
pub struct TypeClass {
    /// The name of the type class.
    pub name: String,
    /// The type parameters of the class.
    pub params: Vec<String>,
    /// Super-class constraints.
    pub supers: Vec<Constraint>,
    /// The member functions of the class.
    pub members: BTreeMap<String, QualType>,
}

/// A type class instance.
#[derive(Debug, Clone)]
pub struct TypeClassInstance {
    /// The class this is an instance of.
    pub class_name: String,
    /// The concrete types for the class parameters.
    pub types: Vec<MonoType>,
    /// Constraints required for this instance.
    pub constraints: Vec<Constraint>,
    /// The member function implementations (as expression names).
    pub members: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_display() {
        assert_eq!(format!("{}", MonoType::Int), "int");
        assert_eq!(format!("{}", MonoType::Bool), "bool");
        assert_eq!(format!("{}", MonoType::Unit), "()");
    }

    #[test]
    fn test_func_display() {
        let func_ty = MonoType::Func(vec![MonoType::Int, MonoType::Int], Box::new(MonoType::Bool));
        assert_eq!(format!("{}", func_ty), "(int, int) -> bool");
    }

    #[test]
    fn test_record_display() {
        let mut fields = BTreeMap::new();
        fields.insert("x".to_string(), MonoType::Int);
        fields.insert("y".to_string(), MonoType::Double);
        let rec = MonoType::Record(fields);
        assert_eq!(format!("{}", rec), "{x: int, y: double}");
    }

    #[test]
    fn test_free_vars() {
        let ty = MonoType::Func(
            vec![MonoType::TVar("a".to_string())],
            Box::new(MonoType::TVar("b".to_string())),
        );
        let mut fv = ty.free_vars();
        fv.sort();
        assert_eq!(fv, vec!["a", "b"]);
    }

    #[test]
    fn test_is_numeric() {
        assert!(MonoType::Int.is_numeric());
        assert!(MonoType::Double.is_numeric());
        assert!(!MonoType::Bool.is_numeric());
        assert!(!MonoType::Str.is_numeric());
    }

    #[test]
    fn test_size_of() {
        assert_eq!(MonoType::Unit.size_of(), Some(0));
        assert_eq!(MonoType::Int.size_of(), Some(4));
        assert_eq!(MonoType::Long.size_of(), Some(8));
        assert_eq!(
            MonoType::FixedArray(Box::new(MonoType::Int), 10).size_of(),
            Some(40)
        );
    }
}
