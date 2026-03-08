//! Parser for the Calvin language.
//!
//! This module implements a parser for Calvin expressions using the chumsky
//! parser combinator library. The grammar is inspired by Haskell with some
//! ML-family influences.

pub mod lexer;

use crate::expr::*;
use crate::types::MonoType;
use chumsky::prelude::*;
use std::collections::BTreeMap;

/// Parse a complete expression from a string.
pub fn parse_expr(input: &str) -> Result<Spanned<Expr>, Vec<Simple<char>>> {
    expr_parser().parse(input)
}

/// Parse a complete module from a string.
pub fn parse_module(name: &str, input: &str) -> Result<Module, Vec<Simple<char>>> {
    module_parser(name).parse(input)
}

/// Parse a type annotation from a string.
pub fn parse_type(input: &str) -> Result<MonoType, Vec<Simple<char>>> {
    type_parser().parse(input)
}



fn ident() -> impl Parser<char, String, Error = Simple<char>> + Clone {
    let keywords = [
        "let", "in", "if", "then", "else", "match", "with", "fn", "true", "false", "do",
        "type", "class", "instance", "data", "import", "where", "forall", "exists", "mu",
        "letrec", "pack", "unpack",
    ];

    filter(|c: &char| c.is_alphabetic() || *c == '_')
        .then(filter(|c: &char| c.is_alphanumeric() || *c == '_' || *c == '\'').repeated())
        .map(|(first, rest)| {
            let mut s = String::new();
            s.push(first);
            s.extend(rest);
            s
        })
        .try_map(move |s, span| {
            if keywords.contains(&s.as_str()) {
                Err(Simple::custom(span, format!("'{}' is a keyword", s)))
            } else {
                Ok(s)
            }
        })
}

fn upper_ident() -> impl Parser<char, String, Error = Simple<char>> + Clone {
    filter(|c: &char| c.is_uppercase())
        .then(filter(|c: &char| c.is_alphanumeric() || *c == '_' || *c == '\'').repeated())
        .map(|(first, rest)| {
            let mut s = String::new();
            s.push(first);
            s.extend(rest);
            s
        })
}

fn int_literal() -> impl Parser<char, Literal, Error = Simple<char>> + Clone {
    let sign = just('-').or_not().map(|s| s.is_some());
    sign.then(
        filter(|c: &char| c.is_ascii_digit())
            .repeated()
            .at_least(1)
            .collect::<String>(),
    )
    .then(
        just('L')
            .or(just('s'))
            .or(just('b'))
            .or(just('q'))
            .or_not(),
    )
    .try_map(|((neg, digits), suffix), span| {
        let val_str = if neg {
            format!("-{}", digits)
        } else {
            digits
        };
        match suffix {
            Some('L') => val_str
                .parse::<i64>()
                .map(Literal::Long)
                .map_err(|e| Simple::custom(span, e.to_string())),
            Some('s') => val_str
                .parse::<i16>()
                .map(Literal::Short)
                .map_err(|e| Simple::custom(span, e.to_string())),
            Some('b') => val_str
                .parse::<u8>()
                .map(Literal::Byte)
                .map_err(|e| Simple::custom(span, e.to_string())),
            Some('q') => val_str
                .parse::<i128>()
                .map(Literal::Int128)
                .map_err(|e| Simple::custom(span, e.to_string())),
            _ => val_str
                .parse::<i32>()
                .map(Literal::Int)
                .map_err(|e| Simple::custom(span, e.to_string())),
        }
    })
}

fn float_literal() -> impl Parser<char, Literal, Error = Simple<char>> + Clone {
    let sign = just('-').or_not().map(|s| s.is_some());
    sign.then(
        filter(|c: &char| c.is_ascii_digit())
            .repeated()
            .at_least(1)
            .collect::<String>(),
    )
    .then_ignore(just('.'))
    .then(
        filter(|c: &char| c.is_ascii_digit())
            .repeated()
            .collect::<String>(),
    )
    .then(just('f').or_not())
    .try_map(|(((neg, int_part), frac_part), suffix), span| {
        let val_str = if neg {
            format!("-{}.{}", int_part, frac_part)
        } else {
            format!("{}.{}", int_part, frac_part)
        };
        match suffix {
            Some('f') => val_str
                .parse::<f32>()
                .map(Literal::Float)
                .map_err(|e| Simple::custom(span, e.to_string())),
            _ => val_str
                .parse::<f64>()
                .map(Literal::Double)
                .map_err(|e| Simple::custom(span, e.to_string())),
        }
    })
}

fn string_literal() -> impl Parser<char, Literal, Error = Simple<char>> + Clone {
    just('"')
        .ignore_then(
            filter(|c: &char| *c != '"' && *c != '\\')
                .or(just('\\').ignore_then(
                    just('n')
                        .to('\n')
                        .or(just('t').to('\t'))
                        .or(just('\\').to('\\'))
                        .or(just('"').to('"'))
                        .or(just('0').to('\0')),
                ))
                .repeated()
                .collect::<String>(),
        )
        .then_ignore(just('"'))
        .map(Literal::String)
}

fn char_literal() -> impl Parser<char, Literal, Error = Simple<char>> + Clone {
    just('\'')
        .ignore_then(
            filter(|c: &char| *c != '\'' && *c != '\\').or(just('\\').ignore_then(
                just('n')
                    .to('\n')
                    .or(just('t').to('\t'))
                    .or(just('\\').to('\\'))
                    .or(just('\'').to('\'')),
            )),
        )
        .then_ignore(just('\''))
        .map(Literal::Char)
}

fn literal() -> impl Parser<char, Literal, Error = Simple<char>> + Clone {
    choice((
        just("true").to(Literal::Bool(true)),
        just("false").to(Literal::Bool(false)),
        just("()").to(Literal::Unit),
        char_literal(),
        string_literal(),
        float_literal(),
        int_literal(),
    ))
}

fn type_parser() -> impl Parser<char, MonoType, Error = Simple<char>> + Clone {
    recursive(|ty| {
        let prim_type = choice((
            just("()").to(MonoType::Unit),
            just("bool").to(MonoType::Bool),
            just("char").to(MonoType::Char),
            just("byte").to(MonoType::Byte),
            just("short").to(MonoType::Short),
            just("int128").to(MonoType::Int128),
            just("int").to(MonoType::Int),
            just("long").to(MonoType::Long),
            just("float").to(MonoType::Float),
            just("double").to(MonoType::Double),
            just("str").to(MonoType::Str),
        ));

        let tvar = ident().map(MonoType::TVar);

        let record_type = just('{')
            .padded()
            .ignore_then(
                ident()
                    .padded()
                    .then_ignore(just(':').padded())
                    .then(ty.clone())
                    .separated_by(just(',').padded())
                    .allow_trailing(),
            )
            .then_ignore(just('}').padded())
            .map(|fields| {
                MonoType::Record(fields.into_iter().collect::<BTreeMap<_, _>>())
            });

        let array_type = just('[')
            .padded()
            .ignore_then(ty.clone())
            .then(
                just(';')
                    .padded()
                    .ignore_then(
                        filter(|c: &char| c.is_ascii_digit())
                            .repeated()
                            .at_least(1)
                            .collect::<String>()
                            .try_map(|s, span| {
                                s.parse::<usize>()
                                    .map_err(|e| Simple::custom(span, e.to_string()))
                            }),
                    )
                    .or_not(),
            )
            .then_ignore(just(']').padded())
            .map(|(elem, size)| match size {
                Some(n) => MonoType::FixedArray(Box::new(elem), n),
                None => MonoType::Array(Box::new(elem)),
            });

        let paren_type = just('(')
            .padded()
            .ignore_then(ty.clone())
            .then_ignore(just(')').padded());

        let atom_type = choice((prim_type, record_type, array_type, paren_type, tvar));

        // Function type: a -> b
        atom_type
            .clone()
            .padded()
            .then(
                just('-')
                    .then(just('>'))
                    .padded()
                    .ignore_then(ty.clone())
                    .or_not(),
            )
            .map(|(param, ret)| match ret {
                Some(ret_ty) => MonoType::Func(vec![param], Box::new(ret_ty)),
                None => param,
            })
    })
}

fn pattern_parser() -> impl Parser<char, Pattern, Error = Simple<char>> + Clone {
    recursive(|pat| {
        let wildcard = just('_').to(Pattern::Wildcard);
        let var_pat = ident().map(Pattern::Var);
        let lit_pat = literal().map(Pattern::Lit);

        let record_pat = just('{')
            .padded()
            .ignore_then(
                ident()
                    .padded()
                    .then_ignore(just('=').padded())
                    .then(pat.clone())
                    .separated_by(just(',').padded())
                    .allow_trailing(),
            )
            .then_ignore(just('}').padded())
            .map(|fields| Pattern::Record(fields.into_iter().collect::<BTreeMap<_, _>>()));

        let variant_pat = just('|')
            .padded()
            .ignore_then(upper_ident().padded())
            .then(pat.clone().or_not())
            .then_ignore(just('|').padded())
            .map(|(name, inner)| {
                Pattern::Variant(name, Box::new(inner.unwrap_or(Pattern::Wildcard)))
            });

        let array_pat = just('[')
            .padded()
            .ignore_then(
                pat.clone()
                    .separated_by(just(',').padded())
                    .allow_trailing(),
            )
            .then_ignore(just(']').padded())
            .map(Pattern::Array);

        let paren_pat = just('(')
            .padded()
            .ignore_then(
                pat.clone()
                    .separated_by(just(',').padded())
                    .allow_trailing(),
            )
            .then_ignore(just(')').padded())
            .map(|pats| {
                if pats.len() == 1 {
                    pats.into_iter().next().unwrap()
                } else {
                    Pattern::Tuple(pats)
                }
            });

        let base_pat = choice((
            wildcard,
            lit_pat,
            variant_pat,
            record_pat,
            array_pat,
            paren_pat,
            var_pat,
        ));

        // Support `pat @ name` syntax
        base_pat
            .then(
                just('@')
                    .padded()
                    .ignore_then(ident())
                    .or_not(),
            )
            .map(|(pat, alias)| match alias {
                Some(name) => Pattern::As(Box::new(pat), name),
                None => pat,
            })
    })
}

fn expr_parser() -> impl Parser<char, Spanned<Expr>, Error = Simple<char>> {
    recursive(|expr| {
        let lit_expr = literal()
            .map_with_span(|lit, span: std::ops::Range<usize>| {
                Spanned::new(Expr::Lit(lit), Span::new(span.start, span.end))
            });

        let var_expr = ident()
            .map_with_span(|name, span: std::ops::Range<usize>| {
                Spanned::new(Expr::Var(name), Span::new(span.start, span.end))
            });

        let paren_expr = just('(')
            .padded()
            .ignore_then(expr.clone())
            .then_ignore(just(')').padded());

        let record_expr = just('{')
            .padded()
            .ignore_then(
                ident()
                    .padded()
                    .then_ignore(just('=').padded())
                    .then(expr.clone())
                    .separated_by(just(',').padded())
                    .allow_trailing(),
            )
            .then_ignore(just('}').padded())
            .map_with_span(|fields, span: std::ops::Range<usize>| {
                Spanned::new(
                    Expr::MkRecord(fields.into_iter().collect::<BTreeMap<_, _>>()),
                    Span::new(span.start, span.end),
                )
            });

        let array_expr = just('[')
            .padded()
            .ignore_then(
                expr.clone()
                    .separated_by(just(',').padded())
                    .allow_trailing(),
            )
            .then_ignore(just(']').padded())
            .map_with_span(|elems, span: std::ops::Range<usize>| {
                Spanned::new(Expr::MkArray(elems), Span::new(span.start, span.end))
            });

        let lambda_expr = just('\\')
            .padded()
            .ignore_then(ident().padded().repeated().at_least(1))
            .then_ignore(just('-').then(just('>')).padded())
            .then(expr.clone())
            .map_with_span(|(params, body), span: std::ops::Range<usize>| {
                Spanned::new(
                    Expr::Lambda(params, Box::new(body)),
                    Span::new(span.start, span.end),
                )
            });

        let let_expr = just("let")
            .padded()
            .ignore_then(ident().padded())
            .then_ignore(just('=').padded())
            .then(expr.clone())
            .then_ignore(just("in").padded())
            .then(expr.clone())
            .map_with_span(|((name, val), body), span: std::ops::Range<usize>| {
                Spanned::new(
                    Expr::Let(name, Box::new(val), Box::new(body)),
                    Span::new(span.start, span.end),
                )
            });

        let if_expr = just("if")
            .padded()
            .ignore_then(expr.clone())
            .then_ignore(just("then").padded())
            .then(expr.clone())
            .then_ignore(just("else").padded())
            .then(expr.clone())
            .map_with_span(|((cond, then_e), else_e), span: std::ops::Range<usize>| {
                Spanned::new(
                    Expr::If(Box::new(cond), Box::new(then_e), Box::new(else_e)),
                    Span::new(span.start, span.end),
                )
            });

        let match_arm = pattern_parser()
            .padded()
            .then_ignore(just('-').then(just('>')).padded())
            .then(expr.clone())
            .map(|(pattern, body)| MatchArm {
                pattern,
                guard: None,
                body,
            });

        let match_expr = just("match")
            .padded()
            .ignore_then(expr.clone())
            .then_ignore(just('{').padded())
            .then(
                match_arm
                    .separated_by(just(',').padded())
                    .allow_trailing(),
            )
            .then_ignore(just('}').padded())
            .map_with_span(|(scrutinee, arms), span: std::ops::Range<usize>| {
                Spanned::new(
                    Expr::Match(Box::new(scrutinee), arms),
                    Span::new(span.start, span.end),
                )
            });

        let variant_expr = just('|')
            .padded()
            .ignore_then(upper_ident().padded())
            .then(expr.clone().or_not())
            .then_ignore(just('|').padded())
            .map_with_span(|(name, inner), span: std::ops::Range<usize>| {
                let inner = inner.unwrap_or_else(|| Spanned::dummy(Expr::Lit(Literal::Unit)));
                Spanned::new(
                    Expr::MkVariant(name, Box::new(inner)),
                    Span::new(span.start, span.end),
                )
            });

        let atom = choice((
            let_expr,
            if_expr,
            match_expr,
            lambda_expr,
            variant_expr,
            record_expr,
            array_expr,
            lit_expr,
            paren_expr,
            var_expr,
        ))
        .padded();

        // Field projection: expr.field
        let projection = atom
            .then(
                just('.')
                    .ignore_then(ident())
                    .repeated(),
            )
            .foldl(|expr, field| {
                let span = expr.span.clone();
                Spanned::new(Expr::Project(Box::new(expr), field), span)
            });

        // Function application: f(args) or f arg
        let app = projection
            .clone()
            .then(
                just('(')
                    .padded()
                    .ignore_then(
                        expr.clone()
                            .separated_by(just(',').padded())
                            .allow_trailing(),
                    )
                    .then_ignore(just(')').padded())
                    .repeated(),
            )
            .foldl(|func, args| {
                let span = func.span.clone();
                if args.is_empty() {
                    func
                } else {
                    Spanned::new(Expr::App(Box::new(func), args), span)
                }
            });

        // Unary operators
        let unary = just('!')
            .or(just('-'))
            .map(|c| c.to_string())
            .padded()
            .repeated()
            .then(app)
            .foldr(|op, expr| {
                let span = expr.span.clone();
                Spanned::new(Expr::UnaryOp(op, Box::new(expr)), span)
            });

        // Multiplicative operators
        let mul_op = just('*')
            .or(just('/'))
            .or(just('%'))
            .map(|c| c.to_string());

        let mul = unary
            .clone()
            .then(
                mul_op
                    .padded()
                    .then(unary)
                    .repeated(),
            )
            .foldl(|lhs, (op, rhs)| {
                let span = lhs.span.clone();
                Spanned::new(
                    Expr::BinOp(op, Box::new(lhs), Box::new(rhs)),
                    span,
                )
            });

        // Additive operators
        let add_op = just('+').or(just('-')).map(|c| c.to_string());

        let add = mul
            .clone()
            .then(
                add_op
                    .padded()
                    .then(mul)
                    .repeated(),
            )
            .foldl(|lhs, (op, rhs)| {
                let span = lhs.span.clone();
                Spanned::new(
                    Expr::BinOp(op, Box::new(lhs), Box::new(rhs)),
                    span,
                )
            });

        // Comparison operators
        let cmp_op = choice((
            just("==").to("==".to_string()),
            just("!=").to("!=".to_string()),
            just("<=").to("<=".to_string()),
            just(">=").to(">=".to_string()),
            just('<').to("<".to_string()),
            just('>').to(">".to_string()),
        ));

        let cmp = add
            .clone()
            .then(
                cmp_op
                    .padded()
                    .then(add)
                    .or_not(),
            )
            .map(|(lhs, rhs)| match rhs {
                Some((op, rhs)) => {
                    let span = lhs.span.clone();
                    Spanned::new(Expr::BinOp(op, Box::new(lhs), Box::new(rhs)), span)
                }
                None => lhs,
            });

        // Logical AND
        let and = cmp
            .clone()
            .then(
                just("&&")
                    .to("&&".to_string())
                    .padded()
                    .then(cmp)
                    .repeated(),
            )
            .foldl(|lhs, (op, rhs)| {
                let span = lhs.span.clone();
                Spanned::new(
                    Expr::BinOp(op, Box::new(lhs), Box::new(rhs)),
                    span,
                )
            });

        // Logical OR
        let or = and
            .clone()
            .then(
                just("||")
                    .to("||".to_string())
                    .padded()
                    .then(and)
                    .repeated(),
            )
            .foldl(|lhs, (op, rhs)| {
                let span = lhs.span.clone();
                Spanned::new(
                    Expr::BinOp(op, Box::new(lhs), Box::new(rhs)),
                    span,
                )
            });

        // Type annotation: expr :: type
        or.then(
            just(':')
                .then(just(':'))
                .padded()
                .ignore_then(type_parser())
                .or_not(),
        )
        .map(|(expr, ty)| match ty {
            Some(ty) => {
                let span = expr.span.clone();
                Spanned::new(Expr::Annotate(Box::new(expr), ty), span)
            }
            None => expr,
        })
    })
}

fn definition_parser() -> impl Parser<char, Definition, Error = Simple<char>> {
    let value_def = just("let")
        .padded()
        .ignore_then(ident().padded())
        .then(ident().padded().repeated())
        .then_ignore(just('=').padded())
        .then(expr_parser())
        .map(|((name, params), body)| {
            if params.is_empty() {
                Definition::Value(name, body)
            } else {
                Definition::Function(name, params, body)
            }
        });

    let type_alias = just("type")
        .padded()
        .ignore_then(upper_ident().padded())
        .then_ignore(just('=').padded())
        .then(type_parser())
        .map(|(name, ty)| Definition::TypeAlias(name, ty));

    let import_def = just("import")
        .padded()
        .ignore_then(
            filter(|c: &char| c.is_alphanumeric() || *c == '.' || *c == '_')
                .repeated()
                .at_least(1)
                .collect::<String>(),
        )
        .map(Definition::Import);

    choice((value_def, type_alias, import_def))
}

fn module_parser(name: &str) -> impl Parser<char, Module, Error = Simple<char>> + '_ {
    let name = name.to_string();
    definition_parser()
        .padded()
        .repeated()
        .then_ignore(end())
        .map(move |defs| {
            let mut module = Module::new(&name);
            for def in defs {
                module.add_definition(def);
            }
            module
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_int_literal() {
        let result = parse_expr("42");
        assert!(result.is_ok());
        if let Expr::Lit(Literal::Int(42)) = &result.unwrap().node {
        } else {
            panic!("Expected Int(42)");
        }
    }

    #[test]
    fn test_parse_bool_literal() {
        let result = parse_expr("true");
        assert!(result.is_ok());
        if let Expr::Lit(Literal::Bool(true)) = &result.unwrap().node {
        } else {
            panic!("Expected Bool(true)");
        }
    }

    #[test]
    fn test_parse_string_literal() {
        let result = parse_expr("\"hello\"");
        assert!(result.is_ok());
        if let Expr::Lit(Literal::String(s)) = &result.unwrap().node {
            assert_eq!(s, "hello");
        } else {
            panic!("Expected String");
        }
    }

    #[test]
    fn test_parse_variable() {
        let result = parse_expr("x");
        assert!(result.is_ok());
        if let Expr::Var(name) = &result.unwrap().node {
            assert_eq!(name, "x");
        } else {
            panic!("Expected Var");
        }
    }

    #[test]
    fn test_parse_binary_op() {
        let result = parse_expr("1 + 2");
        assert!(result.is_ok());
        if let Expr::BinOp(op, _, _) = &result.unwrap().node {
            assert_eq!(op, "+");
        } else {
            panic!("Expected BinOp");
        }
    }

    #[test]
    fn test_parse_lambda() {
        let result = parse_expr("\\x -> x");
        assert!(result.is_ok());
        if let Expr::Lambda(params, _) = &result.unwrap().node {
            assert_eq!(params, &["x"]);
        } else {
            panic!("Expected Lambda");
        }
    }

    #[test]
    fn test_parse_let() {
        let result = parse_expr("let x = 42 in x");
        assert!(result.is_ok());
        if let Expr::Let(name, _, _) = &result.unwrap().node {
            assert_eq!(name, "x");
        } else {
            panic!("Expected Let");
        }
    }

    #[test]
    fn test_parse_if() {
        let result = parse_expr("if true then 1 else 2");
        assert!(result.is_ok());
        if let Expr::If(_, _, _) = &result.unwrap().node {
        } else {
            panic!("Expected If");
        }
    }

    #[test]
    fn test_parse_record() {
        let result = parse_expr("{x = 1, y = 2}");
        assert!(result.is_ok());
        if let Expr::MkRecord(fields) = &result.unwrap().node {
            assert!(fields.contains_key("x"));
            assert!(fields.contains_key("y"));
        } else {
            panic!("Expected MkRecord");
        }
    }

    #[test]
    fn test_parse_array() {
        let result = parse_expr("[1, 2, 3]");
        assert!(result.is_ok());
        if let Expr::MkArray(elems) = &result.unwrap().node {
            assert_eq!(elems.len(), 3);
        } else {
            panic!("Expected MkArray");
        }
    }

    #[test]
    fn test_parse_type_int() {
        let result = parse_type("int");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), MonoType::Int);
    }

    #[test]
    fn test_parse_type_func() {
        let result = parse_type("int -> bool");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            MonoType::Func(vec![MonoType::Int], Box::new(MonoType::Bool))
        );
    }
}
