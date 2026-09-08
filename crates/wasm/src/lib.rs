//! WASM binding layer: exposes the core engine to JavaScript via
//! `wasm-bindgen`. `process` returns the analysis as a JSON string, which the
//! frontend parses with `JSON.parse`.

use wasm_bindgen::prelude::*;

/// Analyse a function expression and return JSON (see `calcsim_core::Analysis`).
///
/// * `input`   — e.g. `"x^2 + sin(x)"`
/// * `x_min` / `x_max` — plotting domain
/// * `samples` — number of grid points (clamped to 50..=4000)
#[wasm_bindgen]
pub fn process(input: &str, x_min: f64, x_max: f64, samples: usize) -> String {
    calcsim_core::process_json(input, x_min, x_max, samples)
}

/// Analyse a sinusoidal AC circuit and return power characteristics as JSON
/// (see `calcsim_core::ac::AcPower`).
///
/// * `v_peak` / `i_peak` — peak voltage (V) / current (A)
/// * `frequency` — Hz
/// * `phase_deg` — current phase relative to voltage, degrees
#[wasm_bindgen]
pub fn ac_power(v_peak: f64, i_peak: f64, frequency: f64, phase_deg: f64, samples: usize) -> String {
    calcsim_core::ac_power_json(v_peak, i_peak, frequency, phase_deg, samples)
}

/// Analyse a chopped (phase-controlled) sine wave with a resistive load and
/// return power characteristics as JSON (see `calcsim_core::chopper::ChopperPower`).
///
/// * `v_peak` — peak voltage (V)
/// * `load_r` — load resistance (Ω)
/// * `frequency` — Hz
/// * `alpha_deg` / `beta_deg` — conduction window, degrees (0 ≤ α < β ≤ 180)
#[wasm_bindgen]
pub fn chopper_power(
    v_peak: f64,
    load_r: f64,
    frequency: f64,
    alpha_deg: f64,
    beta_deg: f64,
    samples: usize,
) -> String {
    calcsim_core::chopper_power_json(v_peak, load_r, frequency, alpha_deg, beta_deg, samples)
}
