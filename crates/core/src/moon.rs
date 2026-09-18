//! Moon mission — launch, Earth→Moon transfer, lunar swing-by and return to a
//! targeted Earth landing site.
//!
//! # Model
//!
//! A point-mass shuttle in the Earth–Moon plane. Earth is fixed at the origin
//! and the Moon at `(D, 0)`; the shuttle is accelerated by the inverse-square
//! gravity of **both** bodies plus a thrust controller that flies a chain of
//! waypoints (a velocity-pursuit law):
//!
//! ```text
//! \dot{\vec r} = \vec v
//! \dot{\vec v} = -μ_E \vec r/|r|³ - μ_M (\vec r-\vec r_M)/|\vec r-\vec r_M|³ + \vec a_T
//! \vec a_T = k (\vec v_des - \vec v),   |\vec a_T| ≤ a_max,
//! \vec v_des = s_i · (w_i - \vec r) / |w_i - \vec r|
//! ```
//!
//! where `w_i` is the current waypoint and `s_i` its target speed. When the
//! shuttle comes within the waypoint's capture radius the target advances to
//! the next, so the path is a launch → transfer → lunar swing-by → return →
//! landing loop. Integrated with RK4.

use serde::Serialize;

const R_E: f64 = 6371.0; // Earth radius [km]
const R_M: f64 = 1737.0; // Moon radius [km]
const D_EM: f64 = 384_400.0; // Earth–Moon distance [km]
const MU_E: f64 = 398_600.0; // Earth μ [km³/s²]
const MU_M: f64 = 4_903.0; // Moon μ [km³/s²]
const G0: f64 = 9.80665e-3; // Earth surface gravity [km/s²]
const K_PURSUIT: f64 = 0.12; // velocity-pursuit gain [1/s]
const SOFT: f64 = 100.0; // gravity softening radius [km] (avoids the singularity)
const DT: f64 = 1.0; // RK4 step [s]
const MAX_STEPS: usize = 200_000; // ≈ 55 h of simulated flight
const OUT_EVERY: usize = 40; // subsample telemetry (≈ 3k points over a full run)

type State = [f64; 4];
const X: usize = 0;
const Y: usize = 1;
const VX: usize = 2;
const VY: usize = 3;

/// A waypoint the shuttle flies through: position, target speed, capture radius
/// and a human-readable phase label.
struct Waypoint {
    x: f64,
    y: f64,
    speed: f64,
    capture: f64,
    phase: &'static str,
}

fn waypoints(target_rad: f64) -> Vec<Waypoint> {
    let tx = R_E * target_rad.cos();
    let ty = R_E * target_rad.sin();
    vec![
        Waypoint { x: R_E, y: 0.0, speed: 0.0, capture: 0.0, phase: "Launch site" },
        Waypoint { x: 2.5 * R_E, y: 2.0 * R_E, speed: 3.0, capture: 2.0 * R_E, phase: "Launch / ascent" },
        Waypoint { x: 5.0 * R_E, y: 5.0 * R_E, speed: 6.0, capture: 2.5 * R_E, phase: "Ascent" },
        Waypoint { x: 12.0 * R_E, y: 9.0 * R_E, speed: 8.0, capture: 3.0 * R_E, phase: "Escape Earth" },
        Waypoint { x: 40.0 * R_E, y: 14.0 * R_E, speed: 10.0, capture: 4.0 * R_E, phase: "Transit to Moon" },
        Waypoint { x: D_EM * 0.72, y: 8.0 * R_E, speed: 9.0, capture: 3.5 * R_E, phase: "Moon approach" },
        Waypoint { x: D_EM - 2.0 * R_M, y: 2.0 * R_M, speed: 5.0, capture: 2.5 * R_M, phase: "Lunar swing-by" },
        Waypoint { x: D_EM + 5.0 * R_M, y: -3.0 * R_M, speed: 5.0, capture: 2.0 * R_M, phase: "Lunar swing-by" },
        Waypoint { x: D_EM + 1.0 * R_M, y: -6.0 * R_M, speed: 5.0, capture: 2.0 * R_M, phase: "Return to Earth" },
        Waypoint { x: D_EM * 0.75, y: -8.0 * R_E, speed: 9.0, capture: 3.5 * R_E, phase: "Return to Earth" },
        Waypoint { x: 30.0 * R_E, y: -14.0 * R_E, speed: 10.0, capture: 3.5 * R_E, phase: "Return to Earth" },
        Waypoint { x: 5.0 * R_E, y: -3.0 * R_E, speed: 4.0, capture: 2.0 * R_E, phase: "Approach & landing" },
        Waypoint { x: tx, y: ty, speed: 2.5, capture: 0.6 * R_E, phase: "Landed" },
    ]
}

// ---------------------------------------------------------------------------
// Dynamics
// ---------------------------------------------------------------------------

fn gravity(x: f64, y: f64) -> [f64; 2] {
    let re = (x * x + y * y + SOFT * SOFT).powf(1.5);
    let ax_e = -MU_E * x / re;
    let ay_e = -MU_E * y / re;
    let dx = x - D_EM;
    let dy = y;
    let rm = (dx * dx + dy * dy + SOFT * SOFT).powf(1.5);
    let ax_m = -MU_M * dx / rm;
    let ay_m = -MU_M * dy / rm;
    [ax_e + ax_m, ay_e + ay_m]
}

/// Velocity-pursuit thrust, clamped to the vehicle's maximum acceleration.
fn thrust(st: &State, wp: &Waypoint, a_max: f64) -> [f64; 2] {
    let dx = wp.x - st[X];
    let dy = wp.y - st[Y];
    let dist = (dx * dx + dy * dy).sqrt().max(1.0);
    let ax = K_PURSUIT * (wp.speed * dx / dist - st[VX]);
    let ay = K_PURSUIT * (wp.speed * dy / dist - st[VY]);
    let mag = (ax * ax + ay * ay).sqrt();
    if mag > a_max {
        let s = a_max / mag;
        [ax * s, ay * s]
    } else {
        [ax, ay]
    }
}

fn thrust_mag(st: &State, wp: &Waypoint, a_max: f64) -> f64 {
    let t = thrust(st, wp, a_max);
    (t[0] * t[0] + t[1] * t[1]).sqrt()
}

fn deriv(st: &State, wp: &Waypoint, a_max: f64) -> State {
    let g = gravity(st[X], st[Y]);
    let t = thrust(st, wp, a_max);
    [st[VX], st[VY], g[0] + t[0], g[1] + t[1]]
}

fn add(a: &State, b: &State) -> State {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2], a[3] + b[3]]
}

fn scale(s: &State, h: f64) -> State {
    [s[0] * h, s[1] * h, s[2] * h, s[3] * h]
}

fn rk4(st: &State, dt: f64, wp: &Waypoint, a_max: f64) -> State {
    let k1 = deriv(st, wp, a_max);
    let p2 = add(st, &scale(&k1, dt / 2.0));
    let k2 = deriv(&p2, wp, a_max);
    let p3 = add(st, &scale(&k2, dt / 2.0));
    let k3 = deriv(&p3, wp, a_max);
    let p4 = add(st, &scale(&k3, dt));
    let k4 = deriv(&p4, wp, a_max);
    let mut m = [0.0; 4];
    for i in 0..4 {
        m[i] = (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]) * dt / 6.0;
    }
    add(st, &m)
}

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct MoonPhase {
    pub name: String,
    pub t: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MoonMission {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    // Inputs (echoed back).
    pub target_deg: f64,
    pub thrust_g: f64,

    // Mission outcome.
    pub landed: bool,
    pub total_time_h: f64,
    pub max_speed_km_s: f64,
    pub max_distance_km: f64,

    // Geometry for the plot.
    pub earth_r: f64,
    pub moon_r: f64,
    pub moon_x: f64,

    // Phase schedule (name + start time, sorted).
    pub phases: Vec<MoonPhase>,
    pub waypoints_x: Vec<f64>,
    pub waypoints_y: Vec<f64>,

    // Trajectory (subsampled).
    pub ts: Vec<f64>,
    pub xs: Vec<f64>,
    pub ys: Vec<f64>,
    pub speeds: Vec<f64>,
    pub thrusts: Vec<f64>,

    pub eom_latex: String,
}

fn error(msg: &str) -> MoonMission {
    MoonMission {
        ok: false,
        error: Some(msg.to_string()),
        target_deg: 0.0,
        thrust_g: 0.0,
        landed: false,
        total_time_h: 0.0,
        max_speed_km_s: 0.0,
        max_distance_km: 0.0,
        earth_r: R_E,
        moon_r: R_M,
        moon_x: D_EM,
        phases: vec![],
        waypoints_x: vec![],
        waypoints_y: vec![],
        ts: vec![],
        xs: vec![],
        ys: vec![],
        speeds: vec![],
        thrusts: vec![],
        eom_latex: String::new(),
    }
}

// ---------------------------------------------------------------------------
// Simulation driver
// ---------------------------------------------------------------------------

/// Simulate a full Moon mission.
///
/// * `target_deg` — landing-site angle around Earth, measured from the +x axis
///   (0° = the launch site on the right side), degrees.
/// * `thrust_g` — maximum thrust in units of Earth surface gravity.
pub fn moon_mission(target_deg: f64, thrust_g: f64) -> MoonMission {
    if !target_deg.is_finite() {
        return error("target angle must be a finite number");
    }
    if !thrust_g.is_finite() || thrust_g <= 0.0 || thrust_g > 20.0 {
        return error("thrust must be in (0, 20] g");
    }

    let a_max = thrust_g * G0;
    let wps = waypoints(target_deg.to_radians());
    let n = wps.len();

    let mut st: State = [wps[0].x, wps[0].y, 0.0, 0.0];
    let mut active = 1usize;
    let mut phase_starts = vec![f64::INFINITY; n];
    phase_starts[0] = 0.0;
    phase_starts[1] = 0.0;

    let mut ts = Vec::new();
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    let mut speeds = Vec::new();
    let mut thrusts = Vec::new();

    let mut max_speed = 0.0f64;
    let mut max_dist = 0.0f64;
    let mut t = 0.0;
    let mut landed = false;

    for step in 0..MAX_STEPS {
        if step % OUT_EVERY == 0 {
            let speed = (st[VX] * st[VX] + st[VY] * st[VY]).sqrt();
            let dist = (st[X] * st[X] + st[Y] * st[Y]).sqrt();
            if speed > max_speed {
                max_speed = speed;
            }
            if dist > max_dist {
                max_dist = dist;
            }
            ts.push(t);
            xs.push(st[X]);
            ys.push(st[Y]);
            speeds.push(speed);
            thrusts.push(thrust_mag(&st, &wps[active], a_max) / G0);
        }

        // Advance the waypoint once it is captured.
        let wp = &wps[active];
        let dx = wp.x - st[X];
        let dy = wp.y - st[Y];
        if (dx * dx + dy * dy).sqrt() < wp.capture {
            if active == n - 1 {
                landed = true;
                break;
            }
            active += 1;
            phase_starts[active] = t;
        }

        st = rk4(&st, DT, &wps[active], a_max);
        t += DT;
    }

    let phases: Vec<MoonPhase> = wps
        .iter()
        .enumerate()
        .filter(|(i, _)| phase_starts[*i].is_finite())
        .map(|(i, w)| MoonPhase {
            name: w.phase.to_string(),
            t: phase_starts[i],
        })
        .collect();

    let waypoints_x: Vec<f64> = wps.iter().map(|w| w.x).collect();
    let waypoints_y: Vec<f64> = wps.iter().map(|w| w.y).collect();

    let eom_latex =
        "\\dot{\\vec r}=\\vec v,\\;\\; \\dot{\\vec v} = -\\mu_E\\frac{\\vec r}{|\\vec r|^3} - \\mu_M\\frac{\\vec r-\\vec r_M}{|\\vec r-\\vec r_M|^3} + \\vec a_T"
            .to_string();

    MoonMission {
        ok: true,
        error: None,
        target_deg,
        thrust_g,
        landed,
        total_time_h: t / 3600.0,
        max_speed_km_s: max_speed,
        max_distance_km: max_dist,
        earth_r: R_E,
        moon_r: R_M,
        moon_x: D_EM,
        phases,
        waypoints_x,
        waypoints_y,
        ts,
        xs,
        ys,
        speeds,
        thrusts,
        eom_latex,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completes_mission() {
        let r = moon_mission(0.0, 4.0);
        assert!(r.ok);
        assert!(r.landed, "mission must reach the landing site");
        assert!(r.ts.len() > 500, "expected a substantial trajectory");
        assert!(r.max_speed_km_s > 3.0, "must reach orbital-class speeds");
        assert!(r.max_distance_km > D_EM * 0.85, "must reach the Moon");
    }

    #[test]
    fn all_finite() {
        let r = moon_mission(120.0, 4.0);
        for &v in r.xs.iter().chain(&r.ys).chain(&r.speeds).chain(&r.thrusts) {
            assert!(v.is_finite());
        }
    }

    #[test]
    fn respects_thrust_limit() {
        let r = moon_mission(0.0, 3.0);
        for &th in &r.thrusts {
            assert!(th <= 3.0 + 1e-9, "thrust {th} exceeds the 3 g limit");
        }
    }

    #[test]
    fn rejects_bad_inputs() {
        assert!(!moon_mission(f64::NAN, 4.0).ok);
        assert!(!moon_mission(0.0, -1.0).ok);
        assert!(!moon_mission(0.0, 0.0).ok);
        assert!(!moon_mission(0.0, 25.0).ok);
    }
}
