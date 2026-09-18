//! Space shuttle landing — glide, flare and touchdown simulation.
//!
//! # Mathematical foundation
//!
//! ## Point-mass flight dynamics (vertical plane, wind/velocity frame)
//!
//! Let `v` be speed and `γ` the flight-path angle (positive = climb). Resolving
//! lift `L`, drag `D` and weight `mg`:
//!
//! ```text
//! m v̇   = −D − m g sin γ
//! m v γ̇ = L − m g cos γ
//! ẋ     = v cos γ          (downrange)
//! ẏ     = v sin γ          (altitude)
//! ```
//!
//! with `α = θ − γ` the angle of attack (`θ` = pitch attitude). Aerodynamics:
//!
//! ```text
//! L = ½ ρ v² S C_L ,   D = ½ ρ v² S C_D ,   C_L = C_L0 + C_Lα·α ,
//! C_D = C_D0 + k·C_L² ,   ρ(y) = ρ₀ e^{−y/H}
//! ```
//!
//! plus **ground effect** near the surface: `C_L ← C_L(1 + k_ge e^{−y/h_ge})`.
//!
//! ## The flare — a commanded flight-path schedule
//!
//! Before the flare the shuttle flies a constant approach angle `γ₀`. Below the
//! flare altitude it follows a rounded-out path that drives the flight-path
//! angle from `γ₀` to a shallow touchdown value `γ_td`, while the angle of
//! attack ramps from its trim value `α_glide` to `α_flare`:
//!
//! ```text
//! r(y)  = 1 − (y / y_f)²                     (0 at flare entry → 1 at ground)
//! γ_cmd = γ₀  + (γ_td  − γ₀)·r(y)
//! α_cmd = α_glide + (α_flare − α_glide)·r(y)
//! θ_cmd = γ_cmd + α_cmd
//! ```
//!
//! The quadratic `r(y)` changes fastest at flare entry (a prompt round-out) and
//! flattens near the ground (a gentle, controlled final descent).
//!
//! A proportional-only path controller would leave a steady-state error, since
//! the trim angle of attack that balances weight rises as the shuttle bleeds
//! speed in the flare.  The simulation therefore uses **PI control**:
//!
//! ```text
//! α_cmd = α_glide + K_P(γ_cmd − γ) + K_I ∫(γ_cmd − γ) dt
//! ```
//!
//! with anti-windup (the integrator freezes while `α` saturates).
//!
//! ## Attitude / oscillatory rotational dynamics — Euler's phasor
//!
//! The pitch attitude tracks `θ_cmd` via the second-order *short-period* mode
//!
//! ```text
//! θ̈ + 2ζωₙ θ̇ + ωₙ² θ = ωₙ² θ_cmd
//! ```
//!
//! whose characteristic roots are
//!
//! ```text
//! λ = −ζωₙ ± i ωₙ√(1−ζ²) = σ ± iω_d
//! ```
//!
//! For `ζ < 1` these are **complex**, and the homogeneous solution is the real
//! part of a decaying **phasor** (Euler's formula in its `e^{λt}` form):
//!
//! ```text
//! θ_h(t) = Re{ Θ e^{(σ + iω_d) t} } = |Θ| e^{σt} cos(ω_d t + φ)
//! ```
//!
//! ## Numerical integration
//!
//! The 7-D first-order system `d(state)/dt = f(state)` (two position, speed,
//! flight-path angle, pitch, pitch rate and the controller integrator) is
//! integrated with Euler (O(h)), Heun's method (O(h²)) and classical RK4
//! (O(h⁴)); RK4 is the reference trajectory, the other two demonstrate
//! truncation-error behaviour.

use serde::Serialize;

const G: f64 = 9.80665;
const RHO_0: f64 = 1.225;
const SCALE_HEIGHT: f64 = 8400.0;
const PI: f64 = std::f64::consts::PI;

/// Vehicle geometry, mass and aerodynamics (shuttle-class, fixed for the UI).
const MASS: f64 = 90_000.0;
const WING_AREA: f64 = 250.0;
const CL0: f64 = 0.05;      // zero-α lift coefficient
const CL_ALPHA: f64 = 4.0;  // lift-curve slope, per radian
const CD0: f64 = 0.04;      // parasitic drag
const K: f64 = 0.07;        // induced-drag factor  (C_D = C_D0 + k C_L²)
const PITCH_WN: f64 = 3.5;  // short-period natural frequency [rad/s]
const PITCH_ZETA: f64 = 0.75; // damping ratio (< 1 → oscillatory)
const K_GE: f64 = 0.25;     // ground-effect lift gain
const H_GE: f64 = 30.0;     // ground-effect height [m]

const ALPHA_GLIDE: f64 = 4.0_f64.to_radians(); // trim AoA during approach
const GAMMA_TD: f64 = -0.5_f64.to_radians();   // target touchdown flight-path angle
const KP: f64 = 4.0;               // flight-path proportional gain (rad/rad)
const KI: f64 = 0.6;               // flight-path integral gain (rad/rad/s)
const ALPHA_MIN: f64 = -8.0_f64.to_radians(); // max nose-down α (anti-balloon)

const WHEEL_HEIGHT: f64 = 3.0;   // main-gear altitude at touchdown [m]
const MAX_SINK: f64 = 1.5;       // safe sink-rate threshold [m/s]
const MIN_SPEED: f64 = 60.0;     // [m/s]
const MAX_SPEED: f64 = 130.0;    // [m/s]
const MAX_PITCH: f64 = 25.0_f64.to_radians(); // [rad]

/// A single 7-element state vector `[y, x, v, gamma, theta, q, err_i]`.
///
/// The extra `err_i` element is the accumulated flight-path error
/// `∫(γ_cmd − γ) dt`, giving the controller integral action (see below).
type State = [f64; 7];
const Y: usize = 0;
const X: usize = 1;
const V: usize = 2;
const GAMMA: usize = 3;
const THETA: usize = 4;
const Q: usize = 5;
const EI: usize = 6;

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct LandingCheck {
    pub name: String,
    pub pass: bool,
    pub value: f64,
    pub limit: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MethodResult {
    pub name: String,
    pub t_touch: f64,
    pub x_touch: f64,
    pub sink_rate: f64,
    pub v_touch: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShuttleSim {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    // Inputs (echoed back).
    pub h0: f64,
    pub v0: f64,
    pub gamma0_deg: f64,
    pub flare_alt: f64,
    pub alpha_flare_deg: f64,

    // Short-period phasor (Euler's formula for the oscillatory mode).
    pub sigma: f64,
    pub omega_d: f64,
    pub period_d: f64,

    // Landing conditions.
    pub t_touch: f64,
    pub x_touch: f64,
    pub v_touch: f64,
    pub sink_rate: f64,
    pub gamma_touch_deg: f64,
    pub alpha_touch_deg: f64,
    pub theta_touch_deg: f64,
    pub safe: bool,
    pub checks: Vec<LandingCheck>,

    // Integrator comparison (Euler / Heun / RK4).
    pub methods: Vec<MethodResult>,

    // Trajectory (RK4).
    pub ts: Vec<f64>,
    pub xs: Vec<f64>,
    pub ys: Vec<f64>,
    pub vs: Vec<f64>,
    pub gammas: Vec<f64>,
    pub alphas: Vec<f64>,
    pub sinks: Vec<f64>,

    // LaTeX.
    pub eom_latex: String,
    pub phasor_latex: String,
}

fn error(msg: &str) -> ShuttleSim {
    ShuttleSim {
        ok: false,
        error: Some(msg.to_string()),
        h0: 0.0,
        v0: 0.0,
        gamma0_deg: 0.0,
        flare_alt: 0.0,
        alpha_flare_deg: 0.0,
        sigma: 0.0,
        omega_d: 0.0,
        period_d: 0.0,
        t_touch: 0.0,
        x_touch: 0.0,
        v_touch: 0.0,
        sink_rate: 0.0,
        gamma_touch_deg: 0.0,
        alpha_touch_deg: 0.0,
        theta_touch_deg: 0.0,
        safe: false,
        checks: vec![],
        methods: vec![],
        ts: vec![],
        xs: vec![],
        ys: vec![],
        vs: vec![],
        gammas: vec![],
        alphas: vec![],
        sinks: vec![],
        eom_latex: String::new(),
        phasor_latex: String::new(),
    }
}

fn fmt(v: f64) -> String {
    if v == 0.0 {
        return "0".into();
    }
    let s = format!("{:.4}", v);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

// ---------------------------------------------------------------------------
// Dynamics
// ---------------------------------------------------------------------------

fn air_density(y: f64) -> f64 {
    RHO_0 * (-y.max(0.0) / SCALE_HEIGHT).exp()
}

/// Commanded flight-path angle from the flare schedule (a rounded-out path).
fn gamma_cmd(y: f64, flare_alt: f64, gamma0: f64) -> f64 {
    if y >= flare_alt {
        gamma0
    } else {
        let r = 1.0 - (y / flare_alt).powi(2); // 0 → 1 as y: flare_alt → 0
        gamma0 + (GAMMA_TD - gamma0) * r
    }
}

fn derivatives(st: &State, flare_alt: f64, gamma0: f64, alpha_flare: f64) -> State {
    let y = st[Y];
    let v = st[V];
    let gamma = st[GAMMA];
    let theta = st[THETA];
    let q = st[Q];
    let ei = st[EI];

    let rho = air_density(y);
    let g_cmd = gamma_cmd(y, flare_alt, gamma0);

    // Flight-path PI controller.  The proportional term gives the immediate
    // round-out; the integral term cancels the steady-state γ error that a
    // pure proportional law leaves when the approach speed bleeds off in the
    // flare (the trim AoA rises, and the integral term supplies it).
    let err = g_cmd - gamma;
    let alpha_unclamped = ALPHA_GLIDE + KP * err + KI * ei;
    let alpha_cmd = alpha_unclamped.clamp(ALPHA_MIN, alpha_flare);
    // Anti-windup: freeze the integrator while the surface is saturated.
    let ei_dot = if alpha_unclamped == alpha_cmd { err } else { 0.0 };

    let alpha = theta - gamma;

    // Aerodynamic coefficients + ground effect.
    let cl_clean = CL0 + CL_ALPHA * alpha;
    let ge = 1.0 + K_GE * (-y.max(0.0) / H_GE).exp();
    let cl = cl_clean * ge;
    let cd = CD0 + K * cl_clean * cl_clean;

    let qdyn = 0.5 * rho * v * v * WING_AREA;
    let lift = qdyn * cl;
    let drag = qdyn * cd;

    // Translational derivatives (unpowered glide: T = 0).
    let v_dot = -drag / MASS - G * gamma.sin();
    let gamma_dot = lift / (MASS * v) - G * gamma.cos() / v;
    let x_dot = v * gamma.cos();
    let y_dot = v * gamma.sin();

    // Rotational derivative: pitch tracks θ_cmd = γ + α_cmd so that α → α_cmd.
    let theta_cmd = gamma + alpha_cmd;
    let q_dot = PITCH_WN * PITCH_WN * (theta_cmd - theta) - 2.0 * PITCH_ZETA * PITCH_WN * q;

    [y_dot, x_dot, v_dot, gamma_dot, q, q_dot, ei_dot]
}

// ---------------------------------------------------------------------------
// Numerical integrators
// ---------------------------------------------------------------------------

fn scale(s: &State, h: f64) -> State {
    [
        s[0] * h,
        s[1] * h,
        s[2] * h,
        s[3] * h,
        s[4] * h,
        s[5] * h,
        s[6] * h,
    ]
}

fn add(a: &State, b: &State) -> State {
    [
        a[0] + b[0],
        a[1] + b[1],
        a[2] + b[2],
        a[3] + b[3],
        a[4] + b[4],
        a[5] + b[5],
        a[6] + b[6],
    ]
}

fn euler_step(st: &State, dt: f64, fa: f64, g0: f64, af: f64) -> State {
    let k1 = derivatives(st, fa, g0, af);
    add(st, &scale(&k1, dt))
}

fn heun_step(st: &State, dt: f64, fa: f64, g0: f64, af: f64) -> State {
    let k1 = derivatives(st, fa, g0, af);
    let p = add(st, &scale(&k1, dt));
    let k2 = derivatives(&p, fa, g0, af);
    let mut m = [0.0; 7];
    for i in 0..7 {
        m[i] = (k1[i] + k2[i]) * dt / 2.0;
    }
    add(st, &m)
}

fn rk4_step(st: &State, dt: f64, fa: f64, g0: f64, af: f64) -> State {
    let k1 = derivatives(st, fa, g0, af);
    let p2 = add(st, &scale(&k1, dt / 2.0));
    let k2 = derivatives(&p2, fa, g0, af);
    let p3 = add(st, &scale(&k2, dt / 2.0));
    let k3 = derivatives(&p3, fa, g0, af);
    let p4 = add(st, &scale(&k3, dt));
    let k4 = derivatives(&p4, fa, g0, af);
    let mut m = [0.0; 7];
    for i in 0..7 {
        m[i] = (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]) * dt / 6.0;
    }
    add(st, &m)
}

// ---------------------------------------------------------------------------
// Simulation driver
// ---------------------------------------------------------------------------

type StepFn = fn(&State, f64, f64, f64, f64) -> State;

/// Run one landing from an initial state with the given integrator, returning
/// (trajectory, touchdown state).
fn run(
    h0: f64,
    v0: f64,
    gamma0: f64,
    flare_alt: f64,
    alpha_flare: f64,
    dt: f64,
    step: StepFn,
) -> (Vec<State>, State) {
    let mut st: State = [h0, 0.0, v0, gamma0, gamma0 + ALPHA_GLIDE, 0.0, 0.0];
    let mut traj: Vec<State> = Vec::new();
    traj.push(st);
    let mut t = 0.0;

    while st[Y] > WHEEL_HEIGHT {
        st = step(&st, dt, flare_alt, gamma0, alpha_flare);
        t += dt;
        traj.push(st);
        if t > 1200.0 {
            break;
        }
    }

    // Interpolate the last step to the exact touchdown altitude.
    let n = traj.len();
    let (prev, now) = (traj[n - 2], traj[n - 1]);
    let frac = if prev[Y] != now[Y] {
        (prev[Y] - WHEEL_HEIGHT) / (prev[Y] - now[Y])
    } else {
        0.0
    };
    let mut td = [0.0; 7];
    for i in 0..7 {
        td[i] = prev[i] + frac * (now[i] - prev[i]);
    }
    td[Y] = WHEEL_HEIGHT;

    (traj, td)
}

/// Full space shuttle landing simulation.
///
/// * `h0` — initial altitude (m)
/// * `v0` — initial speed (m/s)
/// * `gamma0_deg` — approach flight-path angle (deg, negative = descending)
/// * `flare_alt` — altitude below which the flare begins (m)
/// * `alpha_flare_deg` — maximum flare angle of attack (deg)
pub fn shuttle_landing(
    h0: f64,
    v0: f64,
    gamma0_deg: f64,
    flare_alt: f64,
    alpha_flare_deg: f64,
) -> ShuttleSim {
    if !h0.is_finite() || h0 <= WHEEL_HEIGHT {
        return error("initial altitude must be > 3 m");
    }
    if !v0.is_finite() || v0 <= 0.0 {
        return error("initial speed must be a positive number");
    }
    if !gamma0_deg.is_finite() || gamma0_deg >= 0.0 || gamma0_deg < -89.0 {
        return error("glide-path angle must be in (−89°, 0°)");
    }
    if !flare_alt.is_finite() || flare_alt <= 0.0 || flare_alt >= h0 {
        return error("flare altitude must be between 0 and the initial altitude");
    }
    if !alpha_flare_deg.is_finite() || alpha_flare_deg <= 0.0 || alpha_flare_deg >= 45.0 {
        return error("flare angle of attack must be in (0°, 45°)");
    }

    let gamma0 = gamma0_deg.to_radians();
    let alpha_flare = alpha_flare_deg.to_radians();

    // Short-period phasor parameters (Euler's formula).
    let sigma = -PITCH_ZETA * PITCH_WN;
    let omega_d = PITCH_WN * (1.0 - PITCH_ZETA * PITCH_ZETA).sqrt();
    let period_d = 2.0 * PI / omega_d;

    // Reference trajectory with RK4.
    let dt = 0.02;
    let (traj, td) = run(h0, v0, gamma0, flare_alt, alpha_flare, dt, rk4_step);

    // Landed telemetry.
    let v_touch = td[V];
    let gamma_touch = td[GAMMA];
    let theta_touch = td[THETA];
    let alpha_touch = theta_touch - gamma_touch;
    let sink = -v_touch * gamma_touch.sin();

    // Safety checks.
    let checks = vec![
        LandingCheck {
            name: "sink rate".to_string(),
            pass: sink <= MAX_SINK,
            value: sink,
            limit: MAX_SINK,
        },
        LandingCheck {
            name: "touchdown speed".to_string(),
            pass: v_touch >= MIN_SPEED && v_touch <= MAX_SPEED,
            value: v_touch,
            limit: MAX_SPEED,
        },
        LandingCheck {
            name: "pitch attitude".to_string(),
            pass: theta_touch.abs() <= MAX_PITCH,
            value: theta_touch.abs().to_degrees(),
            limit: MAX_PITCH.to_degrees(),
        },
        LandingCheck {
            name: "downrange (forward)".to_string(),
            pass: td[X] > 0.0,
            value: td[X],
            limit: 0.0,
        },
    ];
    let safe = checks.iter().all(|c| c.pass);

    // Integrator comparison at a deliberately coarse step (0.2 s) so that the
    // truncation-error behaviour of the three methods is visible: RK4 (O(h⁴))
    // stays closest to the converged dt=0.02 reference, Heun (O(h²)) next,
    // Euler (O(h)) furthest.
    let dt_coarse = 0.2;
    let methods: Vec<MethodResult> = [
        ("euler", euler_step as StepFn),
        ("heun", heun_step),
        ("rk4", rk4_step),
    ]
    .into_iter()
    .map(|(name, step)| {
        let (traj_m, td_m) = run(h0, v0, gamma0, flare_alt, alpha_flare, dt_coarse, step);
        MethodResult {
            name: name.to_string(),
            t_touch: (traj_m.len() - 1) as f64 * dt_coarse,
            x_touch: td_m[X],
            sink_rate: -td_m[V] * td_m[GAMMA].sin(),
            v_touch: td_m[V],
        }
    })
    .collect();

    // Trajectory arrays for the frontend.
    let mut ts = Vec::with_capacity(traj.len());
    let mut xs = Vec::with_capacity(traj.len());
    let mut ys = Vec::with_capacity(traj.len());
    let mut vs = Vec::with_capacity(traj.len());
    let mut gammas = Vec::with_capacity(traj.len());
    let mut alphas = Vec::with_capacity(traj.len());
    let mut sinks = Vec::with_capacity(traj.len());
    for (i, s) in traj.iter().enumerate() {
        ts.push(i as f64 * dt);
        ys.push(s[Y]);
        xs.push(s[X]);
        vs.push(s[V]);
        gammas.push(s[GAMMA].to_degrees());
        alphas.push((s[THETA] - s[GAMMA]).to_degrees());
        sinks.push(-s[V] * s[GAMMA].sin());
    }

    let eom_latex =
        "m\\dot{v} = -D - mg\\sin\\gamma,\\quad mv\\dot{\\gamma} = L - mg\\cos\\gamma".to_string();
    let phasor_latex = format!(
        "\\lambda = {}\\, \\pm\\, i\\,{}\\,\\Rightarrow\\; \\theta_h(t) = |\\Theta|\\,e^{{{}t}}\\cos({}\\,t + \\phi)",
        fmt(sigma),
        fmt(omega_d),
        fmt(sigma),
        fmt(omega_d)
    );

    ShuttleSim {
        ok: true,
        error: None,
        h0,
        v0,
        gamma0_deg,
        flare_alt,
        alpha_flare_deg,
        sigma,
        omega_d,
        period_d,
        t_touch: (traj.len() - 1) as f64 * dt,
        x_touch: td[X],
        v_touch,
        sink_rate: sink,
        gamma_touch_deg: gamma_touch.to_degrees(),
        alpha_touch_deg: alpha_touch.to_degrees(),
        theta_touch_deg: theta_touch.to_degrees(),
        safe,
        checks,
        methods,
        ts,
        xs,
        ys,
        vs,
        gammas,
        alphas,
        sinks,
        eom_latex,
        phasor_latex,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_landing_is_safe() {
        let r = shuttle_landing(3000.0, 135.0, -12.0, 350.0, 10.0);
        assert!(r.ok);
        assert!(r.safe, "landing must be within thresholds: {:#?}", r.checks);
        assert!(r.sink_rate >= 0.0 && r.sink_rate <= MAX_SINK);
        assert!(r.v_touch >= MIN_SPEED && r.v_touch <= MAX_SPEED);
        assert!(r.x_touch > 0.0);
    }

    #[test]
    fn phasor_roots_are_complex_when_underdamped() {
        let r = shuttle_landing(3000.0, 135.0, -12.0, 350.0, 10.0);
        assert!(r.sigma < 0.0); // decaying
        assert!(r.omega_d > 0.0); // oscillatory (ζ < 1)
    }

    #[test]
    fn rejects_bad_inputs() {
        assert!(!shuttle_landing(1.0, 140.0, -13.0, 100.0, 12.0).ok); // h0 too low
        assert!(!shuttle_landing(3000.0, 140.0, 0.0, 500.0, 12.0).ok); // climbing γ
        assert!(!shuttle_landing(3000.0, 140.0, -13.0, 5000.0, 12.0).ok); // flare above h0
    }
}
