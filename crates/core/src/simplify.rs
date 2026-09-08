//! Best-effort algebraic simplification. Not a full CAS — it removes the
//! obvious cruft that differentiation produces (0s, 1s, constant folding) and
//! normalises a few common shapes so the printed result is readable.

use crate::ast::Expr;

const MAX_PASSES: usize = 8;

pub fn simplify(e: &Expr) -> Expr {
    let mut cur = e.clone();
    for _ in 0..MAX_PASSES {
        let next = pass(&cur);
        if next == cur {
            break;
        }
        cur = next;
    }
    cur
}

fn pass(e: &Expr) -> Expr {
    let e = match e {
        // Constant folding.
        Expr::Add(a, b) if a.is_constant() && b.is_constant() => {
            Expr::num(eval_const(a) + eval_const(b))
        }
        Expr::Sub(a, b) if a.is_constant() && b.is_constant() => {
            Expr::num(eval_const(a) - eval_const(b))
        }
        Expr::Mul(a, b) if a.is_constant() && b.is_constant() => {
            Expr::num(eval_const(a) * eval_const(b))
        }
        Expr::Div(a, b) if a.is_constant() && b.is_constant() => {
            Expr::num(eval_const(a) / eval_const(b))
        }
        Expr::Pow(a, b) if a.is_constant() && b.is_constant() => {
            Expr::num(eval_const(a).powf(eval_const(b)))
        }
        Expr::Neg(a) if a.is_constant() => Expr::num(-eval_const(a)),

        // ---- Additive identities ----
        Expr::Add(a, b) if is_zero(b) => (**a).clone(),
        Expr::Add(a, b) if is_zero(a) => (**b).clone(),
        Expr::Sub(a, b) if is_zero(b) => (**a).clone(),
        Expr::Sub(a, b) if is_zero(a) => Expr::neg((**b).clone()),

        // ---- Multiplicative identities ----
        Expr::Mul(a, b) if is_one(b) => (**a).clone(),
        Expr::Mul(a, b) if is_one(a) => (**b).clone(),
        Expr::Mul(a, b) if is_zero(a) || is_zero(b) => Expr::num(0.0),

        // Normalise products:
        //   - fold numeric coefficients through nested multiplication  2*(2*x) -> 4*x
        //   - float a numeric factor in front of the product            f*(2*x) -> (2*x)*f
        //   - pull a leading negation out of the second factor          a*(-b)  -> (-a)*b
        Expr::Mul(a, b) => {
            if let Expr::Mul(c, d) = &**b {
                match (as_num(a), as_num(c)) {
                    (Some(x), Some(y)) => {
                        return Expr::mul(Expr::num(x * y), (**d).clone());
                    }
                    (None, Some(_)) if !matches!(**a, Expr::Mul(_, _)) => {
                        return Expr::mul((**b).clone(), (**a).clone());
                    }
                    _ => {}
                }
            }
            if let Expr::Neg(inner) = &**b {
                if !matches!(**a, Expr::Neg(_)) {
                    return Expr::mul(Expr::neg((**a).clone()), (**inner).clone());
                }
            }
            e.clone()
        }

        Expr::Div(a, b) if is_one(b) => (**a).clone(),
        Expr::Div(a, b) if is_zero(a) => Expr::num(0.0),

        // ---- Power identities ----
        Expr::Pow(a, b) if is_one(b) => (**a).clone(),
        Expr::Pow(a, b) if is_zero(b) => Expr::num(1.0),
        Expr::Pow(a, b) if is_one(a) => Expr::num(1.0),
        Expr::Pow(a, b) if is_zero(a) => Expr::num(0.0),

        // ---- Negation ----
        Expr::Neg(a) if matches!(**a, Expr::Neg(_)) => {
            if let Expr::Neg(inner) = &**a {
                (**inner).clone()
            } else {
                unreachable!()
            }
        }
        Expr::Neg(a) if is_zero(a) => Expr::num(0.0),

        _ => e.clone(),
    };

    // Recurse into children (bottom-up).
    match e {
        Expr::Add(a, b) => Expr::add(pass(&a), pass(&b)),
        Expr::Sub(a, b) => Expr::sub(pass(&a), pass(&b)),
        Expr::Mul(a, b) => Expr::mul(pass(&a), pass(&b)),
        Expr::Div(a, b) => Expr::div(pass(&a), pass(&b)),
        Expr::Pow(a, b) => Expr::pow(pass(&a), pass(&b)),
        Expr::Neg(a) => Expr::neg(pass(&a)),
        Expr::Sin(a) => Expr::Sin(Box::new(pass(&a))),
        Expr::Cos(a) => Expr::Cos(Box::new(pass(&a))),
        Expr::Tan(a) => Expr::Tan(Box::new(pass(&a))),
        Expr::Asin(a) => Expr::Asin(Box::new(pass(&a))),
        Expr::Acos(a) => Expr::Acos(Box::new(pass(&a))),
        Expr::Atan(a) => Expr::Atan(Box::new(pass(&a))),
        Expr::Sinh(a) => Expr::Sinh(Box::new(pass(&a))),
        Expr::Cosh(a) => Expr::Cosh(Box::new(pass(&a))),
        Expr::Tanh(a) => Expr::Tanh(Box::new(pass(&a))),
        Expr::Exp(a) => Expr::Exp(Box::new(pass(&a))),
        Expr::Ln(a) => Expr::Ln(Box::new(pass(&a))),
        Expr::Log(a) => Expr::Log(Box::new(pass(&a))),
        Expr::Sqrt(a) => Expr::Sqrt(Box::new(pass(&a))),
        Expr::Abs(a) => Expr::Abs(Box::new(pass(&a))),
        other => other,
    }
}

fn eval_const(e: &Expr) -> f64 {
    crate::eval::eval(e, 0.0)
}

fn is_zero(e: &Expr) -> bool {
    matches!(e, Expr::Num(n) if *n == 0.0)
}

fn is_one(e: &Expr) -> bool {
    matches!(e, Expr::Num(n) if *n == 1.0)
}

fn as_num(e: &Expr) -> Option<f64> {
    match e {
        Expr::Num(n) => Some(*n),
        _ => None,
    }
}
