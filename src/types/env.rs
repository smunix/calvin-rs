//! Type environments.
//!
//! A type environment maps variable names to their polymorphic types,
//! and also tracks type aliases, type classes, and type class instances.

use std::collections::{BTreeMap, HashMap};

use super::{Constraint, MonoType, PolyType, QualType, TypeClass, TypeClassInstance};

/// A type environment that maps variable names to their types.
#[derive(Debug, Clone)]
pub struct TypeEnv {
    /// The parent environment (for lexical scoping).
    parent: Option<Box<TypeEnv>>,
    /// Variable bindings: name -> polymorphic type.
    bindings: HashMap<String, PolyType>,
    /// Type aliases: name -> underlying type.
    aliases: HashMap<String, MonoType>,
    /// Type class definitions.
    classes: HashMap<String, TypeClass>,
    /// Type class instances.
    instances: Vec<TypeClassInstance>,
}

impl TypeEnv {
    /// Create a new empty type environment.
    pub fn new() -> Self {
        TypeEnv {
            parent: None,
            bindings: HashMap::new(),
            aliases: HashMap::new(),
            classes: HashMap::new(),
            instances: Vec::new(),
        }
    }

    /// Create a child environment that inherits from this one.
    pub fn child(parent: TypeEnv) -> Self {
        TypeEnv {
            parent: Some(Box::new(parent)),
            bindings: HashMap::new(),
            aliases: HashMap::new(),
            classes: HashMap::new(),
            instances: Vec::new(),
        }
    }

    /// Bind a variable to a polymorphic type.
    pub fn bind(&mut self, name: &str, ty: PolyType) {
        self.bindings.insert(name.to_string(), ty);
    }

    /// Bind a variable to a monomorphic type (convenience).
    pub fn bind_mono(&mut self, name: &str, ty: MonoType) {
        self.bindings
            .insert(name.to_string(), PolyType::mono(ty));
    }

    /// Remove a binding.
    pub fn unbind(&mut self, name: &str) {
        self.bindings.remove(name);
    }

    /// Look up a variable in this environment, searching parent scopes.
    pub fn lookup(&self, name: &str) -> Option<&PolyType> {
        self.bindings
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.lookup(name)))
    }

    /// Check if a variable is bound in this environment (including parents).
    pub fn has_binding(&self, name: &str) -> bool {
        self.lookup(name).is_some()
    }

    /// Check if a variable is bound in this immediate environment only.
    pub fn has_immediate_binding(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }

    /// Register a type alias.
    pub fn alias(&mut self, name: &str, ty: MonoType) {
        self.aliases.insert(name.to_string(), ty);
    }

    /// Resolve a type alias.
    pub fn unalias(&self, name: &str) -> Option<&MonoType> {
        self.aliases
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.unalias(name)))
    }

    /// Register a type class.
    pub fn define_class(&mut self, class: TypeClass) {
        self.classes.insert(class.name.clone(), class);
    }

    /// Look up a type class definition.
    pub fn lookup_class(&self, name: &str) -> Option<&TypeClass> {
        self.classes
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.lookup_class(name)))
    }

    /// Register a type class instance.
    pub fn add_instance(&mut self, instance: TypeClassInstance) {
        self.instances.push(instance);
    }

    /// Find instances of a type class that match the given types.
    pub fn find_instances(&self, class_name: &str) -> Vec<&TypeClassInstance> {
        let mut results: Vec<&TypeClassInstance> = self
            .instances
            .iter()
            .filter(|i| i.class_name == class_name)
            .collect();
        if let Some(ref parent) = self.parent {
            results.extend(parent.find_instances(class_name));
        }
        results
    }

    /// Get all bound variable names in this environment.
    pub fn bound_variables(&self) -> Vec<String> {
        let mut vars: Vec<String> = self.bindings.keys().cloned().collect();
        if let Some(ref parent) = self.parent {
            vars.extend(parent.bound_variables());
        }
        vars.sort();
        vars.dedup();
        vars
    }

    /// Get all bindings as a map (including parent bindings).
    pub fn all_bindings(&self) -> BTreeMap<String, PolyType> {
        let mut result = BTreeMap::new();
        if let Some(ref parent) = self.parent {
            result.extend(parent.all_bindings());
        }
        result.extend(
            self.bindings
                .iter()
                .map(|(k, v)| (k.clone(), v.clone())),
        );
        result
    }

    /// Generalize a monotype over the free variables not in this environment.
    pub fn generalize(&self, qt: &QualType) -> PolyType {
        let env_vars: Vec<String> = self
            .all_bindings()
            .values()
            .flat_map(|pt| pt.body.mono.free_vars())
            .collect();

        let free = qt.mono.free_vars();
        let gen_vars: Vec<String> = free
            .into_iter()
            .filter(|v| !env_vars.contains(v))
            .collect();

        if gen_vars.is_empty() {
            PolyType {
                vars: 0,
                body: qt.clone(),
            }
        } else {
            // Replace free vars with TGen indices
            let mut subst = super::subst::Substitution::new();
            for (i, var) in gen_vars.iter().enumerate() {
                subst.bind(var, MonoType::TGen(i));
            }
            let generalized_mono = qt.mono.apply_subst(&subst);
            let generalized_constraints = qt
                .constraints
                .iter()
                .map(|c| Constraint {
                    name: c.name.clone(),
                    types: c.types.iter().map(|t| t.apply_subst(&subst)).collect(),
                })
                .collect();
            PolyType {
                vars: gen_vars.len(),
                body: QualType {
                    constraints: generalized_constraints,
                    mono: generalized_mono,
                },
            }
        }
    }
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a default type environment with built-in types and operators.
pub fn default_type_env() -> TypeEnv {
    let mut env = TypeEnv::new();

    // Arithmetic operators
    let int_binop = PolyType::mono(MonoType::Func(
        vec![MonoType::Int, MonoType::Int],
        Box::new(MonoType::Int),
    ));
    let long_binop = PolyType::mono(MonoType::Func(
        vec![MonoType::Long, MonoType::Long],
        Box::new(MonoType::Long),
    ));
    let double_binop = PolyType::mono(MonoType::Func(
        vec![MonoType::Double, MonoType::Double],
        Box::new(MonoType::Double),
    ));

    // Comparison operators
    let int_cmp = PolyType::mono(MonoType::Func(
        vec![MonoType::Int, MonoType::Int],
        Box::new(MonoType::Bool),
    ));

    // Boolean operators
    let bool_binop = PolyType::mono(MonoType::Func(
        vec![MonoType::Bool, MonoType::Bool],
        Box::new(MonoType::Bool),
    ));
    let bool_unop = PolyType::mono(MonoType::Func(
        vec![MonoType::Bool],
        Box::new(MonoType::Bool),
    ));

    // String operations
    let str_len = PolyType::mono(MonoType::Func(
        vec![MonoType::Str],
        Box::new(MonoType::Long),
    ));

    // Register built-in bindings
    env.bind("+", int_binop.clone());
    env.bind("-", int_binop.clone());
    env.bind("*", int_binop.clone());
    env.bind("/", int_binop.clone());
    env.bind("%", int_binop);

    env.bind("+l", long_binop.clone());
    env.bind("-l", long_binop.clone());
    env.bind("*l", long_binop.clone());
    env.bind("/l", long_binop);

    env.bind("+d", double_binop.clone());
    env.bind("-d", double_binop.clone());
    env.bind("*d", double_binop.clone());
    env.bind("/d", double_binop);

    env.bind("==", int_cmp.clone());
    env.bind("!=", int_cmp.clone());
    env.bind("<", int_cmp.clone());
    env.bind(">", int_cmp.clone());
    env.bind("<=", int_cmp.clone());
    env.bind(">=", int_cmp);

    env.bind("&&", bool_binop.clone());
    env.bind("||", bool_binop);
    env.bind("!", bool_unop);

    env.bind("strlen", str_len);

    // Type class: Show
    let show_class = TypeClass {
        name: "Show".to_string(),
        params: vec!["a".to_string()],
        supers: vec![],
        members: {
            let mut m = BTreeMap::new();
            m.insert(
                "show".to_string(),
                QualType::unqualified(MonoType::Func(
                    vec![MonoType::TVar("a".to_string())],
                    Box::new(MonoType::Str),
                )),
            );
            m
        },
    };
    env.define_class(show_class);

    // Type class: Eq
    let eq_class = TypeClass {
        name: "Eq".to_string(),
        params: vec!["a".to_string()],
        supers: vec![],
        members: {
            let mut m = BTreeMap::new();
            m.insert(
                "eq".to_string(),
                QualType::unqualified(MonoType::Func(
                    vec![
                        MonoType::TVar("a".to_string()),
                        MonoType::TVar("a".to_string()),
                    ],
                    Box::new(MonoType::Bool),
                )),
            );
            m
        },
    };
    env.define_class(eq_class);

    // Type class: Ord (superclass: Eq)
    let ord_class = TypeClass {
        name: "Ord".to_string(),
        params: vec!["a".to_string()],
        supers: vec![Constraint {
            name: "Eq".to_string(),
            types: vec![MonoType::TVar("a".to_string())],
        }],
        members: {
            let mut m = BTreeMap::new();
            m.insert(
                "compare".to_string(),
                QualType::unqualified(MonoType::Func(
                    vec![
                        MonoType::TVar("a".to_string()),
                        MonoType::TVar("a".to_string()),
                    ],
                    Box::new(MonoType::Int),
                )),
            );
            m
        },
    };
    env.define_class(ord_class);

    // Type class: Num
    let num_class = TypeClass {
        name: "Num".to_string(),
        params: vec!["a".to_string()],
        supers: vec![],
        members: {
            let mut m = BTreeMap::new();
            m.insert(
                "add".to_string(),
                QualType::unqualified(MonoType::Func(
                    vec![
                        MonoType::TVar("a".to_string()),
                        MonoType::TVar("a".to_string()),
                    ],
                    Box::new(MonoType::TVar("a".to_string())),
                )),
            );
            m.insert(
                "sub".to_string(),
                QualType::unqualified(MonoType::Func(
                    vec![
                        MonoType::TVar("a".to_string()),
                        MonoType::TVar("a".to_string()),
                    ],
                    Box::new(MonoType::TVar("a".to_string())),
                )),
            );
            m.insert(
                "mul".to_string(),
                QualType::unqualified(MonoType::Func(
                    vec![
                        MonoType::TVar("a".to_string()),
                        MonoType::TVar("a".to_string()),
                    ],
                    Box::new(MonoType::TVar("a".to_string())),
                )),
            );
            m
        },
    };
    env.define_class(num_class);

    env
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_binding() {
        let mut env = TypeEnv::new();
        env.bind_mono("x", MonoType::Int);
        assert!(env.has_binding("x"));
        assert!(!env.has_binding("y"));
    }

    #[test]
    fn test_child_env_inherits() {
        let mut parent = TypeEnv::new();
        parent.bind_mono("x", MonoType::Int);
        let child = TypeEnv::child(parent);
        assert!(child.has_binding("x"));
    }

    #[test]
    fn test_child_env_shadows() {
        let mut parent = TypeEnv::new();
        parent.bind_mono("x", MonoType::Int);
        let mut child = TypeEnv::child(parent);
        child.bind_mono("x", MonoType::Bool);
        let pt = child.lookup("x").unwrap();
        assert_eq!(pt.body.mono, MonoType::Bool);
    }

    #[test]
    fn test_type_alias() {
        let mut env = TypeEnv::new();
        env.alias("MyInt", MonoType::Int);
        assert_eq!(env.unalias("MyInt"), Some(&MonoType::Int));
        assert_eq!(env.unalias("Unknown"), None);
    }

    #[test]
    fn test_generalize() {
        let env = TypeEnv::new();
        let qt = QualType::unqualified(MonoType::Func(
            vec![MonoType::TVar("a".to_string())],
            Box::new(MonoType::TVar("a".to_string())),
        ));
        let pt = env.generalize(&qt);
        assert_eq!(pt.vars, 1);
    }

    #[test]
    fn test_default_env_has_operators() {
        let env = default_type_env();
        assert!(env.has_binding("+"));
        assert!(env.has_binding("=="));
        assert!(env.has_binding("&&"));
    }
}
