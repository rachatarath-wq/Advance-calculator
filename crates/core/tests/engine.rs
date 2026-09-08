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
