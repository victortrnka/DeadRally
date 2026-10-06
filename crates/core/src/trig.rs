//! Sine and cosine from additions and multiplications only, so every OS computes the same
//! values (libm's differ between platforms, and `crates/core/clippy.toml` bans them). The
//! original computes them with the x87's `fsin` and `fcos`; these agree with those to within an
//! ulp or two, far below anything the original truncates them to.

use std::f64::consts::FRAC_2_PI;

/// pi/2 in three parts (fdlibm's `pio2_1`, `pio2_2`, `pio2_3`) so that `x - k * pi/2` loses
/// nothing for the angles a race turns through.
const PIO2_1: f64 = f64::from_bits(0x3FF9_21FB_5440_0000);
const PIO2_2: f64 = f64::from_bits(0x3DD0_B461_1A60_0000);
const PIO2_3: f64 = f64::from_bits(0x3BA3_198A_2E00_0000);

/// `x` less the nearest multiple of pi/2, and that multiple's quarter turn (0 to 3).
fn reduce(x: f64) -> (f64, i64) {
    let k = (x * FRAC_2_PI).round();
    let r = x - k * PIO2_1 - k * PIO2_2 - k * PIO2_3;
    (r, (k as i64).rem_euclid(4))
}

/// Taylor's series for |r| <= pi/4, its terms to r^23 (the next is below 1e-24).
fn sin_kernel(r: f64) -> f64 {
    let r2 = r * r;
    let mut term = 1.0;
    for n in (1..=11).rev() {
        let n = f64::from(n);
        term = 1.0 - r2 / ((2.0 * n) * (2.0 * n + 1.0)) * term;
    }
    r * term
}

/// Taylor's series for |r| <= pi/4, its terms to r^22.
fn cos_kernel(r: f64) -> f64 {
    let r2 = r * r;
    let mut term = 1.0;
    for n in (1..=11).rev() {
        let n = f64::from(n);
        term = 1.0 - r2 / ((2.0 * n - 1.0) * (2.0 * n)) * term;
    }
    term
}

/// The sine of `x` radians.
pub(crate) fn sin(x: f64) -> f64 {
    let (r, quarter) = reduce(x);
    match quarter {
        0 => sin_kernel(r),
        1 => cos_kernel(r),
        2 => -sin_kernel(r),
        _ => -cos_kernel(r),
    }
}

/// The cosine of `x` radians.
pub(crate) fn cos(x: f64) -> f64 {
    let (r, quarter) = reduce(x);
    match quarter {
        0 => cos_kernel(r),
        1 => -sin_kernel(r),
        2 => -cos_kernel(r),
        _ => sin_kernel(r),
    }
}

#[cfg(test)]
mod tests {
    use super::{cos, sin};
    use std::f64::consts::FRAC_1_SQRT_2;

    /// The double below `x`.
    fn below(x: f64) -> f64 {
        f64::from_bits(x.to_bits() - 1)
    }

    /// Within `ulps` units in the last place of `expected`.
    fn close(got: f64, expected: f64, ulps: u64) -> bool {
        let unit = f64::from_bits(expected.abs().to_bits() + 1) - expected.abs();
        (got - expected).abs() <= unit * ulps as f64
    }

    /// The intro truncates `256 / sin` and `170 * cos` of its angles and the cars will turn
    /// through every degree: values off by more than an ulp or two would move a row of the
    /// tilted track, or a car, a pixel from where the original draws it. The references are
    /// the correctly rounded values.
    #[test]
    fn sine_and_cosine_agree_with_the_correctly_rounded_values() {
        let cases = [
            (1.0, 0.017_452_406_437_283_51, 0.999_847_695_156_391_3),
            (30.0, 0.499_999_999_999_999_94, 0.866_025_403_784_438_7),
            (45.0, below(FRAC_1_SQRT_2), FRAC_1_SQRT_2),
            (60.0, 0.866_025_403_784_438_6, 0.500_000_000_000_000_1),
            (89.0, 0.999_847_695_156_391_3, 0.017_452_406_437_283_6),
            (135.0, FRAC_1_SQRT_2, -below(FRAC_1_SQRT_2)),
            (200.0, -0.342_020_143_325_668_66, -0.939_692_620_785_908_4),
            (300.0, -0.866_025_403_784_438_6, 0.500_000_000_000_000_1),
            (359.0, -0.017_452_406_437_283_56, 0.999_847_695_156_391_3),
        ];
        for (degrees, s, c) in cases {
            let x = f64::to_radians(degrees);
            assert!(close(sin(x), s, 2), "sin {degrees}: {} vs {s}", sin(x));
            assert!(close(cos(x), c, 2), "cos {degrees}: {} vs {c}", cos(x));
        }
        // A right angle: the sine is 1 exactly and the cosine a rounding error's size.
        assert_eq!(sin(f64::to_radians(90.0)), 1.0);
        assert!(cos(f64::to_radians(90.0)).abs() < 1e-16);
    }
}
