//! Abstract syntax tree for mathematical expressions.

/// A named symbolic constant (`pi`, `e`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Const {
    Pi,
    E,
}

impl Const {
    pub fn value(self) -> f64 {
        match self {
            Const::Pi => std::f64::consts::PI,
            Const::E => std::f64::consts::E,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Const::Pi => "pi",
            Const::E => "e",
        }
    }
}

/// A mathematical expression over a single variable `x`.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(f64),
    Const(Const),
    Var,
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Pow(Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),
    Sin(Box<Expr>),
    Cos(Box<Expr>),
    Tan(Box<Expr>),
    Asin(Box<Expr>),
    Acos(Box<Expr>),
    Atan(Box<Expr>),
    Sinh(Box<Expr>),
    Cosh(Box<Expr>),
    Tanh(Box<Expr>),
    Exp(Box<Expr>),
    Ln(Box<Expr>),
    Log(Box<Expr>),
    Sqrt(Box<Expr>),
    Abs(Box<Expr>),
}

impl Expr {
    pub fn num(n: f64) -> Expr {
        Expr::Num(n)
    }
    pub fn var() -> Expr {
        Expr::Var
    }
    pub fn add(a: Expr, b: Expr) -> Expr {
        Expr::Add(Box::new(a), Box::new(b))
    }
    pub fn sub(a: Expr, b: Expr) -> Expr {
        Expr::Sub(Box::new(a), Box::new(b))
    }
    pub fn mul(a: Expr, b: Expr) -> Expr {
        Expr::Mul(Box::new(a), Box::new(b))
    }
    pub fn div(a: Expr, b: Expr) -> Expr {
        Expr::Div(Box::new(a), Box::new(b))
    }
    pub fn pow(base: Expr, exp: Expr) -> Expr {
        Expr::Pow(Box::new(base), Box::new(exp))
    }
    pub fn neg(a: Expr) -> Expr {
        Expr::Neg(Box::new(a))
    }

    pub fn sin(a: Expr) -> Expr {
        Expr::Sin(Box::new(a))
    }
    pub fn cos(a: Expr) -> Expr {
        Expr::Cos(Box::new(a))
    }
    pub fn tan(a: Expr) -> Expr {
        Expr::Tan(Box::new(a))
    }
    pub fn asin(a: Expr) -> Expr {
        Expr::Asin(Box::new(a))
    }
    pub fn acos(a: Expr) -> Expr {
        Expr::Acos(Box::new(a))
    }
    pub fn atan(a: Expr) -> Expr {
        Expr::Atan(Box::new(a))
    }
    pub fn sinh(a: Expr) -> Expr {
        Expr::Sinh(Box::new(a))
    }
    pub fn cosh(a: Expr) -> Expr {
        Expr::Cosh(Box::new(a))
    }
    pub fn tanh(a: Expr) -> Expr {
        Expr::Tanh(Box::new(a))
    }
    pub fn exp(a: Expr) -> Expr {
        Expr::Exp(Box::new(a))
    }
    pub fn ln(a: Expr) -> Expr {
        Expr::Ln(Box::new(a))
    }
    pub fn log(a: Expr) -> Expr {
        Expr::Log(Box::new(a))
    }
    pub fn sqrt(a: Expr) -> Expr {
        Expr::Sqrt(Box::new(a))
    }
    pub fn abs(a: Expr) -> Expr {
        Expr::Abs(Box::new(a))
    }

    /// Whether the expression is a plain constant (independent of `x`).
    pub fn is_constant(&self) -> bool {
        match self {
            Expr::Num(_) | Expr::Const(_) => true,
            _ => false,
        }
    }
}

/// Build a single-argument function node from its name. The parser guarantees
/// the name is one of the known functions, so a fallback never triggers.
pub fn make_unary_fn(name: &str, arg: Expr) -> Expr {
    let b = Box::new(arg);
    match name {
        "sin" => Expr::Sin(b),
        "cos" => Expr::Cos(b),
        "tan" => Expr::Tan(b),
        "asin" => Expr::Asin(b),
        "acos" => Expr::Acos(b),
        "atan" => Expr::Atan(b),
        "sinh" => Expr::Sinh(b),
        "cosh" => Expr::Cosh(b),
        "tanh" => Expr::Tanh(b),
        "exp" => Expr::Exp(b),
        "ln" => Expr::Ln(b),
        "sqrt" => Expr::Sqrt(b),
        "abs" => Expr::Abs(b),
        _ => unreachable!("unknown function name '{}'", name),
    }
}
