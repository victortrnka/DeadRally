//! The race's big power-ups as it starts (`generateBigPowerUps` 0x409460): a value for each of
//! the 20 spots, then one time in four a power-up painted onto the track's picture at the 13th
//! or 14th of the track's spots (from `INF.BIN`), its picture from `ENGINE.BPA`'s
//! `OBSTACLE.BPK`.

use deadrally_gamedata::image::Image;

use crate::campaign::Rand;

/// The spots, and a power-up's picture: 16x16, drawn centred on its spot.
const SPOTS: usize = 20;
const SIDE: usize = 16;

/// What the start set up: each spot's value (100 to 149) and the power-up painted, if any (its
/// spot and kind).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BigPowerUps {
    pub(crate) values: [i32; SPOTS],
    pub(crate) painted: Option<(usize, i32)>,
}

/// The start's power-ups on `image` (the track's picture), `spots` the track's places for
/// them, `pictures` the power-ups' 16x16 pictures one after another.
pub(crate) fn place(
    image: &mut Image,
    spots: &[[i32; 2]],
    pictures: &[u8],
    rand: &mut Rand,
) -> BigPowerUps {
    let values = std::array::from_fn(|_| rand.next() % 50 + 100);
    let mut painted = None;
    if rand.next() % 4 == 0 {
        let spot = (rand.next() % 2 + 12) as usize;
        if spots.get(spot).is_some_and(|&[x, _]| x > 0) {
            let kind = rand.next() % 2 + 7;
            paint(image, spots[spot], pictures, kind);
            painted = Some((spot, kind));
        }
    }
    BigPowerUps { values, painted }
}

/// Picture `kind - 1` painted centred on `spot`, its 0 bytes left out.
fn paint(image: &mut Image, [x, y]: [i32; 2], pictures: &[u8], kind: i32) {
    let width = image.width as i32;
    let start = usize::try_from(kind - 1).unwrap_or(0) * SIDE * SIDE;
    for row in 0..SIDE as i32 {
        for column in 0..SIDE as i32 {
            let pixel = pictures
                .get(start + (row * SIDE as i32 + column) as usize)
                .copied()
                .unwrap_or(0);
            let at = (y + row - 8) * width + x + column - 8;
            if pixel != 0
                && let Some(slot) = usize::try_from(at)
                    .ok()
                    .and_then(|at| image.pixels.get_mut(at))
            {
                *slot = pixel;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track() -> Image {
        Image {
            width: 64,
            height: 64,
            pixels: vec![0; 64 * 64],
        }
    }

    /// The start draws `rand()` 21 times, and a power-up's spot and kind when the 21st comes
    /// out a multiple of 4: later races depend on the numbers left.
    #[test]
    fn the_start_draws_rand_as_the_original_does() {
        let mut spots = [[0; 2]; 16];
        spots[12] = [20, 20];
        spots[13] = [40, 40];
        let pictures: Vec<u8> = (0..8u8)
            .flat_map(|kind| vec![kind + 1; SIDE * SIDE])
            .collect();
        for seed in 0..64 {
            let mut rand = Rand::new(seed);
            let mut image = track();
            let placed = place(&mut image, &spots, &pictures, &mut rand);
            let mut expected = Rand::new(seed);
            for _ in 0..20 {
                expected.next();
            }
            if expected.next() % 4 == 0 {
                let spot = (expected.next() % 2 + 12) as usize;
                let kind = expected.next() % 2 + 7;
                assert_eq!(placed.painted, Some((spot, kind)));
                // Painted centred on the spot, its picture the kind's.
                let [x, y] = spots[spot];
                assert_eq!(image.pixels[(y * 64 + x) as usize], kind as u8);
            } else {
                assert_eq!(placed.painted, None);
                assert!(image.pixels.iter().all(|&p| p == 0));
            }
            assert_eq!(rand, expected, "seed {seed}");
        }
    }
}
