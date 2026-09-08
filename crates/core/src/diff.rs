//! Symbolic differentiation.

use crate::ast::{Const, Expr};

/// Differentiate `e` with respect to `x`. The result is not simplified here;
/// call [`crate::simplify::simplify`] on it for a readable result.
pub fn diff(e: &Expr) -> Expr {
    match e {
        Expr::Num(_) | Expr::Const(_) => Expr::num(0.0),
        Expr::Var => Expr::num(1.0),

        Expr::Add(a, b) => Expr::add(diff(a), diff(b)),
        Expr::Sub(a, b) => Expr::sub(diff(a), diff(b)),

        Expr::Mul(a, b) => {
            // product rule: a'*b + a*b'
            Expr::add(Expr::mul(diff(a), (**b).clone()), Expr::mul((**a).clone(), diff(b)))
        }
        Expr::Div(a, b) => {
            // quotient rule: (a'*b - a*b') / b^2
            let num = Expr::sub(
                Expr::mul(diff(a), (**b).clone()),
                Expr::mul((**a).clone(), diff(b)),
            );
            let den = Expr::pow((**b).clone(), Expr::num(2.0));
            Expr::div(num, den)
        }

        Expr::Pow(base, exp) => {
            // d/dx e^g(x) = e^g(x) * g'(x)  (nicer than the generic log form)
            if matches!(**base, Expr::Const(Const::E)) {
                return Expr::mul(Expr::pow((**base).clone(), (**exp).clone()), diff(exp));
            }
            if exp.is_constant() {
                // power rule: n * base^(n-1) * base'
                let n = (**exp).clone();
                let nm1 = Expr::sub(n.clone(), Expr::num(1.0));
                Expr::mul(
                    Expr::mul(n, Expr::pow((**base).clone(), nm1)),
                    diff(base),
                )
            } else if base.is_constant() {
                // a^g(x)  ->  a^g(x) * ln(a) * g'
                Expr::mul(
                    Expr::mul(
                        Expr::pow((**base).clone(), (**exp).clone()),
                        Expr::ln((**base).clone()),
                    ),
                    diff(exp),
                )
            } else {
                // f(x)^g(x) via exponential rewriting: f^g * (g' ln f + g f'/f)
                let fg = Expr::pow((**base).clone(), (**exp).clone());
                Expr::mul(
                    fg,
                    Expr::add(
                        Expr::mul(diff(exp), Expr::ln((**base).clone())),
                        Expr::mul(
                            (**exp).clone(),
                            Expr::div(diff(base), (**base).clone()),
                        ),
                    ),
                )
            }
        }

        Expr::Neg(a) => Expr::neg(diff(a)),

        // d/dx sin(u) = cos(u) * u'
        Expr::Sin(a) => Expr::mul(Expr::cos((**a).clone()), diff(a)),
        Expr::Cos(a) => Expr::neg(Expr::mul(Expr::sin((**a).clone()), diff(a))),
        Expr::Tan(a) => {
            // sec^2(u) * u'  =  u' / cos(u)^2
            let cosu = Expr::cos((**a).clone());
            Expr::div(diff(a), Expr::pow(cosu, Expr::num(2.0)))
        }
        Expr::Asin(a) => {
            // u' / sqrt(1 - u^2)
            let one = Expr::num(1.0);
            let denom = Expr::sqrt(Expr::sub(one, Expr::pow((**a).clone(), Expr::num(2.0))));
            Expr::div(diff(a), denom)
        }
        Expr::Acos(a) => {
            // -u' / sqrt(1 - u^2)
            let one = Expr::num(1.0);
            let denom = Expr::sqrt(Expr::sub(one, Expr::pow((**a).clone(), Expr::num(2.0))));
            Expr::neg(Expr::div(diff(a), denom))
        }
        Expr::Atan(a) => {
            // u' / (1 + u^2)
            let denom = Expr::add(Expr::num(1.0), Expr::pow((**a).clone(), Expr::num(2.0)));
            Expr::div(diff(a), denom)
        }
        Expr::Sinh(a) => Expr::mul(Expr::cosh((**a).clone()), diff(a)),
        Expr::Cosh(a) => Expr::mul(Expr::sinh((**a).clone()), diff(a)),
        Expr::Tanh(a) => {
            // sech^2(u) * u' = u' / cosh(u)^2
            let coshu = Expr::cosh((**a).clone());
            Expr::div(diff(a), Expr::pow(coshu, Expr::num(2.0)))
        }

        Expr::Exp(a) => Expr::mul(Expr::exp((**a).clone()), diff(a)),
        Expr::Ln(a) => Expr::div(diff(a), (**a).clone()),
        Expr::Log(a) => {
            // d/dx log10(u) = u' / (u * ln(10))
            let ln10 = Expr::ln(Expr::num(10.0));
            Expr::div(diff(a), Expr::mul((**a).clone(), ln10))
        }
        Expr::Sqrt(a) => {
            // d/dx sqrt(u) = u' / (2 sqrt(u))
            let two_sqrt = Expr::mul(Expr::num(2.0), Expr::sqrt((**a).clone()));
            Expr::div(diff(a), two_sqrt)
        }
        Expr::Abs(a) => {
            // d/dx |u| = u' * u / |u|  (undefined where u == 0)
            let au = Expr::abs((**a).clone());
            Expr::mul(diff(a), Expr::div((**a).clone(), au))
        }
    }
}
