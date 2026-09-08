//! RL (inductive) load — AC-motor equivalent — power analysis.
//!
//! A series `R + jωL` load is the standard single-phase model of an AC motor /
//! inductive load. Two views are provided:
//!
//! 1. [`rl_ac_power`] — steady state (full sine): `i(t)` lags `v(t)` by
//!    `φ = atan(ωL/R)`, so real power is `P = Vrms·Irms·cosφ`.
//! 2. [`rl_chopper`] — phase control (TRIAC / AC voltage controller): firing at
//!    `α` lets the inductive current persist past the voltage zero crossing, so
//!    the extinction angle `β′` (> π) is found by solving the circuit ODE
//!    `L·di/dt + R·i = Vm·sin(ωt)`; the power is then integrated over the
//!    conduction window `[α, β′]`.

use crate::ast::Expr;
use crate::{integrate, render, sample};
use serde::Serialize;

/// Compact, human-readable number with at most 4 decimals.
fn fmt(v: f64) -> String {
    if v == 0.0 {
        return "0".into();
    }
    let s = format!("{:.4}", v);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Fixed-grid composite Simpson's rule for a smooth integrand.
fn simpson<F: Fn(f64) -> f64>(f: F, a: f64, b: f64, n: usize) -> f64 {
    let n = (n / 2) * 2;
    let h = (b - a) / n as f64;
    let mut s = f(a) + f(b);
    for i in 1..n {
        let x = a + i as f64 * h;
        s += if i % 2 == 0 { 2.0 } else { 4.0 } * f(x);
    }
    s * h / 3.0
}

/// Current shape (unnormalised) for phase control with an RL load, in phase
/// variable θ = ωt over the conduction interval `[α, β′]`:
///
/// ```text
/// g(θ) = sin(θ − φ) − sin(α − φ)·e^{−(θ − α)/tanφ}
/// ```
///
/// The actual current is `i(θ) = (Vm/|Z|)·g(θ)`.
fn current_shape(theta: f64, alpha: f64, phi: f64, tan_phi: f64) -> f64 {
    (theta - phi).sin() - (alpha - phi).sin() * (-(theta - alpha) / tan_phi).exp()
}

/// Solve for the extinction angle β′ ∈ (α, α+π): the first angle > α where the
/// current returns to zero. `g(α) = 0` is the trivial firing root; `g(α+π) < 0`,
/// so bisection on `[α+ε, α+π]` locates the true root.
fn extinction_angle(alpha: f64, phi: f64, tan_phi: f64) -> f64 {
    let pi = std::f64::consts::PI;
    let mut lo = alpha + 1e-9;
    let mut hi = alpha + pi;
    for _ in 0..200 {
        let mid = (lo + hi) / 2.0;
        if current_shape(mid, alpha, phi, tan_phi) > 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

// ---------------------------------------------------------------------------
// Steady-state RL (full sine)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize)]
pub struct RlPower {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    pub v_peak: f64,
    pub load_r: f64,
    pub load_l: f64,
    pub frequency: f64,

    pub angular_freq: f64,
    pub period: f64,
    pub impedance: f64,
    pub phase_deg: f64, // φ, current lags voltage
    pub phase_rad: f64,
    pub i_peak: f64,

    pub rms_v: f64,
    pub rms_i: f64,
    pub avg_power: f64,
    pub apparent_power: f64,
    pub reactive_power: f64,
    pub power_factor: f64,

    pub v_latex: String,
    pub i_latex: String,
    pub z_latex: String,
    pub integral_latex: String,

    pub t0: f64,
    pub t1: f64,
    pub ts: Vec<f64>,
    pub vs: Vec<f64>,
    pub i_vals: Vec<f64>,
    pub ps: Vec<f64>,
}

fn rl_error(msg: &str) -> RlPower {
    RlPower {
        ok: false,
        error: Some(msg.to_string()),
        ..Default::default()
    }
}

/// Steady-state power of a series RL load (motor equivalent) under a full sine.
pub fn rl_ac_power(
    v_peak: f64,
    load_r: f64,
    load_l: f64,
    frequency: f64,
    samples: usize,
) -> RlPower {
    if !frequency.is_finite() || frequency <= 0.0 {
        return rl_error("frequency must be a positive number");
    }
    if !load_r.is_finite() || load_r <= 0.0 {
        return rl_error("load resistance must be a positive number");
    }
    if !load_l.is_finite() || load_l <= 0.0 {
        return rl_error("load inductance must be a positive number");
    }
    if !v_peak.is_finite() || v_peak <= 0.0 {
        return rl_error("peak voltage must be a positive number");
    }

    let samples = samples.clamp(100, 8000);
    let angular_freq = 2.0 * std::f64::consts::PI * frequency;
    let period = 1.0 / frequency;
    let xl = angular_freq * load_l;
    let impedance = (load_r * load_r + xl * xl).sqrt();
    let phase_rad = xl.atan2(load_r);
    let phase_deg = phase_rad.to_degrees();
    let i_peak = v_peak / impedance;

    // v(t) = Vm·sin(ωt), i(t) = Im·sin(ωt − φ)  (current lags).
    let v_expr = Expr::mul(
        Expr::num(v_peak),
        Expr::sin(Expr::mul(Expr::num(angular_freq), Expr::var())),
    );
    let i_expr = Expr::mul(
        Expr::num(i_peak),
        Expr::sin(Expr::sub(
            Expr::mul(Expr::num(angular_freq), Expr::var()),
            Expr::num(phase_rad),
        )),
    );
    let p_expr = Expr::mul(v_expr.clone(), i_expr.clone());

    let t0 = 0.0;
    let t1 = 2.0 * period;
    let grid = sample::xs(t0, t1, samples);
    let vs = sample::sample(&v_expr, &grid);
    let i_vals = sample::sample(&i_expr, &grid);
    let ps = sample::sample(&p_expr, &grid);

    let total_energy = integrate::integrate_numerical(&p_expr, t0, t1);
    let avg_power = total_energy / (t1 - t0);

    let rms_v = v_peak / std::f64::consts::SQRT_2;
    let rms_i = i_peak / std::f64::consts::SQRT_2;
    let apparent_power = rms_v * rms_i;
    let power_factor = phase_rad.cos();
    let reactive_power = apparent_power * phase_rad.sin();

    let v_latex = format!("v(t) = {}\\,\\sin({}\\,t)", fmt(v_peak), fmt(angular_freq));
    let i_latex = format!(
        "i(t) = {}\\,\\sin({}\\,t - {})",
        fmt(i_peak),
        fmt(angular_freq),
        fmt(phase_rad)
    );
    let z_latex = format!(
        "Z = R + j\\omega L = {} + j{}\\,\\Omega,\\quad \\phi = {}^{{\\circ}}",
        fmt(load_r),
        fmt(xl),
        fmt(phase_deg)
    );
    let integral_latex = format!(
        "P = \\frac{{1}}{{T}}\\int_0^T v(t)\\,i(t)\\,dt = {}\\,\\text{{W}}",
        render::format_number(avg_power)
    );

    RlPower {
        ok: true,
        error: None,
        v_peak,
        load_r,
        load_l,
        frequency,
        angular_freq,
        period,
        impedance,
        phase_deg,
        phase_rad,
        i_peak,
        rms_v,
        rms_i,
        avg_power,
        apparent_power,
        reactive_power,
        power_factor,
        v_latex,
        i_latex,
        z_latex,
        integral_latex,
        t0,
        t1,
        ts: grid,
        vs,
        i_vals,
        ps,
    }
}

// ---------------------------------------------------------------------------
// Phase-controlled RL (chopper / AC voltage controller)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize)]
pub struct RlChopper {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    pub v_peak: f64,
    pub load_r: f64,
    pub load_l: f64,
    pub frequency: f64,
    pub alpha_deg: f64,

    pub angular_freq: f64,
    pub period: f64,
    pub impedance: f64,
    pub phase_deg: f64, // φ
    pub phase_rad: f64,
    pub alpha_rad: f64,

    pub continuous: bool,      // true → α ≤ φ, no chopping effect
    pub extinction_deg: f64,   // β′ (current returns to zero)
    pub conduction_deg: f64,   // β′ − α

    pub avg_power: f64,
    pub rms_v: f64,
    pub rms_i: f64,
    pub apparent_power: f64,
    pub reactive_power: f64,
    pub power_factor: f64,

    pub v_latex: String,
    pub i_latex: String,
    pub integral_latex: String,
    pub note: String,

    pub t0: f64,
    pub t1: f64,
    pub ts: Vec<f64>,
    pub vs: Vec<f64>,
    pub i_vals: Vec<f64>,
    pub ps: Vec<f64>,
}

fn rlc_error(msg: &str) -> RlChopper {
    RlChopper {
        ok: false,
        error: Some(msg.to_string()),
        ..Default::default()
    }
}

/// Phase-controlled (chopped) power of a series RL load, fired at `alpha_deg`.
pub fn rl_chopper(
    v_peak: f64,
    load_r: f64,
    load_l: f64,
    frequency: f64,
    alpha_deg: f64,
    samples: usize,
) -> RlChopper {
    if !frequency.is_finite() || frequency <= 0.0 {
        return rlc_error("frequency must be a positive number");
    }
    if !load_r.is_finite() || load_r <= 0.0 {
        return rlc_error("load resistance must be a positive number");
    }
    if !load_l.is_finite() || load_l <= 0.0 {
        return rlc_error("load inductance must be a positive number");
    }
    if !v_peak.is_finite() || v_peak <= 0.0 {
        return rlc_error("peak voltage must be a positive number");
    }
    if !alpha_deg.is_finite() || !(0.0..=180.0).contains(&alpha_deg) {
        return rlc_error("firing angle must be between 0° and 180°");
    }

    let samples = samples.clamp(100, 8000);
    let pi = std::f64::consts::PI;
    let angular_freq = 2.0 * pi * frequency;
    let period = 1.0 / frequency;
    let xl = angular_freq * load_l;
    let impedance = (load_r * load_r + xl * xl).sqrt();
    let phase_rad = xl.atan2(load_r);
    let phase_deg = phase_rad.to_degrees();
    let tan_phi = xl / load_r;
    let alpha_rad = alpha_deg.to_radians();

    // Continuous conduction when α ≤ φ: the load never de-energises, so the
    // chopper has no effect and the output is the full sine.
    let (continuous, beta, avg_power, rms_v, rms_i, apparent, reactive, pf) =
        if alpha_rad <= phase_rad {
            let p = v_peak * v_peak * load_r / (2.0 * impedance * impedance);
            let rv = v_peak / std::f64::consts::SQRT_2;
            let ri = rv / impedance;
            let s = rv * ri;
            (true, alpha_rad + pi, p, rv, ri, s, s * phase_rad.sin(), phase_rad.cos())
        } else {
            let beta = extinction_angle(alpha_rad, phase_rad, tan_phi);
            let im = v_peak / impedance;
            let p_half = simpson(
                |th| v_peak * th.sin() * im * current_shape(th, alpha_rad, phase_rad, tan_phi),
                alpha_rad,
                beta,
                2000,
            );
            let vsq = simpson(|th| (v_peak * th.sin()).powi(2), alpha_rad, beta, 2000);
            let isq = simpson(
                |th| (im * current_shape(th, alpha_rad, phase_rad, tan_phi)).powi(2),
                alpha_rad,
                beta,
                2000,
            );
            let p = p_half / pi;
            let rv = (vsq / pi).sqrt();
            let ri = (isq / pi).sqrt();
            let s = rv * ri;
            let pf = if s > 0.0 { p / s } else { 0.0 };
            let react = (s * s - p * p).max(0.0).sqrt();
            (false, beta, p, rv, ri, s, react, pf)
        };

    let extinction_deg = beta.to_degrees();
    let conduction_deg = (beta - alpha_rad).to_degrees();

    // Gated samples over two periods for the plot.
    let t0 = 0.0;
    let t1 = 2.0 * period;
    let grid = sample::xs(t0, t1, samples);
    let mut vs = Vec::with_capacity(grid.len());
    let mut i_vals = Vec::with_capacity(grid.len());
    let mut ps = Vec::with_capacity(grid.len());
    let im = v_peak / impedance;
    for &t in &grid {
        let th = angular_freq * t;
        let mut v = 0.0;
        let mut i = 0.0;
        if continuous {
            v = v_peak * th.sin();
            i = im * (th - phase_rad).sin();
        } else {
            for k in 0..4i32 {
                let lo = alpha_rad + k as f64 * pi;
                let hi = beta + k as f64 * pi;
                if th >= lo && th <= hi {
                    let th0 = th - k as f64 * pi;
                    let sign = if k % 2 == 0 { 1.0 } else { -1.0 };
                    v = sign * v_peak * th0.sin();
                    i = sign * im * current_shape(th0, alpha_rad, phase_rad, tan_phi);
                    break;
                }
            }
        }
        vs.push(v);
        i_vals.push(i);
        ps.push(v * i);
    }

    let v_latex = format!("v(t) = {}\\,\\sin({}\\,t)", fmt(v_peak), fmt(angular_freq));
    let i_latex = if continuous {
        format!(
            "i(t) = {}\\,\\sin({}\\,t - {})",
            fmt(im),
            fmt(angular_freq),
            fmt(phase_rad)
        )
    } else {
        "i(\\theta)=\\tfrac{V_m}{|Z|}\\left[\\sin(\\theta-\\phi)-\\sin(\\alpha-\\phi)e^{-(\\theta-\\alpha)/\\tan\\phi}\\right],\\;\\theta\\in[\\alpha,\\beta']".to_string()
    };
    let integral_latex = format!(
        "P = \\frac{{1}}{{\\pi}}\\int_{{\\alpha}}^{{\\beta'}} v(\\theta)\\,i(\\theta)\\,d\\theta = {}\\,\\text{{W}}",
        render::format_number(avg_power)
    );
    let note = if continuous {
        format!(
            "α = {}° ≤ φ = {}° → continuous conduction (no voltage reduction; output = full sine)",
            fmt(alpha_deg),
            fmt(phase_deg)
        )
    } else {
        format!(
            "φ = {}°, α = {}°, extinction β′ = {}°, conduction = {}°",
            fmt(phase_deg),
            fmt(alpha_deg),
            fmt(extinction_deg),
            fmt(conduction_deg)
        )
    };

    RlChopper {
        ok: true,
        error: None,
        v_peak,
        load_r,
        load_l,
        frequency,
        alpha_deg,
        angular_freq,
        period,
        impedance,
        phase_deg,
        phase_rad,
        alpha_rad,
        continuous,
        extinction_deg,
        conduction_deg,
        avg_power,
        rms_v,
        rms_i,
        apparent_power: apparent,
        reactive_power: reactive,
        power_factor: pf,
        v_latex,
        i_latex,
        integral_latex,
        note,
        t0,
        t1,
        ts: grid,
        vs,
        i_vals,
        ps,
    }
}
