//! Integration: a small symbolic table (returns `None` when no rule matches)
//! plus numerical methods (adaptive Simpson) for definite integrals and for
//! generating the antiderivative curve.

use crate::ast::Expr;
use crate::eval::eval;

/// Attempt symbolic antiderivative (indefinite integral). Returns `None` when
/// the expression does not match a supported rule, in which case callers fall
/// back to numerical treatment.
pub fn integrate_symbolic(e: &Expr) -> Option<Expr> {
    Some(match e {
        Expr::Num(_) | Expr::Const(_) => Expr::mul(e.clone(), Expr::var()),

        Expr::Var => Expr::div(Expr::pow(Expr::var(), Expr::num(2.0)), Expr::num(2.0)),

        // x^n  ->  x^(n+1)/(n+1), except n == -1
        Expr::Pow(base, exp) if matches!(**base, Expr::Var) && exp.is_constant() => {
            let n = crate::simplify::simplify(exp);
            let n_val = crate::eval::eval(&n, 0.0);
            if (n_val + 1.0).abs() < 1e-12 {
                Expr::ln(Expr::var())
            } else {
                let np1 = Expr::num(n_val + 1.0);
                Expr::div(Expr::pow(Expr::var(), np1.clone()), np1)
            }
        }

        // 1/x  ->  ln(x)
        Expr::Div(a, b) if matches!(**a, Expr::Num(1.0)) && matches!(**b, Expr::Var) => {
            Expr::ln(Expr::var())
        }

        Expr::Sin(a) if matches!(**a, Expr::Var) => Expr::neg(Expr::cos(Expr::var())),
        Expr::Cos(a) if matches!(**a, Expr::Var) => Expr::sin(Expr::var()),
        Expr::Exp(a) if matches!(**a, Expr::Var) => Expr::exp(Expr::var()),
        Expr::Tan(a) if matches!(**a, Expr::Var) => Expr::neg(Expr::ln(Expr::cos(Expr::var()))),

        // linearity
        Expr::Add(a, b) => Expr::add(integrate_symbolic(a)?, integrate_symbolic(b)?),
        Expr::Sub(a, b) => Expr::sub(integrate_symbolic(a)?, integrate_symbolic(b)?),
        Expr::Neg(a) => Expr::neg(integrate_symbolic(a)?),

        // constant multiple: c * f(x)
        Expr::Mul(a, b) if a.is_constant() => Expr::mul((**a).clone(), integrate_symbolic(b)?),
        Expr::Mul(a, b) if b.is_constant() => Expr::mul((**b).clone(), integrate_symbolic(a)?),

        // c / x  ->  c * ln(x)
        Expr::Div(a, b) if a.is_constant() && matches!(**b, Expr::Var) => {
            Expr::mul((**a).clone(), Expr::ln(Expr::var()))
        }

        _ => return None,
    })
}

/// Adaptive Simpson's rule for ∫[a,b] f(x) dx.
pub fn integrate_numerical(e: &Expr, a: f64, b: f64) -> f64 {
    const EPS: f64 = 1e-7;
    const MAX_DEPTH: u32 = 20;

    fn simpson(_f: &Expr, a: f64, b: f64, fa: f64, fm: f64, fb: f64) -> f64 {
        (b - a) / 6.0 * (fa + 4.0 * fm + fb)
    }

    fn rec(f: &Expr, a: f64, b: f64, fa: f64, fm: f64, fb: f64, whole: f64, depth: u32) -> f64 {
        let m = (a + b) / 2.0;
        let lm = (a + m) / 2.0;
        let rm = (m + b) / 2.0;
        let flm = eval(f, lm);
        let frm = eval(f, rm);

        let left = simpson(f, a, m, fa, flm, fm);
        let right = simpson(f, m, b, fm, frm, fb);
        let delta = left + right - whole;

        if depth >= MAX_DEPTH || delta.abs() <= 15.0 * EPS {
            left + right + delta / 15.0
        } else {
            rec(f, a, m, fa, flm, fm, left, depth + 1)
                + rec(f, m, b, fm, frm, fb, right, depth + 1)
        }
    }

    let fa = eval(e, a);
    let fm = eval(e, (a + b) / 2.0);
    let fb = eval(e, b);
    let whole = simpson(e, a, b, fa, fm, fb);
    rec(e, a, b, fa, fm, fb, whole, 0)
}

/// The antiderivative curve F(x) = ∫[x0, x] f(t) dt, sampled at `xs`, using
/// cumulative adaptive Simpson over consecutive sub-intervals.
pub fn antiderivative_curve(e: &Expr, xs: &[f64]) -> Vec<f64> {
    let mut out = Vec::with_capacity(xs.len());
    let mut acc = 0.0;
    out.push(0.0);
    for w in xs.windows(2) {
        acc += integrate_numerical(e, w[0], w[1]);
        out.push(acc);
    }
    out
}
