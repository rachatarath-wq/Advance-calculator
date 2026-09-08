//! Chopped (phase-controlled) sine-wave power analysis.
//!
//! A sinusoidal voltage is "chopped": within each half-cycle the load conducts
//! only while the phase angle `θ = ω·t (mod π)` lies inside a window `[α, β]`.
//! With a resistive load `R`, `i(t) = v(t)/R` and the instantaneous power is
//! `p(t) = v(t)²/R`. The average (real) power is obtained by numerical
//! integration over one full period:
//!
//! ```text
//! P = (1/T) ∫[0,T] v²(t)/R dt
//! ```
//!
//! Conduction is symmetric in both half-cycles, so this reduces to integrating
//! `sin²` over `[α, β]` twice:
//!
//! ```text
//! P = (Vm²/(R·π)) · [θ/2 − sin(2θ)/4] from α to β
//! ```
//!
//! For α = 0°, β = 180° (no chopping) this returns `Vm²/(2R) = Vrms²/R`.

use crate::ast::Expr;
use crate::{integrate, render, sample};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ChopperPower {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    // Inputs (echoed back).
    pub v_peak: f64,
    pub load_r: f64,
    pub frequency: f64,
    pub alpha_deg: f64,
    pub beta_deg: f64,

    // Derived.
    pub angular_freq: f64,
    pub period: f64,
    pub alpha_rad: f64,
    pub beta_rad: f64,

    // Results.
    pub avg_power: f64,       // P (W) — via integration
    pub rms_v: f64,           // chopped RMS voltage
    pub rms_i: f64,           // chopped RMS current
    pub conduction_duty: f64, // (β − α) / π

    // LaTeX strings for the UI.
    pub v_latex: String,
    pub i_latex: String,
    pub integral_latex: String,

    // Plottable samples over two periods.
    pub t0: f64,
    pub t1: f64,
    pub ts: Vec<f64>,
    pub vs: Vec<f64>,
    pub i_vals: Vec<f64>,
    pub ps: Vec<f64>,
}

/// Compact, human-readable number with at most 4 decimals.
fn fmt(v: f64) -> String {
    if v == 0.0 {
        return "0".into();
    }
    let s = format!("{:.4}", v);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn error(msg: &str) -> ChopperPower {
    ChopperPower {
        ok: false,
        error: Some(msg.to_string()),
        v_peak: 0.0,
        load_r: 0.0,
        frequency: 0.0,
        alpha_deg: 0.0,
        beta_deg: 0.0,
        angular_freq: 0.0,
        period: 0.0,
        alpha_rad: 0.0,
        beta_rad: 0.0,
        avg_power: 0.0,
        rms_v: 0.0,
        rms_i: 0.0,
        conduction_duty: 0.0,
        v_latex: String::new(),
        i_latex: String::new(),
        integral_latex: String::new(),
        t0: 0.0,
        t1: 0.0,
        ts: vec![],
        vs: vec![],
        i_vals: vec![],
        ps: vec![],
    }
}

/// Analyse a chopped sine wave (phase control) with a resistive load.
///
/// * `v_peak` — peak voltage (volt, > 0)
/// * `load_r` — load resistance (ohm, > 0)
/// * `frequency` — Hz (> 0)
/// * `alpha_deg` / `beta_deg` — conduction window in degrees (0 ≤ α < β ≤ 180)
/// * `samples` — grid points over the sampled window (clamped)
pub fn chopper_power(
    v_peak: f64,
    load_r: f64,
    frequency: f64,
    alpha_deg: f64,
    beta_deg: f64,
    samples: usize,
) -> ChopperPower {
    if !frequency.is_finite() || frequency <= 0.0 {
        return error("frequency must be a positive number");
    }
    if !load_r.is_finite() || load_r <= 0.0 {
        return error("load resistance must be a positive number");
    }
    if !v_peak.is_finite() || v_peak <= 0.0 {
        return error("peak voltage must be a positive number");
    }
    if !alpha_deg.is_finite() || !beta_deg.is_finite() {
        return error("conduction angles must be finite numbers");
    }
    if alpha_deg < 0.0 || beta_deg > 180.0 || alpha_deg >= beta_deg {
        return error("conduction window must satisfy 0° ≤ α < β ≤ 180°");
    }

    let samples = samples.clamp(100, 8000);

    let angular_freq = 2.0 * std::f64::consts::PI * frequency;
    let period = 1.0 / frequency;
    let alpha = alpha_deg.to_radians();
    let beta = beta_deg.to_radians();

    // Smooth (ungated) expressions: v(t) = Vm·sin(ωt), i = v/R, p = v²/R.
    let v_expr = Expr::mul(
        Expr::num(v_peak),
        Expr::sin(Expr::mul(Expr::num(angular_freq), Expr::var())),
    );
    let i_expr = Expr::div(v_expr.clone(), Expr::num(load_r));
    let p_expr = Expr::mul(v_expr.clone(), i_expr.clone());
    let v_sq_expr = Expr::mul(v_expr.clone(), v_expr.clone());

    // Conduction windows inside one period [0, T]: [α/ω, β/ω] and [π+α, π+β]/ω.
    let w = angular_freq;
    let pi = std::f64::consts::PI;
    let energy = integrate::integrate_numerical(&p_expr, alpha / w, beta / w)
        + integrate::integrate_numerical(&p_expr, (pi + alpha) / w, (pi + beta) / w);
    let vsq_energy = integrate::integrate_numerical(&v_sq_expr, alpha / w, beta / w)
        + integrate::integrate_numerical(&v_sq_expr, (pi + alpha) / w, (pi + beta) / w);

    let avg_power = energy / period;
    let rms_v = (vsq_energy / period).sqrt();
    let rms_i = rms_v / load_r;
    let conduction_duty = (beta - alpha) / pi;

    // Gated samples over two periods for the plot (zero outside [α, β]).
    let t0 = 0.0;
    let t1 = 2.0 * period;
    let grid = sample::xs(t0, t1, samples);
    let mut vs = Vec::with_capacity(grid.len());
    let mut i_vals = Vec::with_capacity(grid.len());
    let mut ps = Vec::with_capacity(grid.len());
    for &t in &grid {
        let theta = (w * t).rem_euclid(pi);
        let conducting = theta >= alpha && theta <= beta;
        let v = if conducting { v_peak * (w * t).sin() } else { 0.0 };
        let i = v / load_r;
        vs.push(v);
        i_vals.push(i);
        ps.push(v * i);
    }

    let v_latex = format!(
        "v(t) = {}\\,\\sin({}\\,t),\\quad \\theta \\in [{}^\\circ,\\ {}^\\circ]",
        fmt(v_peak),
        fmt(angular_freq),
        fmt(alpha_deg),
        fmt(beta_deg)
    );
    let i_latex = format!(
        "i(t) = \\frac{{v(t)}}{{R}} = \\frac{{{}}}{{{}}}\\,\\sin({}\\,t)",
        fmt(v_peak),
        fmt(load_r),
        fmt(angular_freq)
    );
    let integral_latex = format!(
        "P = \\frac{{1}}{{T}}\\int_{{0}}^{{T}} \\frac{{v^2(t)}}{{R}}\\,dt = {}\\,\\text{{W}}",
        render::format_number(avg_power)
    );

    ChopperPower {
        ok: true,
        error: None,
        v_peak,
        load_r,
        frequency,
        alpha_deg,
        beta_deg,
        angular_freq,
        period,
        alpha_rad: alpha,
        beta_rad: beta,
        avg_power,
        rms_v,
        rms_i,
        conduction_duty,
        v_latex,
        i_latex,
        integral_latex,
        t0,
        t1,
        ts: grid,
        vs,
        i_vals,
        ps,
    }
}
