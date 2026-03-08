//! Compiler and evaluator for the Calvin language.
//!
//! This module provides a tree-walking interpreter that evaluates Calvin
//! expressions directly. Unlike the original hobbes which used LLVM JIT
//! compilation, Calvin uses an interpreter approach that is portable and
//! safe, while still supporting the full expression language.

pub mod value;

use crate::expr::*;
use crate::types::env::{self, TypeEnv};
use crate::types::infer::TypeError;
use std::collections::BTreeMap;
use thiserror::Error;
use value::Value;

/// Errors that can occur during compilation or evaluation.
#[derive(Debug, Error)]
pub enum EvalError {
    #[error("Type error: {0}")]
    TypeError(#[from] TypeError),

    #[error("Unbound variable: {0}")]
    UnboundVariable(String),

    #[error("Not a function")]
    NotAFunction,

    #[error("Arity mismatch: expected {expected}, got {got}")]
    ArityMismatch { expected: usize, got: usize },

    #[error("Pattern match failure")]
    MatchFailure,

    #[error("Division by zero")]
    DivisionByZero,

    #[error("Index out of bounds: {index} (length {length})")]
    IndexOutOfBounds { index: usize, length: usize },

    #[error("Field not found: {0}")]
    FieldNotFound(String),

    #[error("Runtime error: {0}")]
    Runtime(String),
}

/// The runtime environment mapping variable names to values.
#[derive(Debug, Clone)]
pub struct Env {
    bindings: BTreeMap<String, Value>,
    parent: Option<Box<Env>>,
}

impl Env {
    /// Create a new empty environment.
    pub fn new() -> Self {
        Env {
            bindings: BTreeMap::new(),
            parent: None,
        }
    }

    /// Create a child environment.
    pub fn child(parent: Env) -> Self {
        Env {
            bindings: BTreeMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    /// Bind a variable to a value.
    pub fn bind(&mut self, name: &str, value: Value) {
        self.bindings.insert(name.to_string(), value);
    }

    /// Look up a variable.
    pub fn lookup(&self, name: &str) -> Option<&Value> {
        self.bindings
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.lookup(name)))
    }

    /// Get all bindings (including parent bindings).
    pub fn all_bindings(&self) -> BTreeMap<String, Value> {
        let mut result = BTreeMap::new();
        if let Some(ref parent) = self.parent {
            result.extend(parent.all_bindings());
        }
        result.extend(self.bindings.clone());
        result
    }
}

impl Default for Env {
    fn default() -> Self {
        Self::new()
    }
}

/// The main compiler/evaluator context.
pub struct Compiler {
    /// The type environment for type checking.
    pub type_env: TypeEnv,
    /// The runtime environment for evaluation.
    pub runtime_env: Env,
}

impl Compiler {
    /// Create a new compiler with default built-in definitions.
    pub fn new() -> Self {
        let type_env = env::default_type_env();
        let mut runtime_env = Env::new();

        // Register built-in functions
        Self::register_builtins(&mut runtime_env);

        Compiler {
            type_env,
            runtime_env,
        }
    }

    fn register_builtins(env: &mut Env) {
        // Arithmetic
        env.bind("+", Value::BuiltinFn("+".to_string()));
        env.bind("-", Value::BuiltinFn("-".to_string()));
        env.bind("*", Value::BuiltinFn("*".to_string()));
        env.bind("/", Value::BuiltinFn("/".to_string()));
        env.bind("%", Value::BuiltinFn("%".to_string()));

        // Comparison
        env.bind("==", Value::BuiltinFn("==".to_string()));
        env.bind("!=", Value::BuiltinFn("!=".to_string()));
        env.bind("<", Value::BuiltinFn("<".to_string()));
        env.bind(">", Value::BuiltinFn(">".to_string()));
        env.bind("<=", Value::BuiltinFn("<=".to_string()));
        env.bind(">=", Value::BuiltinFn(">=".to_string()));

        // Boolean
        env.bind("&&", Value::BuiltinFn("&&".to_string()));
        env.bind("||", Value::BuiltinFn("||".to_string()));
        env.bind("!", Value::BuiltinFn("!".to_string()));

        // String
        env.bind("strlen", Value::BuiltinFn("strlen".to_string()));
        env.bind("show", Value::BuiltinFn("show".to_string()));
        env.bind("print", Value::BuiltinFn("print".to_string()));
        env.bind("concat", Value::BuiltinFn("concat".to_string()));

        // Array
        env.bind("length", Value::BuiltinFn("length".to_string()));
        env.bind("head", Value::BuiltinFn("head".to_string()));
        env.bind("tail", Value::BuiltinFn("tail".to_string()));
        env.bind("map", Value::BuiltinFn("map".to_string()));
        env.bind("filter", Value::BuiltinFn("filter".to_string()));
        env.bind("fold", Value::BuiltinFn("fold".to_string()));
    }

    /// Parse and evaluate a string expression.
    pub fn eval_str(&mut self, input: &str) -> Result<Value, EvalError> {
        let expr = crate::parser::parse_expr(input)
            .map_err(|errs| EvalError::Runtime(format!("Parse error: {:?}", errs)))?;
        self.eval(&expr)
    }

    /// Define a variable binding.
    pub fn define(&mut self, name: &str, input: &str) -> Result<(), EvalError> {
        let value = self.eval_str(input)?;
        self.runtime_env.bind(name, value);
        Ok(())
    }

    /// Evaluate a spanned expression.
    pub fn eval(&mut self, expr: &Spanned<Expr>) -> Result<Value, EvalError> {
        self.eval_expr(&expr.node, &self.runtime_env.clone())
    }

    /// Evaluate an expression in a given environment.
    fn eval_expr(&mut self, expr: &Expr, env: &Env) -> Result<Value, EvalError> {
        match expr {
            Expr::Lit(lit) => Ok(Value::from_literal(lit)),

            Expr::Var(name) => env
                .lookup(name)
                .cloned()
                .ok_or_else(|| EvalError::UnboundVariable(name.clone())),

            Expr::Let(name, val, body) => {
                let val = self.eval_expr(&val.node, env)?;
                let mut child_env = Env::child(env.clone());
                child_env.bind(name, val);
                self.eval_expr(&body.node, &child_env)
            }

            Expr::LetRec(bindings, body) => {
                let mut child_env = Env::child(env.clone());
                // First pass: bind all names to Unit (placeholder)
                for (name, _) in bindings {
                    child_env.bind(name, Value::Unit);
                }
                // Second pass: evaluate and rebind
                for (name, expr) in bindings {
                    let val = self.eval_expr(&expr.node, &child_env)?;
                    child_env.bind(name, val);
                }
                self.eval_expr(&body.node, &child_env)
            }

            Expr::App(func, args) => {
                let func_val = self.eval_expr(&func.node, env)?;
                let arg_vals: Result<Vec<Value>, _> = args
                    .iter()
                    .map(|a| self.eval_expr(&a.node, env))
                    .collect();
                let arg_vals = arg_vals?;
                self.apply(func_val, arg_vals, env)
            }

            Expr::Lambda(params, body) => Ok(Value::Closure {
                params: params.clone(),
                body: (**body).clone(),
                env: env.clone(),
            }),

            Expr::If(cond, then_e, else_e) => {
                let cond_val = self.eval_expr(&cond.node, env)?;
                match cond_val {
                    Value::Bool(true) => self.eval_expr(&then_e.node, env),
                    Value::Bool(false) => self.eval_expr(&else_e.node, env),
                    _ => Err(EvalError::Runtime(
                        "Condition must be a boolean".to_string(),
                    )),
                }
            }

            Expr::Match(scrutinee, arms) => {
                let val = self.eval_expr(&scrutinee.node, env)?;
                for arm in arms {
                    if let Some(bindings) = self.match_pattern(&arm.pattern, &val) {
                        let mut child_env = Env::child(env.clone());
                        for (name, bound_val) in bindings {
                            child_env.bind(&name, bound_val);
                        }
                        return self.eval_expr(&arm.body.node, &child_env);
                    }
                }
                Err(EvalError::MatchFailure)
            }

            Expr::MkRecord(fields) => {
                let mut record = BTreeMap::new();
                for (name, expr) in fields {
                    let val = self.eval_expr(&expr.node, env)?;
                    record.insert(name.clone(), val);
                }
                Ok(Value::Record(record))
            }

            Expr::Project(expr, field) => {
                let val = self.eval_expr(&expr.node, env)?;
                match val {
                    Value::Record(fields) => fields
                        .get(field)
                        .cloned()
                        .ok_or_else(|| EvalError::FieldNotFound(field.clone())),
                    _ => Err(EvalError::Runtime("Not a record".to_string())),
                }
            }

            Expr::MkVariant(ctor, expr) => {
                let val = self.eval_expr(&expr.node, env)?;
                Ok(Value::Variant(ctor.clone(), Box::new(val)))
            }

            Expr::MkArray(elems) => {
                let vals: Result<Vec<Value>, _> = elems
                    .iter()
                    .map(|e| self.eval_expr(&e.node, env))
                    .collect();
                Ok(Value::Array(vals?))
            }

            Expr::BinOp(op, lhs, rhs) => {
                let lhs_val = self.eval_expr(&lhs.node, env)?;
                let rhs_val = self.eval_expr(&rhs.node, env)?;
                self.eval_binop(op, lhs_val, rhs_val)
            }

            Expr::UnaryOp(op, expr) => {
                let val = self.eval_expr(&expr.node, env)?;
                self.eval_unaryop(op, val)
            }

            Expr::Annotate(expr, _ty) => {
                // Type annotations are checked at compile time; at runtime, just evaluate
                self.eval_expr(&expr.node, env)
            }

            Expr::Do(exprs) => {
                let mut result = Value::Unit;
                for e in exprs {
                    result = self.eval_expr(&e.node, env)?;
                }
                Ok(result)
            }

            Expr::Comprehension(comp) => self.eval_comprehension(comp, env),

            Expr::Pack(expr, _ty) => self.eval_expr(&expr.node, env),

            Expr::Unpack(_ty_var, val_var, pkg, body) => {
                let val = self.eval_expr(&pkg.node, env)?;
                let mut child_env = Env::child(env.clone());
                child_env.bind(val_var, val);
                self.eval_expr(&body.node, &child_env)
            }
        }
    }

    /// Apply a function value to arguments.
    fn apply(&mut self, func: Value, args: Vec<Value>, _env: &Env) -> Result<Value, EvalError> {
        match func {
            Value::Closure {
                params,
                body,
                env: closure_env,
            } => {
                if params.len() != args.len() {
                    return Err(EvalError::ArityMismatch {
                        expected: params.len(),
                        got: args.len(),
                    });
                }
                let mut child_env = Env::child(closure_env);
                for (param, arg) in params.iter().zip(args.into_iter()) {
                    child_env.bind(param, arg);
                }
                self.eval_expr(&body.node, &child_env)
            }
            Value::BuiltinFn(name) => self.call_builtin(&name, args),
            _ => Err(EvalError::NotAFunction),
        }
    }

    /// Call a built-in function.
    fn call_builtin(&self, name: &str, args: Vec<Value>) -> Result<Value, EvalError> {
        match name {
            "show" => {
                let val = args.into_iter().next().ok_or_else(|| {
                    EvalError::ArityMismatch {
                        expected: 1,
                        got: 0,
                    }
                })?;
                Ok(Value::String(format!("{}", val)))
            }
            "print" => {
                let val = args.into_iter().next().ok_or_else(|| {
                    EvalError::ArityMismatch {
                        expected: 1,
                        got: 0,
                    }
                })?;
                println!("{}", val);
                Ok(Value::Unit)
            }
            "strlen" => {
                let val = args.into_iter().next().ok_or_else(|| {
                    EvalError::ArityMismatch {
                        expected: 1,
                        got: 0,
                    }
                })?;
                match val {
                    Value::String(s) => Ok(Value::Long(s.len() as i64)),
                    _ => Err(EvalError::Runtime("strlen expects a string".to_string())),
                }
            }
            "concat" => {
                if args.len() != 2 {
                    return Err(EvalError::ArityMismatch {
                        expected: 2,
                        got: args.len(),
                    });
                }
                match (&args[0], &args[1]) {
                    (Value::String(a), Value::String(b)) => {
                        Ok(Value::String(format!("{}{}", a, b)))
                    }
                    _ => Err(EvalError::Runtime("concat expects two strings".to_string())),
                }
            }
            "length" => {
                let val = args.into_iter().next().ok_or_else(|| {
                    EvalError::ArityMismatch {
                        expected: 1,
                        got: 0,
                    }
                })?;
                match val {
                    Value::Array(arr) => Ok(Value::Long(arr.len() as i64)),
                    _ => Err(EvalError::Runtime("length expects an array".to_string())),
                }
            }
            "head" => {
                let val = args.into_iter().next().ok_or_else(|| {
                    EvalError::ArityMismatch {
                        expected: 1,
                        got: 0,
                    }
                })?;
                match val {
                    Value::Array(arr) => arr
                        .into_iter()
                        .next()
                        .ok_or_else(|| EvalError::Runtime("head of empty array".to_string())),
                    _ => Err(EvalError::Runtime("head expects an array".to_string())),
                }
            }
            "tail" => {
                let val = args.into_iter().next().ok_or_else(|| {
                    EvalError::ArityMismatch {
                        expected: 1,
                        got: 0,
                    }
                })?;
                match val {
                    Value::Array(arr) => {
                        if arr.is_empty() {
                            Err(EvalError::Runtime("tail of empty array".to_string()))
                        } else {
                            Ok(Value::Array(arr[1..].to_vec()))
                        }
                    }
                    _ => Err(EvalError::Runtime("tail expects an array".to_string())),
                }
            }
            _ => Err(EvalError::Runtime(format!(
                "Unknown built-in function: {}",
                name
            ))),
        }
    }

    /// Evaluate a binary operation.
    fn eval_binop(&self, op: &str, lhs: Value, rhs: Value) -> Result<Value, EvalError> {
        match (op, &lhs, &rhs) {
            // Integer arithmetic
            ("+", Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
            ("-", Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
            ("*", Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
            ("/", Value::Int(a), Value::Int(b)) => {
                if *b == 0 {
                    Err(EvalError::DivisionByZero)
                } else {
                    Ok(Value::Int(a / b))
                }
            }
            ("%", Value::Int(a), Value::Int(b)) => {
                if *b == 0 {
                    Err(EvalError::DivisionByZero)
                } else {
                    Ok(Value::Int(a % b))
                }
            }

            // Long arithmetic
            ("+", Value::Long(a), Value::Long(b)) => Ok(Value::Long(a + b)),
            ("-", Value::Long(a), Value::Long(b)) => Ok(Value::Long(a - b)),
            ("*", Value::Long(a), Value::Long(b)) => Ok(Value::Long(a * b)),
            ("/", Value::Long(a), Value::Long(b)) => {
                if *b == 0 {
                    Err(EvalError::DivisionByZero)
                } else {
                    Ok(Value::Long(a / b))
                }
            }

            // Double arithmetic
            ("+", Value::Double(a), Value::Double(b)) => Ok(Value::Double(a + b)),
            ("-", Value::Double(a), Value::Double(b)) => Ok(Value::Double(a - b)),
            ("*", Value::Double(a), Value::Double(b)) => Ok(Value::Double(a * b)),
            ("/", Value::Double(a), Value::Double(b)) => Ok(Value::Double(a / b)),

            // Float arithmetic
            ("+", Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            ("-", Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
            ("*", Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            ("/", Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),

            // String concatenation
            ("+", Value::String(a), Value::String(b)) => {
                Ok(Value::String(format!("{}{}", a, b)))
            }

            // Integer comparison
            ("==", Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a == b)),
            ("!=", Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a != b)),
            ("<", Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a < b)),
            (">", Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a > b)),
            ("<=", Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a <= b)),
            (">=", Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a >= b)),

            // Long comparison
            ("==", Value::Long(a), Value::Long(b)) => Ok(Value::Bool(a == b)),
            ("!=", Value::Long(a), Value::Long(b)) => Ok(Value::Bool(a != b)),
            ("<", Value::Long(a), Value::Long(b)) => Ok(Value::Bool(a < b)),
            (">", Value::Long(a), Value::Long(b)) => Ok(Value::Bool(a > b)),

            // Double comparison
            ("==", Value::Double(a), Value::Double(b)) => Ok(Value::Bool(a == b)),
            ("!=", Value::Double(a), Value::Double(b)) => Ok(Value::Bool(a != b)),
            ("<", Value::Double(a), Value::Double(b)) => Ok(Value::Bool(a < b)),
            (">", Value::Double(a), Value::Double(b)) => Ok(Value::Bool(a > b)),

            // Boolean comparison
            ("==", Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a == b)),
            ("!=", Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a != b)),

            // String comparison
            ("==", Value::String(a), Value::String(b)) => Ok(Value::Bool(a == b)),
            ("!=", Value::String(a), Value::String(b)) => Ok(Value::Bool(a != b)),

            // Boolean operators
            ("&&", Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(*a && *b)),
            ("||", Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(*a || *b)),

            _ => Err(EvalError::Runtime(format!(
                "Cannot apply operator '{}' to {} and {}",
                op, lhs, rhs
            ))),
        }
    }

    /// Evaluate a unary operation.
    fn eval_unaryop(&self, op: &str, val: Value) -> Result<Value, EvalError> {
        match (op, &val) {
            ("!", Value::Bool(b)) => Ok(Value::Bool(!b)),
            ("-", Value::Int(n)) => Ok(Value::Int(-n)),
            ("-", Value::Long(n)) => Ok(Value::Long(-n)),
            ("-", Value::Double(n)) => Ok(Value::Double(-n)),
            ("-", Value::Float(n)) => Ok(Value::Float(-n)),
            _ => Err(EvalError::Runtime(format!(
                "Cannot apply unary operator '{}' to {}",
                op, val
            ))),
        }
    }

    /// Try to match a pattern against a value, returning bindings if successful.
    fn match_pattern(&self, pattern: &Pattern, value: &Value) -> Option<Vec<(String, Value)>> {
        match (pattern, value) {
            (Pattern::Wildcard, _) => Some(vec![]),

            (Pattern::Var(name), val) => Some(vec![(name.clone(), val.clone())]),

            (Pattern::Lit(Literal::Int(a)), Value::Int(b)) if *a == *b => Some(vec![]),
            (Pattern::Lit(Literal::Long(a)), Value::Long(b)) if *a == *b => Some(vec![]),
            (Pattern::Lit(Literal::Bool(a)), Value::Bool(b)) if *a == *b => Some(vec![]),
            (Pattern::Lit(Literal::String(a)), Value::String(b)) if a == b => Some(vec![]),
            (Pattern::Lit(Literal::Char(a)), Value::Char(b)) if *a == *b => Some(vec![]),
            (Pattern::Lit(Literal::Unit), Value::Unit) => Some(vec![]),

            (Pattern::Record(pats), Value::Record(vals)) => {
                let mut bindings = vec![];
                for (name, pat) in pats {
                    let val = vals.get(name)?;
                    let bs = self.match_pattern(pat, val)?;
                    bindings.extend(bs);
                }
                Some(bindings)
            }

            (Pattern::Variant(pctor, ppat), Value::Variant(vctor, vval)) if pctor == vctor => {
                self.match_pattern(ppat, vval)
            }

            (Pattern::Array(pats), Value::Array(vals)) if pats.len() == vals.len() => {
                let mut bindings = vec![];
                for (pat, val) in pats.iter().zip(vals.iter()) {
                    let bs = self.match_pattern(pat, val)?;
                    bindings.extend(bs);
                }
                Some(bindings)
            }

            (Pattern::Tuple(pats), Value::Array(vals)) if pats.len() == vals.len() => {
                let mut bindings = vec![];
                for (pat, val) in pats.iter().zip(vals.iter()) {
                    let bs = self.match_pattern(pat, val)?;
                    bindings.extend(bs);
                }
                Some(bindings)
            }

            (Pattern::As(inner, name), val) => {
                let mut bindings = self.match_pattern(inner, val)?;
                bindings.push((name.clone(), val.clone()));
                Some(bindings)
            }

            _ => None,
        }
    }

    /// Evaluate a comprehension expression.
    fn eval_comprehension(
        &mut self,
        comp: &ComprehensionExpr,
        env: &Env,
    ) -> Result<Value, EvalError> {
        let mut results = Vec::new();
        self.eval_qualifiers(&comp.qualifiers, 0, &comp.body, env, &mut results)?;
        Ok(Value::Array(results))
    }

    fn eval_qualifiers(
        &mut self,
        qualifiers: &[Qualifier],
        idx: usize,
        body: &Spanned<Expr>,
        env: &Env,
        results: &mut Vec<Value>,
    ) -> Result<(), EvalError> {
        if idx >= qualifiers.len() {
            let val = self.eval_expr(&body.node, env)?;
            results.push(val);
            return Ok(());
        }

        match &qualifiers[idx] {
            Qualifier::Generator(pattern, source) => {
                let source_val = self.eval_expr(&source.node, env)?;
                match source_val {
                    Value::Array(elements) => {
                        for elem in elements {
                            if let Some(bindings) = self.match_pattern(pattern, &elem) {
                                let mut child_env = Env::child(env.clone());
                                for (name, val) in bindings {
                                    child_env.bind(&name, val);
                                }
                                self.eval_qualifiers(
                                    qualifiers,
                                    idx + 1,
                                    body,
                                    &child_env,
                                    results,
                                )?;
                            }
                        }
                    }
                    _ => {
                        return Err(EvalError::Runtime(
                            "Generator source must be an array".to_string(),
                        ))
                    }
                }
            }
            Qualifier::Filter(pred) => {
                let pred_val = self.eval_expr(&pred.node, env)?;
                match pred_val {
                    Value::Bool(true) => {
                        self.eval_qualifiers(qualifiers, idx + 1, body, env, results)?;
                    }
                    Value::Bool(false) => {}
                    _ => {
                        return Err(EvalError::Runtime(
                            "Filter must be a boolean".to_string(),
                        ))
                    }
                }
            }
            Qualifier::Let(name, expr) => {
                let val = self.eval_expr(&expr.node, env)?;
                let mut child_env = Env::child(env.clone());
                child_env.bind(name, val);
                self.eval_qualifiers(qualifiers, idx + 1, body, &child_env, results)?;
            }
        }

        Ok(())
    }

    /// Get the type environment.
    pub fn type_env(&self) -> &TypeEnv {
        &self.type_env
    }

    /// Get the runtime environment.
    pub fn runtime_env(&self) -> &Env {
        &self.runtime_env
    }

    /// List all bound variable names.
    pub fn bound_names(&self) -> Vec<String> {
        self.runtime_env
            .all_bindings()
            .keys()
            .cloned()
            .collect()
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_int_literal() {
        let mut cc = Compiler::new();
        let result = cc.eval_str("42");
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn test_eval_bool_literal() {
        let mut cc = Compiler::new();
        assert_eq!(cc.eval_str("true").unwrap(), Value::Bool(true));
        assert_eq!(cc.eval_str("false").unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_eval_string_literal() {
        let mut cc = Compiler::new();
        assert_eq!(
            cc.eval_str("\"hello\"").unwrap(),
            Value::String("hello".to_string())
        );
    }

    #[test]
    fn test_eval_arithmetic() {
        let mut cc = Compiler::new();
        assert_eq!(cc.eval_str("1 + 2").unwrap(), Value::Int(3));
        assert_eq!(cc.eval_str("10 - 3").unwrap(), Value::Int(7));
        assert_eq!(cc.eval_str("4 * 5").unwrap(), Value::Int(20));
        assert_eq!(cc.eval_str("10 / 3").unwrap(), Value::Int(3));
        assert_eq!(cc.eval_str("10 % 3").unwrap(), Value::Int(1));
    }

    #[test]
    fn test_eval_comparison() {
        let mut cc = Compiler::new();
        assert_eq!(cc.eval_str("1 == 1").unwrap(), Value::Bool(true));
        assert_eq!(cc.eval_str("1 != 2").unwrap(), Value::Bool(true));
        assert_eq!(cc.eval_str("1 < 2").unwrap(), Value::Bool(true));
        assert_eq!(cc.eval_str("2 > 1").unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_eval_let() {
        let mut cc = Compiler::new();
        assert_eq!(cc.eval_str("let x = 42 in x").unwrap(), Value::Int(42));
        assert_eq!(
            cc.eval_str("let x = 1 in let y = 2 in x + y").unwrap(),
            Value::Int(3)
        );
    }

    #[test]
    fn test_eval_if() {
        let mut cc = Compiler::new();
        assert_eq!(
            cc.eval_str("if true then 1 else 2").unwrap(),
            Value::Int(1)
        );
        assert_eq!(
            cc.eval_str("if false then 1 else 2").unwrap(),
            Value::Int(2)
        );
    }

    #[test]
    fn test_eval_lambda() {
        let mut cc = Compiler::new();
        assert_eq!(
            cc.eval_str("(\\x -> x + 1)(41)").unwrap(),
            Value::Int(42)
        );
    }

    #[test]
    fn test_eval_record() {
        let mut cc = Compiler::new();
        let result = cc.eval_str("{x = 1, y = 2}").unwrap();
        if let Value::Record(fields) = result {
            assert_eq!(fields.get("x"), Some(&Value::Int(1)));
            assert_eq!(fields.get("y"), Some(&Value::Int(2)));
        } else {
            panic!("Expected Record");
        }
    }

    #[test]
    fn test_eval_record_projection() {
        let mut cc = Compiler::new();
        assert_eq!(cc.eval_str("{x = 42, y = 0}.x").unwrap(), Value::Int(42));
    }

    #[test]
    fn test_eval_array() {
        let mut cc = Compiler::new();
        let result = cc.eval_str("[1, 2, 3]").unwrap();
        assert_eq!(
            result,
            Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
        );
    }

    #[test]
    fn test_eval_match() {
        let mut cc = Compiler::new();
        assert_eq!(
            cc.eval_str("match 1 { 1 -> true, _ -> false }").unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            cc.eval_str("match 2 { 1 -> true, _ -> false }").unwrap(),
            Value::Bool(false)
        );
    }

    #[test]
    fn test_division_by_zero() {
        let mut cc = Compiler::new();
        assert!(cc.eval_str("1 / 0").is_err());
    }
}
