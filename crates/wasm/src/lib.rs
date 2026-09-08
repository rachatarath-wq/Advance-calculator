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
