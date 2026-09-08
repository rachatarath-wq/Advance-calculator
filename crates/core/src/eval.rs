//! Evaluation of an [`Expr`] at a concrete `x` value (f64, NaN-propagating).

use crate::ast::{Const, Expr};

pub fn eval(e: &Expr, x: f64) -> f64 {
    match e {
        Expr::Num(n) => *n,
        Expr::Const(c) => match c {
            Const::Pi => std::f64::consts::PI,
            Const::E => std::f64::consts::E,
        },
        Expr::Var => x,
        Expr::Add(a, b) => eval(a, x) + eval(b, x),
        Expr::Sub(a, b) => eval(a, x) - eval(b, x),
        Expr::Mul(a, b) => eval(a, x) * eval(b, x),
        Expr::Div(a, b) => eval(a, x) / eval(b, x),
        Expr::Pow(a, b) => eval(a, x).powf(eval(b, x)),
        Expr::Neg(a) => -eval(a, x),
        Expr::Sin(a) => eval(a, x).sin(),
        Expr::Cos(a) => eval(a, x).cos(),
        Expr::Tan(a) => eval(a, x).tan(),
        Expr::Asin(a) => eval(a, x).asin(),
        Expr::Acos(a) => eval(a, x).acos(),
        Expr::Atan(a) => eval(a, x).atan(),
        Expr::Sinh(a) => eval(a, x).sinh(),
        Expr::Cosh(a) => eval(a, x).cosh(),
        Expr::Tanh(a) => eval(a, x).tanh(),
        Expr::Exp(a) => eval(a, x).exp(),
        Expr::Ln(a) => eval(a, x).ln(),
        Expr::Log(a) => eval(a, x).log10(),
        Expr::Sqrt(a) => eval(a, x).sqrt(),
        Expr::Abs(a) => eval(a, x).abs(),
    }
}
