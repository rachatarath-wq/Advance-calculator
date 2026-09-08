//! AC power analysis — average (real) power of sinusoidal voltage/current.
//!
//! The instantaneous power is `p(t) = v(t)·i(t)` and the average power (in
//! watts) is computed by numerical integration over one period:
//!
//! ```text
//! P = (1/T) ∫[0,T] v(t) i(t) dt
//! ```
//!
//! For `v(t) = Vm·sin(ωt)` and `i(t) = Im·sin(ωt + φ)` this equals
//! `Vm·Im·cos(φ)/2 = Vrms·Irms·cos(φ)`. The integration path is used so the
//! result is genuinely the area under the power curve.

use crate::ast::Expr;
use crate::{integrate, render, sample};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AcPower {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    // Inputs (echoed back).
    pub v_peak: f64,
    pub i_peak: f64,
    pub frequency: f64,
    pub phase_deg: f64,

    // Derived quantities.
    pub angular_freq: f64,
    pub period: f64,
    pub phase_rad: f64,

    // Results.
    pub rms_v: f64,
    pub rms_i: f64,
    pub avg_power: f64,      // P (W) — via integration
    pub apparent_power: f64, // S = Vrms·Irms (VA)
    pub reactive_power: f64, // Q = Vrms·Irms·sinφ (VAR)
    pub power_factor: f64,   // cos φ

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

/// Format a phase term as `+ 0.5236` / `- 0.5236`.
fn fmt_signed(v: f64) -> String {
    if v < 0.0 {
        format!("- {}", fmt(-v))
    } else {
        format!("+ {}", fmt(v))
    }
}

fn error(msg: &str) -> AcPower {
    AcPower {
        ok: false,
        error: Some(msg.to_string()),
        v_peak: 0.0,
        i_peak: 0.0,
        frequency: 0.0,
        phase_deg: 0.0,
        angular_freq: 0.0,
        period: 0.0,
        phase_rad: 0.0,
        rms_v: 0.0,
        rms_i: 0.0,
        avg_power: 0.0,
        apparent_power: 0.0,
        reactive_power: 0.0,
        power_factor: 0.0,
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

/// Analyse a sinusoidal AC circuit and return its power characteristics.
///
/// * `v_peak` / `i_peak` — peak voltage / current (volt / ampere)
/// * `frequency` — Hz (> 0)
/// * `phase_deg` — phase of current relative to voltage, in degrees
/// * `samples` — grid points over the sampled window (clamped)
pub fn ac_power(v_peak: f64, i_peak: f64, frequency: f64, phase_deg: f64, samples: usize) -> AcPower {
    if !frequency.is_finite() || frequency <= 0.0 {
        return error("frequency must be a positive number");
    }
    if !v_peak.is_finite() || !i_peak.is_finite() || !phase_deg.is_finite() {
        return error("invalid parameter (must be a finite number)");
    }

    let samples = samples.clamp(100, 8000);

    let angular_freq = 2.0 * std::f64::consts::PI * frequency;
    let period = 1.0 / frequency;
    let phase_rad = phase_deg.to_radians();

    // v(t) = Vm·sin(ωt), i(t) = Im·sin(ωt + φ)   (`x` plays the role of time)
    let v_expr = Expr::mul(
        Expr::num(v_peak),
        Expr::sin(Expr::mul(Expr::num(angular_freq), Expr::var())),
    );
    let i_expr = Expr::mul(
        Expr::num(i_peak),
        Expr::sin(Expr::add(
            Expr::mul(Expr::num(angular_freq), Expr::var()),
            Expr::num(phase_rad),
        )),
    );
    let p_expr = Expr::mul(v_expr.clone(), i_expr.clone());

    // Sample over two periods: an integer number of periods keeps the average
    // exact and gives the plot a full picture.
    let t0 = 0.0;
    let t1 = 2.0 * period;
    let grid = sample::xs(t0, t1, samples);
    let vs = sample::sample(&v_expr, &grid);
    let i_vals = sample::sample(&i_expr, &grid);
    let ps = sample::sample(&p_expr, &grid);

    // P = (1/T) ∫[0,T] p dt, integrated numerically.
    let total_energy = integrate::integrate_numerical(&p_expr, t0, t1);
    let avg_power = total_energy / (t1 - t0);

    let rms_v = v_peak / std::f64::consts::SQRT_2;
    let rms_i = i_peak / std::f64::consts::SQRT_2;
    let apparent_power = rms_v * rms_i;
    let power_factor = phase_rad.cos();
    let reactive_power = apparent_power * phase_rad.sin();

    let v_latex = format!(
        "v(t) = {}\\,\\sin({}\\,t)",
        fmt(v_peak),
        fmt(angular_freq)
    );
    let i_latex = format!(
        "i(t) = {}\\,\\sin({}\\,t {})",
        fmt(i_peak),
        fmt(angular_freq),
        fmt_signed(phase_rad)
    );
    let integral_latex = format!(
        "P = \\frac{{1}}{{T}}\\int_{{0}}^{{T}} v(t)\\,i(t)\\,dt = {}\\,\\text{{W}}",
        render::format_number(avg_power)
    );

    AcPower {
        ok: true,
        error: None,
        v_peak,
        i_peak,
        frequency,
        phase_deg,
        angular_freq,
        period,
        phase_rad,
        rms_v,
        rms_i,
        avg_power,
        apparent_power,
        reactive_power,
        power_factor,
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
