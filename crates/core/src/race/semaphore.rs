//! The start lights (`raceSemaphore` 0x414FC0), drawn at the view's top right from
//! `ENGINE.BPA`'s `GEN-LAM.BPK`: three 80x60 pictures. From the race's frame 11 the first
//! slides in from the right a pixel a tick; from frame 131 the second stands in its place;
//! from frame 191 the race starts and the third slides out two pixels a tick.

use super::buffer::{Buffer, LEFT};

/// A light's size, where it stands (column 240), and how far right it starts.
const WIDTH: usize = 80;
const HEIGHT: usize = 60;
const PLACE: i64 = LEFT as i64 + 240;
const OUT: i32 = 80;
/// The frames each light shows from.
const FIRST: i32 = 10;
const SECOND: i32 = 130;
pub(crate) const GO: i32 = 190;

/// What the lights did this frame that the race must follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Event {
    /// The first light came on, with its sound.
    Ready,
    /// The second light came on, with its sound.
    Set,
    /// The race started: its clocks start from 0, with the start's sound.
    Go,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Semaphore {
    pictures: Vec<u8>,
    /// How far right of its place the light is (0x464F64).
    x: i32,
    /// Whether the first light's sound has played (0x456AD8).
    ready: bool,
    /// 0 before the second light, 1 while it shows, 2 once the race has started (0x456AD4).
    stage: u8,
}

impl Semaphore {
    pub(crate) fn new(pictures: Vec<u8>) -> Semaphore {
        Semaphore {
            pictures,
            x: OUT,
            ready: false,
            stage: 0,
        }
    }

    /// The lights at the race's frame `frame`, `ticks` the ticks the HUD counted last frame.
    pub(crate) fn draw(&mut self, buffer: &mut Buffer, frame: i32, ticks: i32) -> Option<Event> {
        let mut event = None;
        if frame > FIRST && frame <= SECOND {
            if !self.ready {
                self.ready = true;
                event = Some(Event::Ready);
            }
            self.x = (self.x - ticks).max(0);
            self.picture(buffer, 0, self.x);
        }
        if frame > GO {
            if self.stage == 1 {
                event = Some(Event::Go);
            }
            self.x = if self.x + ticks > OUT {
                OUT
            } else {
                self.x + 2 * ticks
            };
            self.picture(buffer, 2, self.x);
            self.stage = 2;
        }
        if frame > SECOND && frame <= GO {
            if self.stage == 0 {
                event = Some(Event::Set);
            }
            self.picture(buffer, 1, 0);
            self.stage = 1;
        }
        event
    }

    fn picture(&self, buffer: &mut Buffer, which: usize, x: i32) {
        let size = WIDTH * HEIGHT;
        let picture = self
            .pictures
            .get(which * size..(which + 1) * size)
            .unwrap_or(&[]);
        buffer.draw_opaque(picture, WIDTH, HEIGHT, PLACE + i64::from(x));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lights() -> Semaphore {
        let pictures = (1..=3u8)
            .flat_map(|light| vec![light; WIDTH * HEIGHT])
            .collect();
        Semaphore::new(pictures)
    }

    /// Where light `which` shows on the top row: its leftmost column on the screen.
    fn leftmost(buffer: &Buffer, which: u8) -> Option<usize> {
        (0..320).find(|&x| buffer.pixel(x, 0) == which)
    }

    /// The first light slides in from the right a pixel a tick from the race's frame 11 and
    /// is fully in at frame 90, with its sound once.
    #[test]
    fn the_first_light_slides_in_a_pixel_a_tick() {
        let mut semaphore = lights();
        let mut buffer = Buffer::default();
        assert_eq!(semaphore.draw(&mut buffer, 10, 1), None);
        assert_eq!(leftmost(&buffer, 1), None);
        assert_eq!(semaphore.draw(&mut buffer, 11, 1), Some(Event::Ready));
        assert_eq!(leftmost(&buffer, 1), Some(319));
        for frame in 12..=90 {
            assert_eq!(semaphore.draw(&mut buffer, frame, 1), None);
        }
        assert_eq!(leftmost(&buffer, 1), Some(240));
    }

    /// At frame 131 the second light stands in its place; at frame 191 the race starts and the
    /// third slides out two pixels a tick.
    #[test]
    fn the_race_starts_at_frame_191() {
        let mut semaphore = lights();
        let mut buffer = Buffer::default();
        for frame in 11..=130 {
            semaphore.draw(&mut buffer, frame, 1);
        }
        assert_eq!(semaphore.draw(&mut buffer, 131, 1), Some(Event::Set));
        assert_eq!(leftmost(&buffer, 2), Some(240));
        for frame in 132..=190 {
            assert_eq!(semaphore.draw(&mut buffer, frame, 1), None);
        }
        assert_eq!(semaphore.draw(&mut buffer, 191, 1), Some(Event::Go));
        assert_eq!(leftmost(&buffer, 3), Some(242));
        let mut next = Buffer::default();
        assert_eq!(semaphore.draw(&mut next, 192, 1), None);
        assert_eq!(leftmost(&next, 3), Some(244));
    }
}
