//! Integration tests for the calculus engine.

use calcsim_core::{parser, process, render, simplify};

fn derivative_of(input: &str) -> String {
    let e = parser::parse(input).unwrap();
    let d = simplify::simplify(&calcsim_core::diff::diff(&e));
    render::to_string(&d)
}

fn integral_of(input: &str) -> Option<String> {
    let e = parser::parse(input).unwrap();
    calcsim_core::integrate::integrate_symbolic(&e).map(|i| {
        let i = simplify::simplify(&i);
        render::to_string(&i)
    })
}

#[test]
fn diff_power() {
    assert_eq!(derivative_of("x^3"), "3*x^2");
    assert_eq!(derivative_of("x^2 + x"), "2*x + 1");
}

#[test]
fn diff_trig() {
    assert_eq!(derivative_of("sin(x)"), "cos(x)");
    assert_eq!(derivative_of("cos(x)"), "-sin(x)");
    assert_eq!(derivative_of("tan(x)"), "1/cos(x)^2");
}

#[test]
fn diff_exp_and_log() {
    assert_eq!(derivative_of("exp(x)"), "exp(x)");
    assert_eq!(derivative_of("e^x"), "e^x");
    assert_eq!(derivative_of("ln(x)"), "1/x");
}

#[test]
fn diff_product_and_chain() {
    assert_eq!(derivative_of("sin(x)*cos(x)"), "cos(x)*cos(x) + -sin(x)*sin(x)");
    assert_eq!(derivative_of("sin(x^2)"), "2*x*cos(x^2)");
}

#[test]
fn integral_polynomial() {
    assert_eq!(integral_of("x^2"), Some("x^3/3".to_string()));
    assert_eq!(integral_of("x").unwrap(), "x^2/2");
}

#[test]
fn integral_trig() {
    assert_eq!(integral_of("cos(x)"), Some("sin(x)".to_string()));
    assert_eq!(integral_of("sin(x)"), Some("-cos(x)".to_string()));
}

#[test]
fn integral_unsupported_is_none() {
    assert_eq!(integral_of("sin(x^2)"), None);
}

#[test]
fn definite_integral_value() {
    let a = process("x^2", 0.0, 1.0, 200);
    assert!(a.ok);
    let v = a.definite_value.unwrap();
    assert!((v - 1.0 / 3.0).abs() < 1e-6, "got {v}");
}

#[test]
fn parse_implicit_multiplication() {
    assert_eq!(derivative_of("2x^2"), "4*x");
    assert_eq!(derivative_of("3sin(x)"), "3*cos(x)");
}

#[test]
fn latex_basic() {
    let e = parser::parse("x^2 + sin(x)").unwrap();
    let s = render::to_latex(&e);
    assert_eq!(s, "x^{2} + \\sin\\left(x\\right)");
}

#[test]
fn bad_input_is_graceful() {
    let a = process("x + * 3", -1.0, 1.0, 100);
    assert!(!a.ok);
    assert!(a.error.is_some());
}

#[test]
fn constants_derivative_is_zero() {
    assert_eq!(derivative_of("pi"), "0");
    assert_eq!(derivative_of("5"), "0");
}

#[test]
fn ac_power_resistive() {
    // φ = 0 → pure resistive: P = Vm·Im/2 = 311·2/2 = 311 W.
    let a = calcsim_core::ac::ac_power(311.0, 2.0, 50.0, 0.0, 600);
    assert!(a.ok, "{}", a.error.unwrap_or_default());
    assert!((a.avg_power - 311.0).abs() < 0.5, "got {}", a.avg_power);
    assert!((a.power_factor - 1.0).abs() < 1e-9);
    assert!((a.rms_v - 311.0 / 2.0f64.sqrt()).abs() < 1e-6);
    assert_eq!(a.ts.len(), a.ps.len());
}

#[test]
fn ac_power_pure_reactive() {
    // φ = 90° → no real power: P ≈ 0 (reactive power only).
    let a = calcsim_core::ac::ac_power(311.0, 2.0, 60.0, 90.0, 600);
    assert!(a.ok);
    assert!(a.avg_power.abs() < 1e-6, "got {}", a.avg_power);
    assert!(a.reactive_power.abs() > 300.0);
    assert!(a.power_factor.abs() < 1e-6);
}

#[test]
fn ac_power_rejects_bad_frequency() {
    let a = calcsim_core::ac::ac_power(311.0, 2.0, 0.0, 0.0, 600);
    assert!(!a.ok);
    assert!(a.error.is_some());
}

#[test]
fn chopper_full_conduction() {
    // α=0°, β=180° → no chopping: P = Vm²/(2R) = 311²/100 ≈ 967.2 W.
    let c = calcsim_core::chopper::chopper_power(311.0, 50.0, 50.0, 0.0, 180.0, 800);
    assert!(c.ok, "{}", c.error.unwrap_or_default());
    let expected = 311.0f64.powi(2) / 100.0;
    assert!((c.avg_power - expected).abs() < 1.0, "got {}", c.avg_power);
    assert!((c.rms_v - 311.0 / 2.0f64.sqrt()).abs() < 0.5);
    assert!((c.conduction_duty - 1.0).abs() < 1e-9);
    assert_eq!(c.ts.len(), c.ps.len());
}

#[test]
fn chopper_half_conduction() {
    // α=0°, β=90° → P = Vm²/(4R) = 311²/200 ≈ 483.6 W.
    let c = calcsim_core::chopper::chopper_power(311.0, 50.0, 50.0, 0.0, 90.0, 800);
    assert!(c.ok);
    let expected = 311.0f64.powi(2) / 200.0;
    assert!((c.avg_power - expected).abs() < 1.0, "got {}", c.avg_power);
    assert!((c.conduction_duty - 0.5).abs() < 1e-9);
}

#[test]
fn chopper_rejects_bad_input() {
    // α ≥ β, f = 0, and R = 0 are all invalid.
    assert!(!calcsim_core::chopper::chopper_power(311.0, 50.0, 50.0, 150.0, 30.0, 800).ok);
    assert!(!calcsim_core::chopper::chopper_power(311.0, 50.0, 0.0, 0.0, 180.0, 800).ok);
    assert!(!calcsim_core::chopper::chopper_power(311.0, 0.0, 50.0, 0.0, 180.0, 800).ok);
}

#[test]
fn rl_ac_power_inductive() {
    // R = 50 Ω, ωL = 28.868 Ω (φ = 30°), Vm = 311 V, f = 50 Hz.
    let f = 50.0;
    let r = 50.0;
    let l = (r * 30.0f64.to_radians().tan()) / (2.0 * std::f64::consts::PI * f);
    let a = calcsim_core::rl::rl_ac_power(311.0, r, l, f, 800);
    assert!(a.ok, "{}", a.error.unwrap_or_default());
    let z = (r * r + (2.0 * std::f64::consts::PI * f * l).powi(2)).sqrt();
    let expected = 311.0f64.powi(2) * r / (2.0 * z * z);
    assert!((a.avg_power - expected).abs() < 1.0, "got {} vs {expected}", a.avg_power);
    assert!((a.power_factor - 30.0f64.to_radians().cos()).abs() < 1e-6);
    assert!(a.reactive_power > 0.0, "inductive Q should be positive");
    assert_eq!(a.ts.len(), a.ps.len());
}

#[test]
fn rl_ac_power_rejects_bad_input() {
    assert!(!calcsim_core::rl::rl_ac_power(311.0, 50.0, 0.0, 50.0, 800).ok); // L = 0
    assert!(!calcsim_core::rl::rl_ac_power(311.0, 0.0, 0.1, 50.0, 800).ok); // R = 0
    assert!(!calcsim_core::rl::rl_ac_power(311.0, 50.0, 0.1, 0.0, 800).ok); // f = 0
}

#[test]
fn rl_chopper_continuous_conduction() {
    // α = 10° < φ = 30° → CCM: output equals full sine, PF = cos φ.
    let f = 50.0;
    let r = 50.0;
    let l = (r * 30.0f64.to_radians().tan()) / (2.0 * std::f64::consts::PI * f);
    let c = calcsim_core::rl::rl_chopper(311.0, r, l, f, 10.0, 800);
    assert!(c.ok, "{}", c.error.unwrap_or_default());
    assert!(c.continuous);
    let z = (r * r + (2.0 * std::f64::consts::PI * f * l).powi(2)).sqrt();
    let expected = 311.0f64.powi(2) * r / (2.0 * z * z);
    assert!((c.avg_power - expected).abs() < 1.0, "got {} vs {expected}", c.avg_power);
    assert!((c.power_factor - 30.0f64.to_radians().cos()).abs() < 1e-6);
}

#[test]
fn rl_chopper_discontinuous_conduction() {
    // α = 90° > φ = 30° → DCM: extinction angle β′ > 180°, power less than full.
    let f = 50.0;
    let r = 50.0;
    let l = (r * 30.0f64.to_radians().tan()) / (2.0 * std::f64::consts::PI * f);
    let c = calcsim_core::rl::rl_chopper(311.0, r, l, f, 90.0, 800);
    assert!(c.ok);
    assert!(!c.continuous);
    assert!(c.extinction_deg > 180.0 && c.extinction_deg < 270.0, "β′ = {}", c.extinction_deg);
    let z = (r * r + (2.0 * std::f64::consts::PI * f * l).powi(2)).sqrt();
    let full = 311.0f64.powi(2) * r / (2.0 * z * z);
    assert!(c.avg_power > 0.0 && c.avg_power < full, "got {} vs full {full}", c.avg_power);
}

#[test]
fn rl_chopper_rejects_bad_input() {
    assert!(!calcsim_core::rl::rl_chopper(311.0, 50.0, 0.1, 50.0, 200.0, 800).ok); // α > 180
    assert!(!calcsim_core::rl::rl_chopper(311.0, 50.0, 0.0, 50.0, 90.0, 800).ok); // L = 0
}
