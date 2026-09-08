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
