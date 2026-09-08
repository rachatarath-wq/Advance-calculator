//! Lexer + recursive-descent parser turning an input string into an [`Expr`].
//!
//! Supported syntax:
//! - binary operators `+ - * / ^` (with `^` right-associative)
//! - unary `-`
//! - implicit multiplication: `2x`, `3sin(x)`, `x(x+1)`
//! - functions: `sin cos tan asin acos atan sinh cosh tanh exp ln log sqrt abs`
//! - constants: `pi`, `e`
//! - variable: `x`
//! - `pow(a, b)` and `log(x, base)` two-argument forms

use crate::ast::{Const, Expr};

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LParen,
    RParen,
    Comma,
}

struct Lexer<'a> {
    chars: Vec<char>,
    pos: usize,
    _src: &'a str,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Lexer {
            chars: src.chars().collect(),
            pos: 0,
            _src: src,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn tokenize(mut self) -> Result<Vec<Tok>, String> {
        let mut out = Vec::new();
        while let Some(c) = self.peek() {
            match c {
                ' ' | '\t' | '\n' | '\r' => {
                    self.bump();
                }
                '+' => {
                    self.bump();
                    out.push(Tok::Plus);
                }
                '-' => {
                    self.bump();
                    out.push(Tok::Minus);
                }
                '*' => {
                    self.bump();
                    out.push(Tok::Star);
                }
                '/' => {
                    self.bump();
                    out.push(Tok::Slash);
                }
                '^' => {
                    self.bump();
                    out.push(Tok::Caret);
                }
                '(' => {
                    self.bump();
                    out.push(Tok::LParen);
                }
                ')' => {
                    self.bump();
                    out.push(Tok::RParen);
                }
                ',' => {
                    self.bump();
                    out.push(Tok::Comma);
                }
                c if c.is_ascii_digit() || c == '.' => {
                    out.push(self.lex_number()?);
                }
                c if c.is_alphabetic() => {
                    out.push(Tok::Ident(self.lex_ident()));
                }
                other => {
                    return Err(format!("unexpected character '{}'", other));
                }
            }
        }
        Ok(out)
    }

    fn lex_number(&mut self) -> Result<Tok, String> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '.' {
                self.bump();
            } else {
                break;
            }
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        text.parse::<f64>()
            .map(Tok::Num)
            .map_err(|_| format!("invalid number '{}'", text))
    }

    fn lex_ident(&mut self) -> String {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() {
                self.bump();
            } else {
                break;
            }
        }
        self.chars[start..self.pos].iter().collect()
    }
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn bump(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn expect_rparen(&mut self) -> Result<(), String> {
        match self.bump() {
            Some(Tok::RParen) => Ok(()),
            _ => Err("expected ')'".into()),
        }
    }

    /// Full expression parse entry point.
    pub fn parse(mut self) -> Result<Expr, String> {
        if self.toks.is_empty() {
            return Err("empty expression".into());
        }
        let e = self.parse_add()?;
        if let Some(t) = self.peek() {
            return Err(format!("unexpected trailing input: {:?}", t));
        }
        Ok(e)
    }

    // additive layer: + -
    fn parse_add(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_mul()?;
        loop {
            match self.peek() {
                Some(Tok::Plus) => {
                    self.bump();
                    let rhs = self.parse_mul()?;
                    lhs = Expr::add(lhs, rhs);
                }
                Some(Tok::Minus) => {
                    self.bump();
                    let rhs = self.parse_mul()?;
                    lhs = Expr::sub(lhs, rhs);
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    // multiplicative layer: * / and implicit multiplication
    fn parse_mul(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_unary()?;
        loop {
            match self.peek() {
                Some(Tok::Star) => {
                    self.bump();
                    let rhs = self.parse_unary()?;
                    lhs = Expr::mul(lhs, rhs);
                }
                Some(Tok::Slash) => {
                    self.bump();
                    let rhs = self.parse_unary()?;
                    lhs = Expr::div(lhs, rhs);
                }
                // implicit multiplication: a primary followed by something that
                // starts a new primary (number, ident, or open paren).
                Some(Tok::Num(_)) | Some(Tok::Ident(_)) | Some(Tok::LParen) => {
                    let rhs = self.parse_unary()?;
                    lhs = Expr::mul(lhs, rhs);
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    // unary layer: - and power (right-associative)
    fn parse_unary(&mut self) -> Result<Expr, String> {
        if let Some(Tok::Minus) = self.peek() {
            self.bump();
            let e = self.parse_unary()?;
            return Ok(Expr::neg(e));
        }
        self.parse_power()
    }

    fn parse_power(&mut self) -> Result<Expr, String> {
        let base = self.parse_primary()?;
        if let Some(Tok::Caret) = self.peek() {
            self.bump();
            let exp = self.parse_unary()?; // right-associative
            return Ok(Expr::pow(base, exp));
        }
        Ok(base)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.bump() {
            Some(Tok::Num(n)) => Ok(Expr::num(n)),
            Some(Tok::Ident(name)) => self.parse_ident(name),
            Some(Tok::LParen) => {
                let e = self.parse_add()?;
                self.expect_rparen()?;
                Ok(e)
            }
            other => Err(format!("unexpected token: {:?}", other)),
        }
    }

    fn parse_ident(&mut self, name: String) -> Result<Expr, String> {
        match name.as_str() {
            "x" => Ok(Expr::var()),
            "pi" => Ok(Expr::Const(Const::Pi)),
            "e" => Ok(Expr::Const(Const::E)),
            // single-argument functions
            "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "sinh" | "cosh" | "tanh"
            | "exp" | "ln" | "sqrt" | "abs" => {
                let arg = self.parse_fn_arg()?;
                Ok(crate::ast::make_unary_fn(&name, arg))
            }
            "log" => {
                // log(x) -> log10(x); log(x, b) -> log_b(x)
                let a = self.parse_fn_arg()?;
                if let Some(Tok::Comma) = self.peek() {
                    self.bump();
                    let b = self.parse_add()?;
                    self.expect_rparen()?;
                    Ok(Expr::div(Expr::ln(a), Expr::ln(b)))
                } else {
                    Ok(Expr::Log(Box::new(a)))
                }
            }
            "pow" => {
                let a = self.parse_fn_arg()?;
                if let Some(Tok::Comma) = self.peek() {
                    self.bump();
                    let b = self.parse_add()?;
                    self.expect_rparen()?;
                    Ok(Expr::pow(a, b))
                } else {
                    Err("pow() requires two arguments: pow(base, exp)".into())
                }
            }
            other => Err(format!("unknown identifier '{}'", other)),
        }
    }

    fn parse_fn_arg(&mut self) -> Result<Expr, String> {
        match self.bump() {
            Some(Tok::LParen) => {
                let e = self.parse_add()?;
                self.expect_rparen()?;
                Ok(e)
            }
            _ => Err("expected '(' after function name".into()),
        }
    }
}

/// Parse a string into an [`Expr`].
pub fn parse(input: &str) -> Result<Expr, String> {
    let toks = Lexer::new(input).tokenize()?;
    Parser { toks, pos: 0 }.parse()
}
