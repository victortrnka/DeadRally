use crate::test_scene::TestScene;
use crate::{Frame, InputEvent};

/// The whole game state. In M0 it runs the throwaway test scene.
#[derive(Debug)]
pub struct Game {
    scene: TestScene,
}

impl Game {
    #[must_use]
    pub fn new() -> Game {
        Game {
            scene: TestScene::new(),
        }
    }

    pub fn input(&mut self, event: InputEvent) {
        self.scene.input(event);
    }

    /// Advances the simulation by exactly 1/70 s.
    pub fn tick(&mut self) {
        self.scene.tick();
    }

    #[must_use]
    pub fn frame(&self) -> Frame<'_> {
        self.scene.frame()
    }

    /// Appends the interleaved stereo samples produced since the last call
    /// (`AUDIO_FRAMES_PER_TICK * AUDIO_CHANNELS` per tick).
    pub fn take_audio(&mut self, out: &mut Vec<i16>) {
        self.scene.take_audio(out);
    }
}

impl Default for Game {
    fn default() -> Game {
        Game::new()
    }
}
