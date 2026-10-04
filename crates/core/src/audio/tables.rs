//! Pitch tables, computed without floating point so every OS gets the same values (the core
//! may not call `powf`).

/// `2^(k / 768)` for `k` in `0..768`, scaled by `2^30`: a semitone is 64 steps, an octave 768.
pub(crate) static EXP2: [u32; 768] = exp2_table();

const fn exp2_table() -> [u32; 768] {
    // 2^(1/768) in Q62, found by bisection on r^768 = 2, then powers of it. The upper bound
    // 1 + 1/512 keeps every product within u128 (its 768th power is about 4.5).
    const ONE: u128 = 1 << 62;
    let (mut low, mut high) = (ONE, ONE + ONE / 512);
    while high - low > 1 {
        let middle = (low + high) / 2;
        if power(middle, 768) <= 2 * ONE {
            low = middle;
        } else {
            high = middle;
        }
    }
    let mut table = [0u32; 768];
    let mut value = ONE;
    let mut k = 0;
    while k < 768 {
        // Q62 to Q30, rounded.
        table[k] = ((value + (1 << 31)) >> 32) as u32;
        value = (value * low) >> 62;
        k += 1;
    }
    table
}

/// `base^exponent` in Q62.
const fn power(base: u128, exponent: u32) -> u128 {
    let (mut result, mut base, mut exponent) = (1u128 << 62, base, exponent);
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = (result * base) >> 62;
        }
        base = (base * base) >> 62;
        exponent >>= 1;
    }
    result
}

/// `base_hz · 2^(steps / 768)`, rounded down like the original's integer conversion.
pub(crate) fn scale_by_exp2(base_hz: u32, steps: i32) -> u32 {
    let octaves = steps.div_euclid(768);
    let fraction = EXP2[steps.rem_euclid(768) as usize];
    let scaled = u64::from(base_hz) * u64::from(fraction);
    let shift = 30 - octaves;
    let hz = if shift >= 0 {
        scaled >> shift.min(63)
    } else {
        scaled << (-shift).min(32)
    };
    u32::try_from(hz).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_exponential_table_hits_its_known_points() {
        // An error here detunes every effect; these are exact to the last bit.
        assert_eq!(EXP2[0], 1 << 30);
        assert_eq!(EXP2[384], 1_518_500_250, "the square root of 2");
        assert_eq!(EXP2[64], 1_137_589_835, "a semitone up");
        assert!(EXP2.windows(2).all(|pair| pair[1] > pair[0]));
        // One more step after the last entry is an octave.
        let next = (u64::from(EXP2[767]) * u64::from(EXP2[1])) >> 30;
        assert!((next as i64 - (1 << 31)).abs() <= 2, "{next}");
    }

    #[test]
    fn scaling_by_octaves_doubles_and_halves() {
        assert_eq!(scale_by_exp2(8363, 0), 8363);
        assert_eq!(scale_by_exp2(8363, 768), 16_726);
        assert_eq!(scale_by_exp2(8363, -768), 4181);
        // XM's middle C on a linear table: period 4608 plays at 8363 Hz.
        assert_eq!(scale_by_exp2(8363, 64), 8860, "a semitone above middle C");
    }
}
