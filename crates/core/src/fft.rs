//! Fast Fourier Transform (FFT) from scratch.
//!
//! # Mathematical foundation
//!
//! ## Euler's formula — the engine underneath everything
//!
//! ```text
//! e^{iθ} = cos(θ) + i·sin(θ)
//! ```
//!
//! The N-th roots of unity (the *twiddle factors*) are pure phasors:
//!
//! ```text
//! W_N^k = e^{-i 2πk/N} = cos(2πk/N) − i·sin(2πk/N)
//! ```
//!
//! They lie on the unit circle and obey the two symmetry laws that make the
//! FFT fast:
//!
//! ```text
//! W_N^{k+N}   = W_N^k          (periodicity)
//! W_N^{k+N/2} = −W_N^k         (half-turn negation: e^{−iπ} = −1)
//! ```
//!
//! ## The DFT
//!
//! The Discrete Fourier Transform maps a length-N sequence `x[n]` to `X[k]`:
//!
//! ```text
//! X[k] = Σ_{n=0}^{N-1} x[n] · W_N^{kn}
//! ```
//!
//! Direct evaluation costs O(N²): N bins × N samples each.
//!
//! ## Cooley–Tukey Radix-2 Decimation-In-Time
//!
//! Split `x` into even- and odd-indexed halves and pull out `W_N^{2nk}`:
//!
//! ```text
//! X[k] = Σ_m x[2m]   · W_{N/2}^{mk}  +  W_N^k · Σ_m x[2m+1] · W_{N/2}^{mk}
//!      = E[k]                          +  W_N^k · O[k]
//! ```
//!
//! where `E`, `O` are length-(N/2) DFTs. Using the half-turn symmetry, the
//! second half of the output needs no extra multiplies:
//!
//! ```text
//! X[k]       = E[k] + W_N^k · O[k]      for k < N/2
//! X[k + N/2] = E[k] − W_N^k · O[k]      for k < N/2
//! ```
//!
//! Recurrence: `T(N) = 2·T(N/2) + O(N)  ⇒  T(N) = O(N log N)` — the rotational
//! symmetry of Euler's phasors collapses O(N²) work into O(N log N).
//!
//! The `fft_radix2` below is the **iterative** DIT variant: bit-reversal
//! permutation, then in-place butterflies of size 2, 4, 8, …, N.

use serde::Serialize;

const PI: f64 = std::f64::consts::PI;
const TWO_PI: f64 = 2.0 * PI;

// ---------------------------------------------------------------------------
// Minimal complex arithmetic (dependency-free — the standard library has none)
// ---------------------------------------------------------------------------

/// A complex number `re + i·im`.
#[derive(Clone, Copy, Debug)]
struct C64 {
    re: f64,
    im: f64,
}

impl C64 {
    const ZERO: C64 = C64 { re: 0.0, im: 0.0 };
    const ONE: C64 = C64 { re: 1.0, im: 0.0 };
    fn new(re: f64, im: f64) -> Self {
        C64 { re, im }
    }
    /// Euler's formula: `e^{iθ} = cos θ + i sin θ` (unit phasor at angle θ).
    fn exp_i(theta: f64) -> Self {
        C64::new(theta.cos(), theta.sin())
    }
    fn abs(self) -> f64 {
        (self.re * self.re + self.im * self.im).sqrt()
    }
}

impl std::ops::Add for C64 {
    type Output = C64;
    fn add(self, o: C64) -> C64 {
        C64::new(self.re + o.re, self.im + o.im)
    }
}

impl std::ops::Sub for C64 {
    type Output = C64;
    fn sub(self, o: C64) -> C64 {
        C64::new(self.re - o.re, self.im - o.im)
    }
}

impl std::ops::Mul for C64 {
    type Output = C64;
    fn mul(self, o: C64) -> C64 {
        C64::new(
            self.re * o.re - self.im * o.im,
            self.re * o.im + self.im * o.re,
        )
    }
}

impl std::ops::Neg for C64 {
    type Output = C64;
    fn neg(self) -> C64 {
        C64::new(-self.re, -self.im)
    }
}

impl std::ops::AddAssign for C64 {
    fn add_assign(&mut self, o: C64) {
        *self = *self + o;
    }
}

impl std::ops::MulAssign for C64 {
    fn mul_assign(&mut self, o: C64) {
        *self = *self * o;
    }
}

// ---------------------------------------------------------------------------
// Naive DFT — the O(N²) definition, kept as an independent correctness check
// ---------------------------------------------------------------------------

/// Direct evaluation of the DFT definition: `X[k] = Σ x[n]·W_N^{kn}`.
fn dft(x: &[C64]) -> Vec<C64> {
    let n = x.len();
    (0..n)
        .map(|k| {
            let mut acc = C64::ZERO;
            for (m, &xm) in x.iter().enumerate() {
                acc += xm * C64::exp_i(-TWO_PI * (k * m) as f64 / n as f64);
            }
            acc
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Radix-2 Cooley–Tukey DIT FFT (iterative, in-place, bit-reversed input)
// ---------------------------------------------------------------------------

fn bit_reverse(mut n: usize, width: u32) -> usize {
    let mut r = 0usize;
    for _ in 0..width {
        r = (r << 1) | (n & 1);
        n >>= 1;
    }
    r
}

/// Iterative Radix-2 DIT FFT. Returns `Err` unless `x.len()` is a power of two.
fn fft_radix2(x: &[C64]) -> Result<Vec<C64>, String> {
    let n = x.len();
    if n == 0 || (n & (n - 1)) != 0 {
        return Err("radix-2 FFT requires a power-of-two length".to_string());
    }

    let width = n.trailing_zeros(); // log2(n)
    let mut a = vec![C64::ZERO; n];
    for i in 0..n {
        a[i] = x[bit_reverse(i, width)];
    }

    let mut size = 2usize;
    while size <= n {
        let half = size / 2;
        // Twiddle step for this butterfly: W_size = e^{−i 2π/size}.
        let w_size = C64::exp_i(-TWO_PI / size as f64);
        let mut start = 0usize;
        while start < n {
            let mut w = C64::ONE;
            for j in 0..half {
                let u = a[start + j];
                let v = w * a[start + j + half];
                a[start + j] = u + v;
                a[start + j + half] = u - v;
                w *= w_size; // rotate the phasor incrementally
            }
            start += size;
        }
        size <<= 1;
    }
    Ok(a)
}

/// Smallest power of two `>= n`.
fn next_pow2(n: usize) -> usize {
    if n <= 1 {
        1
    } else {
        1usize << (usize::BITS - (n - 1).leading_zeros())
    }
}

// ---------------------------------------------------------------------------
// Result type (serde → JSON → frontend)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct FftResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    // Inputs (echoed back).
    pub fs: f64,
    pub n: usize,          // requested sample count
    pub n_padded: usize,   // power-of-two length actually transformed
    pub freqs_in: Vec<f64>,
    pub amps_in: Vec<f64>,
    pub phases_in: Vec<f64>,

    // Derived.
    pub df: f64,       // frequency resolution  Δf = fs / N
    pub nyquist: f64,  // fs / 2

    // Time-domain signal (the input, reconstructed).
    pub ts: Vec<f64>,
    pub xs: Vec<f64>,

    // One-sided spectra.
    pub spectrum_freqs: Vec<f64>,
    pub spectrum_mag: Vec<f64>, // |X[k]|
    pub spectrum_amp: Vec<f64>, // peak amplitude (2|X[k]|/N, DC /N)

    // Detected peaks (top-k by amplitude, from the spectrum alone).
    pub peak_freqs: Vec<f64>,
    pub peak_amps: Vec<f64>,

    // Accuracy check: max |FFT − naive DFT| over all bins.
    pub dft_max_err: f64,

    // LaTeX strings for the UI.
    pub dft_latex: String,
    pub twiddle_latex: String,
    pub result_latex: String,
}

fn error(msg: &str) -> FftResult {
    FftResult {
        ok: false,
        error: Some(msg.to_string()),
        fs: 0.0,
        n: 0,
        n_padded: 0,
        freqs_in: vec![],
        amps_in: vec![],
        phases_in: vec![],
        df: 0.0,
        nyquist: 0.0,
        ts: vec![],
        xs: vec![],
        spectrum_freqs: vec![],
        spectrum_mag: vec![],
        spectrum_amp: vec![],
        peak_freqs: vec![],
        peak_amps: vec![],
        dft_max_err: 0.0,
        dft_latex: String::new(),
        twiddle_latex: String::new(),
        result_latex: String::new(),
    }
}

/// Compact, human-readable number with at most 4 decimals.
fn fmt(v: f64) -> String {
    if v == 0.0 {
        return "0".into();
    }
    let s = format!("{:.4}", v);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Build a multi-frequency signal, FFT it from scratch, and extract its
/// spectrum.
///
/// * `fs` — sampling rate (Hz, > 0)
/// * `n` — requested sample count (clamped; zero-padded to a power of two)
/// * `freqs` / `amps` / `phases_deg` — the sinusoid components to sum
///
/// The signal is  `x[j] = Σ_i A_i·cos(2π f_i·(j/fs) + φ_i)`.
pub fn fft_spectrum(
    fs: f64,
    n: usize,
    freqs: &[f64],
    amps: &[f64],
    phases_deg: &[f64],
) -> FftResult {
    if !fs.is_finite() || fs <= 0.0 {
        return error("sampling rate must be a positive number");
    }
    if freqs.is_empty() || freqs.len() != amps.len() || freqs.len() != phases_deg.len() {
        return error("component arrays must be non-empty and equal length");
    }
    if freqs.iter().any(|f| !f.is_finite() || *f < 0.0 || *f >= fs / 2.0) {
        return error("every frequency must satisfy 0 ≤ f < fs/2 (Nyquist)");
    }

    let n = n.clamp(16, 8192);
    let n_pad = next_pow2(n);

    // --- Generate the composite time-domain signal -------------------------
    let mut ts = Vec::with_capacity(n);
    let mut xs = Vec::with_capacity(n);
    for j in 0..n {
        let t = j as f64 / fs;
        let mut acc = 0.0;
        for i in 0..freqs.len() {
            acc += amps[i] * (TWO_PI * freqs[i] * t + phases_deg[i].to_radians()).cos();
        }
        ts.push(t);
        xs.push(acc);
    }

    // --- Zero-pad and transform --------------------------------------------
    let mut buf: Vec<C64> = xs.iter().map(|&v| C64::new(v, 0.0)).collect();
    buf.resize(n_pad, C64::ZERO);
    let spectrum = match fft_radix2(&buf) {
        Ok(s) => s,
        Err(e) => return error(&e),
    };

    // Independent cross-check against the O(N²) definition.
    let reference = dft(&buf);
    let mut dft_max_err = 0.0f64;
    for (a, b) in spectrum.iter().zip(reference.iter()) {
        let d = (*a - *b).abs();
        if d > dft_max_err {
            dft_max_err = d;
        }
    }

    // --- One-sided amplitude spectrum --------------------------------------
    let half = n_pad / 2;
    let df = fs / n_pad as f64;
    let mut spectrum_freqs = Vec::with_capacity(half + 1);
    let mut spectrum_mag = Vec::with_capacity(half + 1);
    let mut spectrum_amp = Vec::with_capacity(half + 1);
    for k in 0..=half {
        let mag = spectrum[k].abs();
        spectrum_freqs.push(k as f64 * df);
        spectrum_mag.push(mag);
        // DC (k=0) and Nyquist (k=N/2) are not doubled.
        let amp = if k == 0 || k == half {
            mag / n_pad as f64
        } else {
            2.0 * mag / n_pad as f64
        };
        spectrum_amp.push(amp);
    }

    // --- Detect peaks (local maxima, no prior knowledge of the inputs) ------
    let mut peak_freqs = Vec::new();
    let mut peak_amps = Vec::new();
    let floor = 1e-9;
    for k in 1..half {
        let a = spectrum_amp[k];
        if a <= floor {
            continue;
        }
        if a >= spectrum_amp[k - 1] && a >= spectrum_amp[k + 1] {
            peak_freqs.push(k as f64 * df);
            peak_amps.push(a);
        }
    }
    // Sort by amplitude, keep the strongest few.
    let mut order: Vec<usize> = (0..peak_amps.len()).collect();
    order.sort_by(|&a, &b| peak_amps[b].partial_cmp(&peak_amps[a]).unwrap());
    let keep = freqs.len().max(1).min(order.len());
    order.truncate(keep);
    order.sort_unstable();
    peak_freqs = order.iter().map(|&i| peak_freqs[i]).collect();
    peak_amps = order.iter().map(|&i| peak_amps[i]).collect();

    // --- LaTeX --------------------------------------------------------------
    let twiddle_latex = format!(
        "W_N^k = e^{{-i\\,2\\pi k/N}} = \\cos\\left(\\tfrac{{2\\pi k}}{{N}}\\right) - i\\,\\sin\\left(\\tfrac{{2\\pi k}}{{N}}\\right)"
    );
    let dft_latex = format!(
        "X[k] = \\sum_{{n=0}}^{{N-1}} x[n]\\,W_N^{{kn}},\\quad N = {},\\; \\Delta f = {}\\ \\text{{Hz}}",
        n_pad,
        fmt(df)
    );
    let mut terms = String::new();
    for i in 0..freqs.len() {
        if i > 0 {
            terms.push_str(" + ");
        }
        terms.push_str(&format!(
            "{}\\,\\cos(2\\pi\\cdot{} t{})",
            fmt(amps[i]),
            fmt(freqs[i]),
            if phases_deg[i] == 0.0 {
                String::new()
            } else {
                format!(" + {}^\\circ", fmt(phases_deg[i]))
            }
        ));
    }
    let result_latex = format!(
        "x(t) = {},\\qquad \\text{{FFT error vs. DFT}} = {:.2e}",
        terms, dft_max_err
    );

    FftResult {
        ok: true,
        error: None,
        fs,
        n,
        n_padded: n_pad,
        freqs_in: freqs.to_vec(),
        amps_in: amps.to_vec(),
        phases_in: phases_deg.to_vec(),
        df,
        nyquist: fs / 2.0,
        ts,
        xs,
        spectrum_freqs,
        spectrum_mag,
        spectrum_amp,
        peak_freqs,
        peak_amps,
        dft_max_err,
        dft_latex,
        twiddle_latex,
        result_latex,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn fft_matches_definition() {
        let x: Vec<C64> = (0..64)
            .map(|j| C64::new((0.3 * j as f64).sin() + (0.7 * j as f64).cos(), 0.0))
            .collect();
        let f = fft_radix2(&x).unwrap();
        let d = dft(&x);
        for (a, b) in f.iter().zip(d.iter()) {
            assert!(close(a.re, b.re, 1e-9) && close(a.im, b.im, 1e-9));
        }
    }

    #[test]
    fn recovers_exact_bin_frequencies() {
        // fs = 1024, n = 512 → Δf = 2 Hz, so 50/120/300 Hz land exactly on bins.
        let r = fft_spectrum(1024.0, 512, &[50.0, 120.0, 300.0], &[1.0, 0.6, 0.25], &[0.0, 0.0, 0.0]);
        assert!(r.ok);
        assert_eq!(r.peak_freqs.len(), 3);
        assert!(close(r.peak_freqs[0], 50.0, 0.01));
        assert!(close(r.peak_freqs[1], 120.0, 0.01));
        assert!(close(r.peak_freqs[2], 300.0, 0.01));
        assert!(close(r.peak_amps[0], 1.0, 1e-6));
        assert!(close(r.peak_amps[1], 0.6, 1e-6));
        assert!(close(r.peak_amps[2], 0.25, 1e-6));
    }

    #[test]
    fn rejects_non_power_of_two() {
        let x = vec![C64::ONE; 12];
        assert!(fft_radix2(&x).is_err());
    }
}
