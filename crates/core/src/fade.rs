//! The original's palette fades, in its fixed-point arithmetic (`transitionToCurrentImage`
//! 0x427280, `transitionToBlack` 0x427300; spec M1a §3.6). Matching the rounding exactly is what
//! lets a screenshot taken mid-fade be matched pixel for pixel.

use deadrally_gamedata::image::Palette;

/// One fade step: 4 % brightness, `0x40000` in the original's 16.16 fixed point.
pub(crate) const FADE_STEP: i64 = 0x4_0000;

/// Full brightness, 100 %: `0x640000`.
pub(crate) const FADE_FULL: i64 = 0x64_0000;

/// A 6-bit component at brightness `level` (0..=[`FADE_FULL`]).
pub(crate) fn fade_component(component: u8, level: i64) -> u8 {
    // colorToPaletteEntry (0x43B290) stores the component in units of 1/655.36 ...
    let stored = (i64::from(component) << 32) / 6_553_600;
    // ... convertColorToPaletteColor (0x43B2C0) scales it, and the caller rounds.
    let scaled = (level * stored) >> 16;
    u8::try_from((scaled + 0x8000) >> 16).expect("a fade never brightens")
}

/// The whole palette at brightness `level`.
pub(crate) fn fade(palette: &Palette, level: i64) -> Palette {
    Palette(
        palette
            .0
            .map(|rgb| rgb.map(|component| fade_component(component, level))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_and_zero_brightness_are_exact() {
        for component in 0..=63 {
            assert_eq!(fade_component(component, FADE_FULL), component);
            assert_eq!(fade_component(component, 0), 0);
        }
    }

    #[test]
    fn the_fade_in_ends_at_96_percent() {
        // The original's fade-in stops one step short (0x427280 loops while level < 0x640000),
        // so held logos are slightly dimmer than their palette: 63 shows as 60.
        assert_eq!(fade_component(63, 24 * FADE_STEP), 60);
        assert_eq!(fade_component(32, 24 * FADE_STEP), 31);
    }

    #[test]
    fn rounding_follows_the_original() {
        // 63 at 4 %: 41287 * 4 / 65536 = 2.52, rounded to 3.
        assert_eq!(fade_component(63, FADE_STEP), 3);
        // 10 at 12 %: 6553 * 12 / 65536 = 1.199, rounded to 1.
        assert_eq!(fade_component(10, 3 * FADE_STEP), 1);
    }

    #[test]
    fn brightness_never_decreases_as_the_level_rises() {
        for component in 0..=63 {
            let levels: Vec<u8> = (0..=25)
                .map(|step| fade_component(component, step * FADE_STEP))
                .collect();
            assert!(
                levels.windows(2).all(|pair| pair[0] <= pair[1]),
                "{component}: {levels:?}"
            );
        }
    }
}
