//! The original's animation player, `openAnimation` (0x4185B0): a HAF animation in the 320x200
//! mode, its 320x120 frames from row 40 inside `FRAMES.BPK`'s letterbox, each frame's effect
//! played on the next of six channels. The intro plays `SANIM.haf` with it (spec M1a §5.2), the
//! Adversary's turn `ENDANI0.HAF` and the end `ENDANI.HAF` (spec M6).

use deadrally_gamedata::assets::Picture;
use deadrally_gamedata::haf::{Animation, FRAME_PIXELS};
use deadrally_gamedata::image::Palette;

use crate::Frame;
use crate::audio::Sound;
use crate::keys::Keys;

/// The animation's screen: 320x200, with the frames from row 40.
const WIDTH: u32 = 320;
const HEIGHT: u32 = 200;
const FIRST_ROW: usize = 40;
/// The letterbox owns palette entries 0..=15, the animation's frames the rest.
const LETTERBOX_COLOURS: usize = 16;
/// The effects take channels 1..=6 in turn.
const EFFECT_CHANNELS: usize = 6;

/// Where an animation is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Player {
    /// The frame being waited for, and the ticks since the previous one.
    next: usize,
    waited: u32,
    /// The channel the next effect plays on.
    effect_channel: usize,
    pixels: Vec<u8>,
    palette: Palette,
}

/// What a tick of the animation did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tick {
    Playing,
    /// The last frame was due, or a key was pressed: the animation is over.
    Ended,
}

impl Player {
    /// A black screen with the letterbox's colours set, as `openAnimation` starts.
    pub(crate) fn new(letterbox: &Picture) -> Player {
        assert_eq!(
            (letterbox.image.width, letterbox.image.height),
            (WIDTH, HEIGHT),
            "the animations' letterbox is 320x200"
        );
        let mut palette = Palette::BLACK;
        palette.0[..LETTERBOX_COLOURS].copy_from_slice(&letterbox.palette.0[..LETTERBOX_COLOURS]);
        Player {
            next: 0,
            waited: 0,
            effect_channel: 1,
            pixels: letterbox.image.pixels.clone(),
            palette,
        }
    }

    /// One tick of `openAnimation`: when frame `next` is due it replaces the previous one, then
    /// the original checks for a key before showing it. So a key press ends the animation at
    /// the next frame, which is never shown, and the last frame is never shown either.
    ///
    /// The original also checks once before frame 0. A press made while the game loads is read
    /// only when a frame is next shown (`refreshScreen`, 0x43B580), that is during frame 0's
    /// wait, so it ends the animation when frame 0 is due, as here.
    pub(crate) fn tick(
        &mut self,
        animation: &Animation,
        keys: &mut Keys,
        sound: &mut Sound,
    ) -> Tick {
        self.waited += 1;
        let mut due = None;
        while self.waited >= u32::from(animation.delays[self.next]) {
            due = Some(self.next);
            self.next += 1;
            self.waited = 0;
            if self.next == animation.len() || keys.take() != 0 {
                // The frame ending the animation is never shown, and its effect, which the
                // original starts and cuts at once, never sounds.
                return Tick::Ended;
            }
            // The original triggers a frame's effect right after drawing it.
            let effect = animation.effects[self.next - 1];
            if effect != 0 {
                sound.trigger(self.effect_channel, effect);
                self.effect_channel = self.effect_channel % EFFECT_CHANNELS + 1;
            }
        }
        if let Some(index) = due {
            match animation.frame(index) {
                Ok(frame) => {
                    self.palette.0[LETTERBOX_COLOURS..]
                        .copy_from_slice(&frame.palette.0[LETTERBOX_COLOURS..]);
                    let start = FIRST_ROW * WIDTH as usize;
                    self.pixels[start..start + FRAME_PIXELS].copy_from_slice(&frame.pixels);
                }
                // Only data of an unknown version can get here (the known version's frames are
                // all tested), and the player was warned about it at start-up.
                Err(_) => return Tick::Ended,
            }
        }
        Tick::Playing
    }

    pub(crate) fn frame(&self) -> Frame<'_> {
        Frame {
            width: WIDTH,
            height: HEIGHT,
            pixels: &self.pixels,
            palette: &self.palette.0,
            aspect: (4, 3),
        }
    }
}
