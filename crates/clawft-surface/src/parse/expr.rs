//! Hand-rolled recursive-descent parser for the M1.5 binding
//! expression sublanguage (ADR-016 §5, reduced subset).
//!
//! Supported:
//! - Literals: numbers, double-quoted and single-quoted strings, bools.
//! - `$path/seg.field` — ontology path read; a subsequent `.field` is
//!   parsed as a field access on the resolved JSON value.
//! - Function calls: `ident(arg, …)` — the evaluator permits `count`,
//!   `filter`, `len`, `first`, `last`, `fmt_percent` / `fmt_pct`,
//!   `fmt_number`, `fmt_count`, `fmt_duration`, `fmt_bytes`, `exists`.
//! - Field access: `expr.field`.
//! - Binops: `==`, `!=`, `<`, `<=`, `>`, `>=`, `&&`, `||`, `+`, `-`,
//!   `*`, `/` with precedence climbing.
//! - Lambdas for list-combinators: `ident -> expr` (one parameter).
//!
//! Explicitly out of scope for M1.5 (rejected by the parser with a
//! typed error so future milestones can grow it safely):
//! - Ternary `?:` operator.
//! - Nested lambdas (a lambda body that itself contains `->`).
//! - User-defined compositions (`[compositions.*]`).

use std::iter::Peekable;
use std::str::Chars;

use thiserror::Error;

use crate::tree::AttrValue;

/// AST for the binding expression language. Kept small by design —
/// governance has to statically audit it (ADR-016 §5 rationale).
#[derive(Clone, Debug)]
pub enum Expr {
    Literal(AttrValue),
    /// `$substrate/kernel/status` — the raw path string, slash-delimited.
    Path(String),
    /// A lambda-parameter reference. Only legal as the head of a
    /// primary when a lambda is in scope; the evaluator enforces it.
    Var(String),
    /// `expr.field` — access a field on the resolved JSON value.
    Access(Box<Expr>, String),
    /// `fn_name(arg1, arg2, …)`.
    Call(String, Vec<Expr>),
    /// `param -> body`. Only list-combinator use. Captured closures
    /// are evaluated with the lambda's parameter bound to the
    /// current list element.
    Lambda(String, Box<Expr>),
    /// Binary operator application.
    Binop(BinOp, Box<Expr>, Box<Expr>),
    /// Unary minus.
    Neg(Box<Expr>),
    /// Unary not.
    Not(Box<Expr>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinOp {
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    And,
    Or,
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("unexpected end of expression at offset {0}")]
    Eof(usize),
    #[error("unexpected character {found:?} at offset {at}")]
    Unexpected { found: char, at: usize },
    #[error("expected {expected} at offset {at}, found {found:?}")]
    Expected {
        expected: &'static str,
        found: Option<char>,
        at: usize,
    },
    #[error("malformed number literal at offset {0}")]
    BadNumber(usize),
    #[error("unterminated string literal starting at offset {0}")]
    UnterminatedString(usize),
    #[error("ternary `?:` is not supported in M1.5 (ADR-016 subset); at offset {0}")]
    TernaryNotSupported(usize),
    #[error("nested lambda is not supported in M1.5 (ADR-016 subset); at offset {0}")]
    NestedLambda(usize),
}

/// Entry point. Trims the input and parses one complete expression.
pub fn parse(src: &str) -> Result<Expr, ParseError> {
    let mut p = Parser::new(src);
    let e = p.parse_lambda_or_expr(false)?;
    p.skip_ws();
    if p.peek().is_some() {
        return Err(ParseError::Unexpected {
            found: p.peek().unwrap(),
            at: p.offset,
        });
    }
    Ok(e)
}

struct Parser<'a> {
    chars: Peekable<Chars<'a>>,
    offset: usize,
    /// Depth counter for nested lambdas. Incremented when we enter a
    /// lambda body; if a lambda is attempted while already > 0 we
    /// emit `NestedLambda`.
    lambda_depth: u32,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            chars: src.chars().peekable(),
            offset: 0,
            lambda_depth: 0,
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.chars.next();
        if let Some(c) = c {
            self.offset += c.len_utf8();
        }
        c
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.bump();
            } else {
                break;
            }
        }
    }

    /// Try to parse a top-level `ident -> body` lambda; falls back
    /// to a normal expression otherwise. The `_accept_lambda`
    /// parameter is retained for API compatibility but the actual
    /// nesting guard uses [`Parser::lambda_depth`] so multiple
    /// *sibling* lambdas (e.g. as args in `count(…, s -> …)` inside
    /// an outer `filter(…, t -> …)`) are allowed, while genuinely
    /// nested ones (a lambda body that itself contains `->`) are
    /// rejected.
    fn parse_lambda_or_expr(&mut self, _accept_lambda: bool) -> Result<Expr, ParseError> {
        self.skip_ws();
        let start = self.offset;
        if let Some(c) = self.peek()
            && is_ident_start(c)
        {
            let ident = self.parse_ident();
            self.skip_ws();
            if self.peek() == Some('-') {
                self.bump(); // '-'
                if self.peek() == Some('>') {
                    if self.lambda_depth > 0 {
                        return Err(ParseError::NestedLambda(start));
                    }
                    self.bump(); // '>'
                    self.lambda_depth += 1;
                    let body_res = self.parse_expr();
                    self.lambda_depth -= 1;
                    let body = body_res?;
                    return Ok(Expr::Lambda(ident, Box::new(body)));
                }
                // Not an arrow — treat the ident as a primary and
                // the `-` as a subtraction.
                let lhs = self.continue_primary_from_ident(ident)?;
                let rhs = self.parse_binop_rhs(lhs, 0, Some(BinOp::Sub))?;
                return self.finish_top(rhs);
            }
            // Not a lambda.
            let lhs = self.continue_primary_from_ident(ident)?;
            return self.finish_top(lhs);
        }
        self.parse_expr()
    }

    /// After a top-level primary, continue with any trailing binops.
    fn finish_top(&mut self, lhs: Expr) -> Result<Expr, ParseError> {
        let e = self.parse_binop_rhs(lhs, 0, None)?;
        self.skip_ws();
        if self.peek() == Some('?') {
            return Err(ParseError::TernaryNotSupported(self.offset));
        }
        Ok(e)
    }

    /// After we've eaten an ident as a primary seed, finish it into a
    /// call / access / simple primary.
    fn continue_primary_from_ident(&mut self, ident: String) -> Result<Expr, ParseError> {
        self.skip_ws();
        let base = match ident.as_str() {
            "true" => Expr::Literal(AttrValue::Bool(true)),
            "false" => Expr::Literal(AttrValue::Bool(false)),
            _ => {
                if self.peek() == Some('(') {
                    self.bump();
                    let args = self.parse_args()?;
                    Expr::Call(ident, args)
                } else {
                    // A bare ident outside a call is either a
                    // lambda parameter reference (`s` in
                    // `s.status`) or malformed; the evaluator
                    // rejects unbound names. Parse as `Var`.
                    Expr::Var(ident)
                }
            }
        };
        self.parse_accesses(base)
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        let lhs = self.parse_unary()?;
        let e = self.parse_binop_rhs(lhs, 0, None)?;
        self.skip_ws();
        if self.peek() == Some('?') {
            return Err(ParseError::TernaryNotSupported(self.offset));
        }
        Ok(e)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        self.skip_ws();
        match self.peek() {
            Some('-') => {
                self.bump();
                let e = self.parse_unary()?;
                Ok(Expr::Neg(Box::new(e)))
            }
            Some('!') => {
                self.bump();
                let e = self.parse_unary()?;
                Ok(Expr::Not(Box::new(e)))
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        self.skip_ws();
        let c = match self.peek() {
            Some(c) => c,
            None => return Err(ParseError::Eof(self.offset)),
        };
        let base = match c {
            '(' => {
                self.bump();
                let e = self.parse_expr()?;
                self.skip_ws();
                if self.peek() != Some(')') {
                    return Err(ParseError::Expected {
                        expected: "')'",
                        found: self.peek(),
                        at: self.offset,
                    });
                }
                self.bump();
                e
            }
            '$' => {
                self.bump();
                let p = self.parse_path();
                Expr::Path(p)
            }
            '\'' | '"' => {
                let s = self.parse_string_literal(c)?;
                Expr::Literal(AttrValue::Str(s))
            }
            c if c.is_ascii_digit() => {
                let n = self.parse_number()?;
                Expr::Literal(n)
            }
            c if is_ident_start(c) => {
                let ident = self.parse_ident();
                match ident.as_str() {
                    "true" => Expr::Literal(AttrValue::Bool(true)),
                    "false" => Expr::Literal(AttrValue::Bool(false)),
                    _ => {
                        self.skip_ws();
                        if self.peek() == Some('(') {
                            self.bump();
                            let args = self.parse_args()?;
                            Expr::Call(ident, args)
                        } else {
                            // Lambda-parameter reference — the
                            // evaluator will error if no lambda is
                            // currently in scope.
                            Expr::Var(ident)
                        }
                    }
                }
            }
            c => {
                return Err(ParseError::Unexpected {
                    found: c,
                    at: self.offset,
                });
            }
        };
        self.parse_accesses(base)
    }

    /// Eat `.field` field-access chain on top of a primary.
    fn parse_accesses(&mut self, mut base: Expr) -> Result<Expr, ParseError> {
        loop {
            self.skip_ws();
            if self.peek() != Some('.') {
                break;
            }
            // Lookahead: `.` could be a floating-point mid-number
            // but since we've already consumed the primary, any `.`
            // here is an access.
            self.bump();
            self.skip_ws();
            if let Some(c) = self.peek() {
                if !is_ident_start(c) {
                    return Err(ParseError::Expected {
                        expected: "identifier after '.'",
                        found: Some(c),
                        at: self.offset,
                    });
                }
            } else {
                return Err(ParseError::Eof(self.offset));
            }
            let field = self.parse_ident();
            base = Expr::Access(Box::new(base), field);
        }
        Ok(base)
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut args = Vec::new();
        self.skip_ws();
        if self.peek() == Some(')') {
            self.bump();
            return Ok(args);
        }
        loop {
            // The first arg of `count`/`filter` etc. is the list path;
            // the second arg is frequently a lambda. Allow one level
            // of `ident -> body` per argument, but no nesting.
            let arg = self.parse_lambda_or_expr(true)?;
            args.push(arg);
            self.skip_ws();
            match self.peek() {
                Some(',') => {
                    self.bump();
                }
                Some(')') => {
                    self.bump();
                    break;
                }
                _ => {
                    return Err(ParseError::Expected {
                        expected: "',' or ')'",
                        found: self.peek(),
                        at: self.offset,
                    });
                }
            }
        }
        Ok(args)
    }

    fn parse_binop_rhs(
        &mut self,
        mut lhs: Expr,
        min_prec: u8,
        preconsumed: Option<BinOp>,
    ) -> Result<Expr, ParseError> {
        // Optionally use a pre-consumed operator on the first pass
        // (e.g. we've already eaten `-` in lambda lookahead).
        let mut first = preconsumed;
        loop {
            self.skip_ws();
            let op = if let Some(op) = first.take() {
                Some(op)
            } else {
                self.peek_binop()
            };
            let (op, prec) = match op {
                Some(op) => (op, binop_prec(op)),
                None => break,
            };
            if prec < min_prec {
                // Unwind: restore the operator characters we just
                // peeked. But `peek_binop` does not consume, so we
                // only need to break here.
                break;
            }
            if first.is_some() {
                // Shouldn't happen: `first` was just read above, but
                // we guard defensively.
            } else {
                self.consume_binop(op);
            }
            let mut rhs = self.parse_unary()?;
            loop {
                self.skip_ws();
                let next = self.peek_binop();
                let next_prec = match next {
                    Some(op) => binop_prec(op),
                    None => break,
                };
                if next_prec <= prec {
                    break;
                }
                rhs = self.parse_binop_rhs(rhs, prec + 1, None)?;
            }
            lhs = Expr::Binop(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn peek_binop(&mut self) -> Option<BinOp> {
        self.skip_ws();
        let mut it = self.chars.clone();
        let c = it.next()?;
        let c2 = it.next();
        match (c, c2) {
            ('=', Some('=')) => Some(BinOp::Eq),
            ('!', Some('=')) => Some(BinOp::Neq),
            ('<', Some('=')) => Some(BinOp::Lte),
            ('>', Some('=')) => Some(BinOp::Gte),
            ('&', Some('&')) => Some(BinOp::And),
            ('|', Some('|')) => Some(BinOp::Or),
            ('<', _) => Some(BinOp::Lt),
            ('>', _) => Some(BinOp::Gt),
            ('+', _) => Some(BinOp::Add),
            ('-', _) => Some(BinOp::Sub),
            ('*', _) => Some(BinOp::Mul),
            ('/', _) => Some(BinOp::Div),
            _ => None,
        }
    }

    fn consume_binop(&mut self, op: BinOp) {
        match op {
            BinOp::Eq | BinOp::Neq | BinOp::Lte | BinOp::Gte | BinOp::And | BinOp::Or => {
                self.bump();
                self.bump();
            }
            BinOp::Lt | BinOp::Gt | BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                self.bump();
            }
        }
    }

    fn parse_ident(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if is_ident_continue(c) {
                s.push(c);
                self.bump();
            } else {
                break;
            }
        }
        s
    }

    /// An ontology path is a slash-delimited sequence of ident-like
    /// segments after `$`. Segments may contain `-`.
    fn parse_path(&mut self) -> String {
        let mut s = String::new();
        loop {
            while let Some(c) = self.peek() {
                if c.is_alphanumeric() || c == '_' || c == '-' {
                    s.push(c);
                    self.bump();
                } else {
                    break;
                }
            }
            if self.peek() == Some('/') {
                s.push('/');
                self.bump();
            } else {
                break;
            }
        }
        s
    }

    fn parse_string_literal(&mut self, quote: char) -> Result<String, ParseError> {
        let start = self.offset;
        self.bump(); // opening quote
        let mut out = String::new();
        loop {
            match self.peek() {
                None => return Err(ParseError::UnterminatedString(start)),
                Some(c) if c == quote => {
                    self.bump();
                    return Ok(out);
                }
                Some('\\') => {
                    self.bump();
                    match self.bump() {
                        Some('n') => out.push('\n'),
                        Some('t') => out.push('\t'),
                        Some('\\') => out.push('\\'),
                        Some(c) if c == quote => out.push(c),
                        Some(c) => {
                            out.push('\\');
                            out.push(c);
                        }
                        None => return Err(ParseError::UnterminatedString(start)),
                    }
                }
                Some(c) => {
                    out.push(c);
                    self.bump();
                }
            }
        }
    }

    fn parse_number(&mut self) -> Result<AttrValue, ParseError> {
        let start = self.offset;
        let mut s = String::new();
        let mut seen_dot = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                s.push(c);
                self.bump();
            } else if c == '.' && !seen_dot {
                // Peek one past to distinguish `1.5` from `1.foo` (field access on int).
                let mut it = self.chars.clone();
                it.next();
                if let Some(nc) = it.next()
                    && nc.is_ascii_digit()
                {
                    seen_dot = true;
                    s.push(c);
                    self.bump();
                    continue;
                }
                break;
            } else {
                break;
            }
        }
        if seen_dot {
            s.parse::<f64>()
                .map(AttrValue::Number)
                .map_err(|_| ParseError::BadNumber(start))
        } else {
            s.parse::<i64>()
                .map(AttrValue::Int)
                .map_err(|_| ParseError::BadNumber(start))
        }
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn binop_prec(op: BinOp) -> u8 {
    match op {
        BinOp::Or => 1,
        BinOp::And => 2,
        BinOp::Eq | BinOp::Neq => 3,
        BinOp::Lt | BinOp::Lte | BinOp::Gt | BinOp::Gte => 4,
        BinOp::Add | BinOp::Sub => 5,
        BinOp::Mul | BinOp::Div => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_number_literal() {
        let e = parse("42").unwrap();
        matches!(e, Expr::Literal(AttrValue::Int(42)));
    }

    #[test]
    fn parses_path() {
        let e = parse("$substrate/kernel/status").unwrap();
        if let Expr::Path(p) = e {
            assert_eq!(p, "substrate/kernel/status");
        } else {
            panic!("expected Path");
        }
    }

    #[test]
    fn parses_field_access() {
        let e = parse("$substrate/kernel/status.state").unwrap();
        if let Expr::Access(inner, field) = e {
            assert_eq!(field, "state");
            if let Expr::Path(p) = *inner {
                assert_eq!(p, "substrate/kernel/status");
            } else {
                panic!("expected Path inside Access");
            }
        } else {
            panic!("expected Access");
        }
    }

    #[test]
    fn parses_count_filter_lambda() {
        let e = parse("count(filter($services, s -> s.status == \"healthy\"))").unwrap();
        if let Expr::Call(name, args) = e {
            assert_eq!(name, "count");
            assert_eq!(args.len(), 1);
            if let Expr::Call(fname, _) = &args[0] {
                assert_eq!(fname, "filter");
            } else {
                panic!("expected inner filter() call");
            }
        } else {
            panic!("expected outer count() call");
        }
    }

    #[test]
    fn rejects_ternary() {
        let err = parse("1 == 1 ? 2 : 3").unwrap_err();
        assert!(matches!(err, ParseError::TernaryNotSupported(_)));
    }

    #[test]
    fn rejects_nested_lambda() {
        let err = parse("map($xs, x -> map($ys, y -> y))").unwrap_err();
        assert!(matches!(err, ParseError::NestedLambda(_)));
    }

    #[test]
    fn binop_precedence_mul_over_add() {
        let e = parse("1 + 2 * 3").unwrap();
        if let Expr::Binop(BinOp::Add, _, r) = e {
            if let Expr::Binop(BinOp::Mul, _, _) = *r {
                // ok
            } else {
                panic!("expected Mul under Add-right");
            }
        } else {
            panic!("expected Add at top");
        }
    }
}
