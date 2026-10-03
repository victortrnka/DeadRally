//! Throwaway test scene for M0. It exercises the frame, palette, input and audio contract so
//! the platform spike has something to show and play. Removed when M2 brings the real menus.

use crate::{AUDIO_FRAMES_PER_TICK, Frame, InputEvent, Key, PadButton};

// Palette entries 0..=3 are pinned so the bar, the grid and the stick dot do not cycle with
// the ramp.
const WHITE: u8 = 1;
const GREY: u8 = 2;
const RED: u8 = 3;
const PINNED: [[u8; 3]; 4] = [[0, 0, 0], [63, 63, 63], [16, 16, 16], [63, 0, 0]];

const BAR_WIDTH: u32 = 4;
const GRID_COLUMNS: u32 = 16;

/// 440 Hz in 32-bit phase units per sample: 440 * 2^32 / 44 100, rounded.
const TONE_STEP: u32 = 42_852_281;
const TONE_AMPLITUDE: i32 = 2_048;
/// The click is 5 ms (221 frames at 44.1 kHz) of square wave at about 1 kHz.
const CLICK_FRAMES: u32 = 221;
const CLICK_HALF_PERIOD: u32 = 22;
const CLICK_AMPLITUDE: i32 = 8_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Vga640x480,
    Vga320x200,
    Wide640x360,
}

impl Mode {
    fn size(self) -> (u32, u32) {
        match self {
            Mode::Vga640x480 => (640, 480),
            Mode::Vga320x200 => (320, 200),
            Mode::Wide640x360 => (640, 360),
        }
    }

    fn aspect(self) -> (u32, u32) {
        match self {
            Mode::Vga640x480 | Mode::Vga320x200 => (4, 3),
            Mode::Wide640x360 => (16, 9),
        }
    }

    fn next(self) -> Mode {
        match self {
            Mode::Vga640x480 => Mode::Vga320x200,
            Mode::Vga320x200 => Mode::Wide640x360,
            Mode::Wide640x360 => Mode::Vga640x480,
        }
    }
}

#[derive(Debug)]
pub(crate) struct TestScene {
    tick: u64,
    mode: Mode,
    palette: [[u8; 3]; 256],
    pixels: Vec<u8>,
    held_keys: Vec<bool>,
    held_buttons: [bool; 4],
    stick: [i16; 2],
    tone_on: bool,
    tone_phase: u32,
    click_frames_left: u32,
    audio: Vec<i16>,
}

impl TestScene {
    pub(crate) fn new() -> TestScene {
        let mut scene = TestScene {
            tick: 0,
            mode: Mode::Vga640x480,
            palette: [[0; 3]; 256],
            pixels: Vec::new(),
            held_keys: vec![false; Key::ALL.len()],
            held_buttons: [false; 4],
            stick: [0; 2],
            tone_on: true,
            tone_phase: 0,
            click_frames_left: 0,
            audio: Vec::new(),
        };
        scene.set_mode(Mode::Vga640x480);
        scene.update_palette();
        scene
    }

    pub(crate) fn input(&mut self, event: InputEvent) {
        match event {
            InputEvent::Key { key, pressed } => {
                let was_held = std::mem::replace(&mut self.held_keys[key as usize], pressed);
                if pressed && !was_held {
                    self.click_frames_left = CLICK_FRAMES;
                    match key {
                        Key::Tab => self.set_mode(self.mode.next()),
                        Key::T => self.tone_on = !self.tone_on,
                        _ => {}
                    }
                }
            }
            InputEvent::PadButton { button, pressed } => {
                let was_held = std::mem::replace(&mut self.held_buttons[button as usize], pressed);
                if pressed && !was_held {
                    self.click_frames_left = CLICK_FRAMES;
                }
            }
            InputEvent::PadAxis { axis, value } => {
                self.stick[axis as usize] = value;
            }
        }
    }

    pub(crate) fn tick(&mut self) {
        self.tick += 1;
        self.update_palette();
        self.render();
        self.mix_audio();
    }

    pub(crate) fn frame(&self) -> Frame<'_> {
        let (width, height) = self.mode.size();
        Frame {
            width,
            height,
            pixels: &self.pixels,
            palette: &self.palette,
            aspect: self.mode.aspect(),
        }
    }

    pub(crate) fn take_audio(&mut self, out: &mut Vec<i16>) {
        out.append(&mut self.audio);
    }

    /// Switches mode and redraws at once, so `frame()` never pairs the new size with old
    /// pixels.
    fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        let (width, height) = mode.size();
        self.pixels = vec![0; (width * height) as usize];
        self.render();
    }

    /// Rotates the ramp by one entry per tick, then re-pins the fixed colours.
    fn update_palette(&mut self) {
        let shift = (self.tick % 256) as usize;
        for (index, entry) in self.palette.iter_mut().enumerate() {
            *entry = ramp_color(((index + shift) % 256) as u8);
        }
        self.palette[..PINNED.len()].copy_from_slice(&PINNED);
    }

    fn render(&mut self) {
        let (width, height) = self.mode.size();
        for (y, row) in self.pixels.chunks_exact_mut(width as usize).enumerate() {
            for (x, pixel) in row.iter_mut().enumerate() {
                *pixel = ((x + y) & 0xFF) as u8;
            }
        }

        let bar_x = (self.tick % u64::from(width)) as u32;
        self.fill_rect(bar_x, 0, BAR_WIDTH, height, WHITE);

        // A frame pixel is shown aspect.0 / width wide and aspect.1 / height tall, so this many
        // pixels across look as long on screen as `tall` pixels down. Squares stay squares in
        // every mode; a wider mode shows more, it never stretches.
        let (aspect_width, aspect_height) = self.mode.aspect();
        let across = |tall: u32| tall * width * aspect_height / (height * aspect_width);

        let cell_height = height / 16;
        let cell_width = across(cell_height);
        let key_count = Key::ALL.len();
        for cell in 0..key_count + PadButton::ALL.len() {
            let held = if cell < key_count {
                self.held_keys[cell]
            } else {
                self.held_buttons[cell - key_count]
            };
            let column = cell as u32 % GRID_COLUMNS;
            let row = cell as u32 / GRID_COLUMNS;
            self.fill_rect(
                cell_width * (2 + column),
                cell_height * (2 + row),
                cell_width - 1,
                cell_height - 1,
                if held { WHITE } else { GREY },
            );
        }

        let radius_y = height / 8;
        let radius_x = across(radius_y);
        let dot_height = (cell_height / 2).max(2);
        let dot_width = across(dot_height).max(2);
        let offset = |value: i16, radius: u32, dot: u32, centre: u32| {
            let radius = i32::try_from(radius).expect("radius fits i32");
            let corner = i32::try_from(centre - dot / 2).expect("centre fits i32");
            u32::try_from(corner + i32::from(value) * radius / 32_768).expect("dot stays on screen")
        };
        let dot_x = offset(self.stick[0], radius_x, dot_width, width / 2);
        let dot_y = offset(self.stick[1], radius_y, dot_height, height * 3 / 4);
        self.fill_rect(dot_x, dot_y, dot_width, dot_height, RED);
    }

    /// Fills a rectangle, clipped to the frame.
    fn fill_rect(&mut self, x: u32, y: u32, width: u32, height: u32, color: u8) {
        let (frame_width, frame_height) = self.mode.size();
        if x >= frame_width || y >= frame_height {
            return;
        }
        let x_end = (x + width).min(frame_width);
        let y_end = (y + height).min(frame_height);
        for row in y..y_end {
            let start = (row * frame_width + x) as usize;
            let end = (row * frame_width + x_end) as usize;
            self.pixels[start..end].fill(color);
        }
    }

    fn mix_audio(&mut self) {
        for _ in 0..AUDIO_FRAMES_PER_TICK {
            let mut sample = 0;
            if self.tone_on {
                sample += triangle(self.tone_phase) * TONE_AMPLITUDE / 32_768;
            }
            self.tone_phase = self.tone_phase.wrapping_add(TONE_STEP);
            if self.click_frames_left > 0 {
                let elapsed = CLICK_FRAMES - self.click_frames_left;
                sample += if (elapsed / CLICK_HALF_PERIOD).is_multiple_of(2) {
                    CLICK_AMPLITUDE
                } else {
                    -CLICK_AMPLITUDE
                };
                self.click_frames_left -= 1;
            }
            let sample = sample.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
            self.audio.extend_from_slice(&[sample, sample]);
        }
    }
}

/// A full-scale triangle wave (-32768..=32767) from a 32-bit phase.
fn triangle(phase: u32) -> i32 {
    let p = (phase >> 16) as i32;
    if p < 32_768 {
        p * 2 - 32_768
    } else {
        (65_535 - p) * 2 - 32_767
    }
}

fn ramp_color(index: u8) -> [u8; 3] {
    [index / 4, index % 64, 63 - index / 4]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triangle_spans_full_scale_without_overflow() {
        // The mixer multiplies this by the amplitude in i32; out-of-range values would clip.
        assert_eq!(triangle(0), -32_768);
        assert_eq!(triangle(0x7FFF_0000), 32_766);
        assert_eq!(triangle(0x8000_0000), 32_767);
        assert_eq!(triangle(u32::MAX), -32_767);
    }

    #[test]
    fn ramp_colours_stay_within_six_bits() {
        // VGA palettes are 6-bit; a value above 63 would be silently masked by the DAC.
        for index in 0..=255u8 {
            assert!(ramp_color(index).iter().all(|&c| c <= 63), "index {index}");
        }
    }
}
