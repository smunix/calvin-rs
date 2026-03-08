//! Type substitution.
//!
//! A substitution maps type variables to concrete types. It is the core
//! mechanism used during type inference to record the results of unification.

use std::collections::HashMap;

use super::MonoType;

/// A substitution mapping type variable names to monomorphic types.
#[derive(Debug, Clone, Default)]
pub struct Substitution {
    /// Mappings from type variable names to their resolved types.
    vars: HashMap<String, MonoType>,
    /// Mappings from generic type indices (TGen) to their resolved types.
    gens: HashMap<usize, MonoType>,
}

impl Substitution {
    /// Create an empty substitution.
    pub fn new() -> Self {
        Substitution {
            vars: HashMap::new(),
            gens: HashMap::new(),
        }
    }

    /// Create a substitution from a list of TGen replacements.
    pub fn from_tgens(types: &[MonoType]) -> Self {
        let gens = types
            .iter()
            .enumerate()
            .map(|(i, t)| (i, t.clone()))
            .collect();
        Substitution {
            vars: HashMap::new(),
            gens,
        }
    }

    /// Bind a type variable to a type.
    pub fn bind(&mut self, name: &str, ty: MonoType) {
        self.vars.insert(name.to_string(), ty);
    }

    /// Look up a type variable in the substitution.
    pub fn lookup(&self, name: &str) -> Option<&MonoType> {
        self.vars.get(name)
    }

    /// Look up a generic type index in the substitution.
    pub fn lookup_gen(&self, idx: usize) -> Option<&MonoType> {
        self.gens.get(&idx)
    }

    /// Create a new substitution that excludes a given variable name.
    /// This is used to avoid capturing bound variables in existential
    /// and recursive types.
    pub fn without(&self, name: &str) -> Substitution {
        let vars = self
            .vars
            .iter()
            .filter(|(k, _)| k.as_str() != name)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Substitution {
            vars,
            gens: self.gens.clone(),
        }
    }

    /// Compose two substitutions: `self` after `other`.
    /// The resulting substitution first applies `other`, then `self`.
    pub fn compose(&self, other: &Substitution) -> Substitution {
        let mut result = other.clone();
        // Apply self to all types in other
        for ty in result.vars.values_mut() {
            *ty = ty.apply_subst(self);
        }
        for ty in result.gens.values_mut() {
            *ty = ty.apply_subst(self);
        }
        // Add bindings from self that are not in other
        for (k, v) in &self.vars {
            result.vars.entry(k.clone()).or_insert_with(|| v.clone());
        }
        for (k, v) in &self.gens {
            result.gens.entry(*k).or_insert_with(|| v.clone());
        }
        result
    }

    /// Returns true if this substitution is empty.
    pub fn is_empty(&self) -> bool {
        self.vars.is_empty() && self.gens.is_empty()
    }

    /// Returns an iterator over the variable bindings.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &MonoType)> {
        self.vars.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_substitution() {
        let subst = Substitution::new();
        assert!(subst.is_empty());
        assert_eq!(subst.lookup("a"), None);
    }

    #[test]
    fn test_bind_and_lookup() {
        let mut subst = Substitution::new();
        subst.bind("a", MonoType::Int);
        assert_eq!(subst.lookup("a"), Some(&MonoType::Int));
        assert_eq!(subst.lookup("b"), None);
    }

    #[test]
    fn test_apply_subst() {
        let mut subst = Substitution::new();
        subst.bind("a", MonoType::Int);
        let ty = MonoType::TVar("a".to_string());
        assert_eq!(ty.apply_subst(&subst), MonoType::Int);
    }

    #[test]
    fn test_compose() {
        let mut s1 = Substitution::new();
        s1.bind("a", MonoType::Int);
        let mut s2 = Substitution::new();
        s2.bind("b", MonoType::TVar("a".to_string()));
        let composed = s1.compose(&s2);
        // b -> a, then a -> Int, so b -> Int
        assert_eq!(composed.lookup("b"), Some(&MonoType::Int));
        assert_eq!(composed.lookup("a"), Some(&MonoType::Int));
    }

    #[test]
    fn test_without() {
        let mut subst = Substitution::new();
        subst.bind("a", MonoType::Int);
        subst.bind("b", MonoType::Bool);
        let filtered = subst.without("a");
        assert_eq!(filtered.lookup("a"), None);
        assert_eq!(filtered.lookup("b"), Some(&MonoType::Bool));
    }

    #[test]
    fn test_from_tgens() {
        let subst = Substitution::from_tgens(&[MonoType::Int, MonoType::Bool]);
        assert_eq!(subst.lookup_gen(0), Some(&MonoType::Int));
        assert_eq!(subst.lookup_gen(1), Some(&MonoType::Bool));
        assert_eq!(subst.lookup_gen(2), None);
    }
}
