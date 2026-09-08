//! Rendering an [`Expr`] to a plain ASCII string and to LaTeX.

use crate::ast::{Const, Expr};

/// Format an f64 compactly, trimming float noise and trailing zeros.
pub fn format_number(n: f64) -> String {
    if n.is_nan() {
        return "NaN".into();
    }
    if n.is_infinite() {
        return if n > 0.0 { "inf".into() } else { "-inf".into() };
    }
    if n.fract() == 0.0 && n.abs() < 1e15 {
        return format!("{}", n as i64);
    }
    // Round to kill representation noise (e.g. 0.30000000000000004).
    let r = (n * 1e10).round() / 1e10;
    let mut s = format!("{:.10}", r);
    while s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    s
}

/// Precedence used for parenthesisation decisions (higher binds tighter).
fn prec(e: &Expr) -> u8 {
    match e {
        Expr::Add(_, _) | Expr::Sub(_, _) => 1,
        Expr::Mul(_, _) | Expr::Div(_, _) => 2,
        Expr::Neg(_) => 3,
        Expr::Pow(_, _) => 4,
        _ => 5,
    }
}

fn paren(s: String, cond: bool) -> String {
    if cond {
        format!("({})", s)
    } else {
        s
    }
}

fn latex_paren(s: String, cond: bool) -> String {
    if cond {
        format!("\\left({}\\right)", s)
    } else {
        s
    }
}

/// Plain ASCII infix rendering (fully unambiguous, uses `*` and `^`).
pub fn to_string(e: &Expr) -> String {
    match e {
        Expr::Num(n) => format_number(*n),
        Expr::Const(c) => c.name().to_string(),
        Expr::Var => "x".into(),
        Expr::Add(a, b) => format!("{} + {}", to_string(a), to_string(b)),
        Expr::Sub(a, b) => format!("{} - {}", to_string(a), paren(to_string(b), prec(b) <= 1)),
        Expr::Mul(a, b) => format!(
            "{}*{}",
            paren(to_string(a), prec(a) < 2),
            paren(to_string(b), prec(b) < 2)
        ),
        Expr::Div(a, b) => format!(
            "{}/{}",
            paren(to_string(a), prec(a) < 2),
            paren(to_string(b), prec(b) <= 2)
        ),
        Expr::Pow(a, b) => format!(
            "{}^{}",
            paren(to_string(a), prec(a) < 4),
            paren(to_string(b), prec(b) <= 4)
        ),
        Expr::Neg(a) => format!("-{}", paren(to_string(a), prec(a) <= 3)),
        Expr::Sin(a) => format!("sin({})", to_string(a)),
        Expr::Cos(a) => format!("cos({})", to_string(a)),
        Expr::Tan(a) => format!("tan({})", to_string(a)),
        Expr::Asin(a) => format!("asin({})", to_string(a)),
        Expr::Acos(a) => format!("acos({})", to_string(a)),
        Expr::Atan(a) => format!("atan({})", to_string(a)),
        Expr::Sinh(a) => format!("sinh({})", to_string(a)),
        Expr::Cosh(a) => format!("cosh({})", to_string(a)),
        Expr::Tanh(a) => format!("tanh({})", to_string(a)),
        Expr::Exp(a) => format!("exp({})", to_string(a)),
        Expr::Ln(a) => format!("ln({})", to_string(a)),
        Expr::Log(a) => format!("log({})", to_string(a)),
        Expr::Sqrt(a) => format!("sqrt({})", to_string(a)),
        Expr::Abs(a) => format!("abs({})", to_string(a)),
    }
}

/// Nice LaTeX rendering for KaTeX / MathJax display.
pub fn to_latex(e: &Expr) -> String {
    match e {
        Expr::Num(n) => format_number(*n),
        Expr::Const(Const::Pi) => "\\pi".into(),
        Expr::Const(Const::E) => "e".into(),
        Expr::Var => "x".into(),

        Expr::Add(a, b) => format!("{} + {}", to_latex(a), to_latex(b)),
        Expr::Sub(a, b) => {
            format!("{} - {}", to_latex(a), latex_paren(to_latex(b), prec(b) <= 1))
        }

        Expr::Mul(a, b) => {
            // Coefficient form: "2x", "3\\sin(x)", "-\\pi x^2".
            if let Expr::Num(_) | Expr::Const(_) = &**a {
                let coeff = to_latex(a);
                let factor = to_latex(b);
                let factor = latex_paren(factor, prec(b) <= 1);
                // "1 * x" should have been simplified away; avoid emitting "1x".
                if coeff == "1" {
                    factor
                } else if coeff == "-1" {
                    format!("-{}", factor)
                } else {
                    format!("{}{}", coeff, factor)
                }
            } else {
                format!(
                    "{}\\cdot{}",
                    latex_paren(to_latex(a), prec(a) < 2),
                    latex_paren(to_latex(b), prec(b) < 2)
                )
            }
        }

        Expr::Div(a, b) => format!("\\frac{{{}}}{{{}}}", to_latex(a), to_latex(b)),

        Expr::Pow(a, b) => format!(
            "{}^{{{}}}",
            latex_paren(to_latex(a), prec(a) < 4),
            to_latex(b)
        ),

        Expr::Neg(a) => format!("-{}", latex_paren(to_latex(a), prec(a) <= 3)),

        Expr::Sin(a) => format!("\\sin\\left({}\\right)", to_latex(a)),
        Expr::Cos(a) => format!("\\cos\\left({}\\right)", to_latex(a)),
        Expr::Tan(a) => format!("\\tan\\left({}\\right)", to_latex(a)),
        Expr::Asin(a) => format!("\\arcsin\\left({}\\right)", to_latex(a)),
        Expr::Acos(a) => format!("\\arccos\\left({}\\right)", to_latex(a)),
        Expr::Atan(a) => format!("\\arctan\\left({}\\right)", to_latex(a)),
        Expr::Sinh(a) => format!("\\sinh\\left({}\\right)", to_latex(a)),
        Expr::Cosh(a) => format!("\\cosh\\left({}\\right)", to_latex(a)),
        Expr::Tanh(a) => format!("\\tanh\\left({}\\right)", to_latex(a)),
        Expr::Exp(a) => format!("e^{{{}}}", to_latex(a)),
        Expr::Ln(a) => format!("\\ln\\left({}\\right)", to_latex(a)),
        Expr::Log(a) => format!("\\log\\left({}\\right)", to_latex(a)),
        Expr::Sqrt(a) => format!("\\sqrt{{{}}}", to_latex(a)),
        Expr::Abs(a) => format!("\\left|{}\\right|", to_latex(a)),
    }
}
