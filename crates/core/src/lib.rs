//! `calcsim-core` — the calculus engine.
//!
//! Public entry point is [`process`] (and its JSON helper [`process_json`]),
//! which parse an input string, differentiate and integrate it, and return
//! formatted results plus plottable samples for the frontend.

pub mod ac;
pub mod ast;
pub mod chopper;
pub mod diff;
pub mod eval;
pub mod fft;
pub mod integrate;
pub mod moon;
pub mod parser;
pub mod render;
pub mod rl;
pub mod sample;
pub mod shuttle;
pub mod simplify;

use ast::Expr;
use serde::Serialize;

/// Result of analysing one function. Serialisable straight to JSON for the
/// WASM and Tauri bindings.
#[derive(Debug, Clone, Serialize)]
pub struct Analysis {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Echo of the input, normalised into readable form.
    pub input: String,
    pub input_latex: String,
    /// Symbolic derivative (simplified).
    pub derivative: String,
    pub derivative_latex: String,
    /// Symbolic antiderivative, if a rule matched.
    pub integral: String,
    pub integral_latex: String,
    pub integral_symbolic: bool,
    /// ∫[x_min, x_max] f(x) dx via adaptive Simpson.
    pub definite_value: Option<f64>,
    pub x_min: f64,
    pub x_max: f64,
    pub xs: Vec<f64>,
    pub ys: Vec<f64>,
    pub dys: Vec<f64>,
    pub fs: Vec<f64>,
}

/// Analyse `input` over `[x_min, x_max]` with `samples` grid points.
///
/// Never panics on bad input — returns an [`Analysis`] with `ok == false` and
/// a human-readable `error` instead.
pub fn process(input: &str, x_min: f64, x_max: f64, samples: usize) -> Analysis {
    let samples = samples.clamp(50, 4000);

    let expr: Expr = match parser::parse(input) {
        Ok(e) => e,
        Err(msg) => {
            return Analysis {
                ok: false,
                error: Some(msg),
                input: input.to_string(),
                input_latex: String::new(),
                derivative: String::new(),
                derivative_latex: String::new(),
                integral: String::new(),
                integral_latex: String::new(),
                integral_symbolic: false,
                definite_value: None,
                x_min,
                x_max,
                xs: vec![],
                ys: vec![],
                dys: vec![],
                fs: vec![],
            }
        }
    };

    let grid = sample::xs(x_min, x_max, samples);

    let derivative = simplify::simplify(&diff::diff(&expr));

    let (integral, integral_latex, integral_symbolic) = match integrate::integrate_symbolic(&expr)
    {
        Some(anti) => {
            let anti = simplify::simplify(&anti);
            (render::to_string(&anti), render::to_latex(&anti), true)
        }
        None => (
            "numerical only".to_string(),
            "\\text{numerical only}".to_string(),
            false,
        ),
    };

    let definite_value = {
        let v = integrate::integrate_numerical(&expr, x_min, x_max);
        if v.is_finite() {
            Some(v)
        } else {
            None
        }
    };

    let ys = sample::sample(&expr, &grid);
    let dys = sample::sample(&derivative, &grid);

    // Antiderivative curve F(x) = ∫[x_min, x] f(t) dt.
    let fs_raw = integrate::antiderivative_curve(&expr, &grid);
    let fs = fs_raw
        .into_iter()
        .map(|v| {
            if v.is_finite() && v.abs() <= sample::Y_CAP {
                v
            } else {
                f64::NAN
            }
        })
        .collect();

    Analysis {
        ok: true,
        error: None,
        input: render::to_string(&expr),
        input_latex: render::to_latex(&expr),
        derivative: render::to_string(&derivative),
        derivative_latex: render::to_latex(&derivative),
        integral,
        integral_latex,
        integral_symbolic,
        definite_value,
        x_min,
        x_max,
        xs: grid,
        ys,
        dys,
        fs,
    }
}

/// Convenience wrapper returning the analysis serialised as a JSON string.
pub fn process_json(input: &str, x_min: f64, x_max: f64, samples: usize) -> String {
    serde_json::to_string(&process(input, x_min, x_max, samples))
        .unwrap_or_else(|_| r#"{"ok":false,"error":"serialization failed"}"#.to_string())
}

/// JSON wrapper for [`ac::ac_power`].
pub fn ac_power_json(v_peak: f64, i_peak: f64, frequency: f64, phase_deg: f64, samples: usize) -> String {
    serde_json::to_string(&ac::ac_power(v_peak, i_peak, frequency, phase_deg, samples))
        .unwrap_or_else(|_| r#"{"ok":false,"error":"serialization failed"}"#.to_string())
}

/// JSON wrapper for [`chopper::chopper_power`].
pub fn chopper_power_json(
    v_peak: f64,
    load_r: f64,
    frequency: f64,
    alpha_deg: f64,
    beta_deg: f64,
    samples: usize,
) -> String {
    serde_json::to_string(&chopper::chopper_power(v_peak, load_r, frequency, alpha_deg, beta_deg, samples))
        .unwrap_or_else(|_| r#"{"ok":false,"error":"serialization failed"}"#.to_string())
}

/// JSON wrapper for [`rl::rl_ac_power`].
pub fn rl_ac_power_json(v_peak: f64, load_r: f64, load_l: f64, frequency: f64, samples: usize) -> String {
    serde_json::to_string(&rl::rl_ac_power(v_peak, load_r, load_l, frequency, samples))
        .unwrap_or_else(|_| r#"{"ok":false,"error":"serialization failed"}"#.to_string())
}

/// JSON wrapper for [`rl::rl_chopper`].
pub fn rl_chopper_json(
    v_peak: f64,
    load_r: f64,
    load_l: f64,
    frequency: f64,
    alpha_deg: f64,
    samples: usize,
) -> String {
    serde_json::to_string(&rl::rl_chopper(v_peak, load_r, load_l, frequency, alpha_deg, samples))
        .unwrap_or_else(|_| r#"{"ok":false,"error":"serialization failed"}"#.to_string())
}

/// JSON wrapper for [`fft::fft_spectrum`].
pub fn fft_spectrum_json(
    fs: f64,
    n: usize,
    freqs: &[f64],
    amps: &[f64],
    phases_deg: &[f64],
) -> String {
    serde_json::to_string(&fft::fft_spectrum(fs, n, freqs, amps, phases_deg))
        .unwrap_or_else(|_| r#"{"ok":false,"error":"serialization failed"}"#.to_string())
}

/// JSON wrapper for [`shuttle::shuttle_landing`].
pub fn shuttle_landing_json(
    h0: f64,
    v0: f64,
    gamma0_deg: f64,
    flare_alt: f64,
    alpha_flare_deg: f64,
) -> String {
    serde_json::to_string(&shuttle::shuttle_landing(h0, v0, gamma0_deg, flare_alt, alpha_flare_deg))
        .unwrap_or_else(|_| r#"{"ok":false,"error":"serialization failed"}"#.to_string())
}

/// JSON wrapper for [`moon::moon_mission`].
pub fn moon_mission_json(target_deg: f64, thrust_g: f64) -> String {
    serde_json::to_string(&moon::moon_mission(target_deg, thrust_g))
        .unwrap_or_else(|_| r#"{"ok":false,"error":"serialization failed"}"#.to_string())
}
