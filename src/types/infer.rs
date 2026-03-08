//! Type inference via unification.
//!
//! This module implements Hindley-Milner style type inference with
//! unification. It provides the core algorithm for determining the
//! types of expressions.

use super::subst::Substitution;
use super::MonoType;
use thiserror::Error;

/// Errors that can occur during type inference.
#[derive(Debug, Error)]
pub enum TypeError {
    #[error("Cannot unify {0} with {1}")]
    UnificationFailure(MonoType, MonoType),

    #[error("Occurs check failed: {0} occurs in {1}")]
    OccursCheck(String, MonoType),

    #[error("Unbound variable: {0}")]
    UnboundVariable(String),

    #[error("Type mismatch: expected {expected}, found {found}")]
    TypeMismatch { expected: MonoType, found: MonoType },

    #[error("Record field not found: {0}")]
    FieldNotFound(String),

    #[error("Variant constructor not found: {0}")]
    ConstructorNotFound(String),

    #[error("Arity mismatch: expected {expected} arguments, found {found}")]
    ArityMismatch { expected: usize, found: usize },

    #[error("Cannot resolve constraint: {0}")]
    UnresolvedConstraint(String),

    #[error("Ambiguous type: {0}")]
    AmbiguousType(String),

    #[error("{0}")]
    Other(String),
}

/// Perform the occurs check: ensure that a type variable does not
/// appear in a type, which would create an infinite type.
fn occurs_check(name: &str, ty: &MonoType) -> bool {
    match ty {
        MonoType::TVar(n) => n == name,
        MonoType::TApp(f, args) => {
            occurs_check(name, f) || args.iter().any(|a| occurs_check(name, a))
        }
        MonoType::Record(fields) => fields.values().any(|t| occurs_check(name, t)),
        MonoType::Variant(ctors) => ctors.values().any(|t| occurs_check(name, t)),
        MonoType::FixedArray(elem, _) | MonoType::Array(elem) => occurs_check(name, elem),
        MonoType::Exists(bound, body) => {
            if bound == name {
                false
            } else {
                occurs_check(name, body)
            }
        }
        MonoType::Recursive(bound, body) => {
            if bound == name {
                false
            } else {
                occurs_check(name, body)
            }
        }
        MonoType::Func(params, ret) => {
            params.iter().any(|p| occurs_check(name, p)) || occurs_check(name, ret)
        }
        MonoType::Alias(_, inner) => occurs_check(name, inner),
        _ => false,
    }
}

/// Unify two monomorphic types, producing a substitution that makes them equal.
pub fn unify(t1: &MonoType, t2: &MonoType) -> Result<Substitution, TypeError> {
    match (t1, t2) {
        // Identical primitive types
        (MonoType::Unit, MonoType::Unit)
        | (MonoType::Bool, MonoType::Bool)
        | (MonoType::Char, MonoType::Char)
        | (MonoType::Byte, MonoType::Byte)
        | (MonoType::Short, MonoType::Short)
        | (MonoType::Int, MonoType::Int)
        | (MonoType::Long, MonoType::Long)
        | (MonoType::Int128, MonoType::Int128)
        | (MonoType::Float, MonoType::Float)
        | (MonoType::Double, MonoType::Double)
        | (MonoType::Str, MonoType::Str) => Ok(Substitution::new()),

        // Type variable on the left
        (MonoType::TVar(name), ty) => {
            if let MonoType::TVar(n2) = ty {
                if name == n2 {
                    return Ok(Substitution::new());
                }
            }
            if occurs_check(name, ty) {
                return Err(TypeError::OccursCheck(name.clone(), ty.clone()));
            }
            let mut subst = Substitution::new();
            subst.bind(name, ty.clone());
            Ok(subst)
        }

        // Type variable on the right
        (ty, MonoType::TVar(name)) => {
            if occurs_check(name, ty) {
                return Err(TypeError::OccursCheck(name.clone(), ty.clone()));
            }
            let mut subst = Substitution::new();
            subst.bind(name, ty.clone());
            Ok(subst)
        }

        // Type application
        (MonoType::TApp(f1, args1), MonoType::TApp(f2, args2)) => {
            if args1.len() != args2.len() {
                return Err(TypeError::UnificationFailure(t1.clone(), t2.clone()));
            }
            let mut subst = unify(f1, f2)?;
            for (a1, a2) in args1.iter().zip(args2.iter()) {
                let a1s = a1.apply_subst(&subst);
                let a2s = a2.apply_subst(&subst);
                let s = unify(&a1s, &a2s)?;
                subst = s.compose(&subst);
            }
            Ok(subst)
        }

        // Record types
        (MonoType::Record(fields1), MonoType::Record(fields2)) => {
            if fields1.len() != fields2.len() {
                return Err(TypeError::UnificationFailure(t1.clone(), t2.clone()));
            }
            let mut subst = Substitution::new();
            for (name, ty1) in fields1 {
                let ty2 = fields2
                    .get(name)
                    .ok_or_else(|| TypeError::FieldNotFound(name.clone()))?;
                let ty1s = ty1.apply_subst(&subst);
                let ty2s = ty2.apply_subst(&subst);
                let s = unify(&ty1s, &ty2s)?;
                subst = s.compose(&subst);
            }
            Ok(subst)
        }

        // Variant types
        (MonoType::Variant(ctors1), MonoType::Variant(ctors2)) => {
            if ctors1.len() != ctors2.len() {
                return Err(TypeError::UnificationFailure(t1.clone(), t2.clone()));
            }
            let mut subst = Substitution::new();
            for (name, ty1) in ctors1 {
                let ty2 = ctors2
                    .get(name)
                    .ok_or_else(|| TypeError::ConstructorNotFound(name.clone()))?;
                let ty1s = ty1.apply_subst(&subst);
                let ty2s = ty2.apply_subst(&subst);
                let s = unify(&ty1s, &ty2s)?;
                subst = s.compose(&subst);
            }
            Ok(subst)
        }

        // Fixed-size arrays
        (MonoType::FixedArray(elem1, n1), MonoType::FixedArray(elem2, n2)) => {
            if n1 != n2 {
                return Err(TypeError::UnificationFailure(t1.clone(), t2.clone()));
            }
            unify(elem1, elem2)
        }

        // Dynamic arrays
        (MonoType::Array(elem1), MonoType::Array(elem2)) => unify(elem1, elem2),

        // Function types
        (MonoType::Func(params1, ret1), MonoType::Func(params2, ret2)) => {
            if params1.len() != params2.len() {
                return Err(TypeError::ArityMismatch {
                    expected: params1.len(),
                    found: params2.len(),
                });
            }
            let mut subst = Substitution::new();
            for (p1, p2) in params1.iter().zip(params2.iter()) {
                let p1s = p1.apply_subst(&subst);
                let p2s = p2.apply_subst(&subst);
                let s = unify(&p1s, &p2s)?;
                subst = s.compose(&subst);
            }
            let r1s = ret1.apply_subst(&subst);
            let r2s = ret2.apply_subst(&subst);
            let s = unify(&r1s, &r2s)?;
            Ok(s.compose(&subst))
        }

        // Aliases: unify through the alias
        (MonoType::Alias(_, inner), other) | (other, MonoType::Alias(_, inner)) => {
            unify(inner, other)
        }

        // Existential types
        (MonoType::Exists(v1, body1), MonoType::Exists(v2, body2)) => {
            // Alpha-rename v2 to v1 in body2
            let mut rename = Substitution::new();
            rename.bind(v2, MonoType::TVar(v1.clone()));
            let body2_renamed = body2.apply_subst(&rename);
            unify(body1, &body2_renamed)
        }

        // Recursive types
        (MonoType::Recursive(v1, body1), MonoType::Recursive(v2, body2)) => {
            let mut rename = Substitution::new();
            rename.bind(v2, MonoType::TVar(v1.clone()));
            let body2_renamed = body2.apply_subst(&rename);
            unify(body1, &body2_renamed)
        }

        // Failure
        _ => Err(TypeError::UnificationFailure(t1.clone(), t2.clone())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unify_same_primitives() {
        let result = unify(&MonoType::Int, &MonoType::Int);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_unify_different_primitives_fails() {
        let result = unify(&MonoType::Int, &MonoType::Bool);
        assert!(result.is_err());
    }

    #[test]
    fn test_unify_tvar_with_concrete() {
        let result = unify(&MonoType::TVar("a".to_string()), &MonoType::Int);
        assert!(result.is_ok());
        let subst = result.unwrap();
        assert_eq!(subst.lookup("a"), Some(&MonoType::Int));
    }

    #[test]
    fn test_unify_concrete_with_tvar() {
        let result = unify(&MonoType::Int, &MonoType::TVar("a".to_string()));
        assert!(result.is_ok());
        let subst = result.unwrap();
        assert_eq!(subst.lookup("a"), Some(&MonoType::Int));
    }

    #[test]
    fn test_unify_two_tvars() {
        let result = unify(
            &MonoType::TVar("a".to_string()),
            &MonoType::TVar("b".to_string()),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_unify_same_tvar() {
        let result = unify(
            &MonoType::TVar("a".to_string()),
            &MonoType::TVar("a".to_string()),
        );
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_occurs_check_fails() {
        // a ~ [a] should fail
        let result = unify(
            &MonoType::TVar("a".to_string()),
            &MonoType::Array(Box::new(MonoType::TVar("a".to_string()))),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_unify_functions() {
        let f1 = MonoType::Func(
            vec![MonoType::TVar("a".to_string())],
            Box::new(MonoType::TVar("b".to_string())),
        );
        let f2 = MonoType::Func(vec![MonoType::Int], Box::new(MonoType::Bool));
        let result = unify(&f1, &f2);
        assert!(result.is_ok());
        let subst = result.unwrap();
        assert_eq!(subst.lookup("a"), Some(&MonoType::Int));
        assert_eq!(subst.lookup("b"), Some(&MonoType::Bool));
    }

    #[test]
    fn test_unify_function_arity_mismatch() {
        let f1 = MonoType::Func(
            vec![MonoType::Int, MonoType::Int],
            Box::new(MonoType::Int),
        );
        let f2 = MonoType::Func(vec![MonoType::Int], Box::new(MonoType::Int));
        let result = unify(&f1, &f2);
        assert!(result.is_err());
    }

    #[test]
    fn test_unify_arrays() {
        let a1 = MonoType::Array(Box::new(MonoType::TVar("a".to_string())));
        let a2 = MonoType::Array(Box::new(MonoType::Int));
        let result = unify(&a1, &a2);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().lookup("a"), Some(&MonoType::Int));
    }

    #[test]
    fn test_unify_records() {
        use std::collections::BTreeMap;
        let mut fields1 = BTreeMap::new();
        fields1.insert("x".to_string(), MonoType::TVar("a".to_string()));
        let mut fields2 = BTreeMap::new();
        fields2.insert("x".to_string(), MonoType::Int);
        let result = unify(&MonoType::Record(fields1), &MonoType::Record(fields2));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().lookup("a"), Some(&MonoType::Int));
    }
}
