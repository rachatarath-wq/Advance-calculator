//! Turning expressions into plottable (x, y) samples, with NaN gaps so the
//! frontend can break lines at asymptotes / undefined regions.

use crate::ast::Expr;
use crate::eval::eval;

/// Clip values this far from zero so vertical asymptotes don't draw a solid
/// line across the plot. Anything beyond is emitted as NaN (a gap).
pub const Y_CAP: f64 = 1e6;

/// Uniform sample grid over `[x_min, x_max]`.
pub fn xs(x_min: f64, x_max: f64, n: usize) -> Vec<f64> {
    if n <= 1 {
        return vec![x_min];
    }
    let step = (x_max - x_min) / (n as f64 - 1.0);
    (0..n).map(|i| x_min + i as f64 * step).collect()
}

/// Evaluate `e` on the grid, returning `y` values with NaN marking gaps.
pub fn sample(e: &Expr, grid: &[f64]) -> Vec<f64> {
    grid.iter()
        .map(|&x| {
            let y = eval(e, x);
            if y.is_finite() && y.abs() <= Y_CAP {
                y
            } else {
                f64::NAN
            }
        })
        .collect()
}
