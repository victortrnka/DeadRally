use deadrally_gamedata::assets::Assets;

use crate::startup::Startup;
use crate::test_scene::TestScene;
use crate::{Frame, InputEvent};

/// The whole game state. In M1a it runs the original's startup sequence, or the M0 test scene.
#[derive(Debug)]
pub struct Game {
    scene: Scene,
}

#[derive(Debug)]
enum Scene {
    Test(Box<TestScene>),
    Startup(Box<Startup>),
}

impl Game {
    /// Starts the original's startup sequence: intro, Apogee, Remedy, title.
    ///
    /// # Panics
    ///
    /// If the intro letterbox is not 320x200 ([`Assets::load`] guarantees it is).
    #[must_use]
    pub fn new(assets: Assets) -> Game {
        Game {
            scene: Scene::Startup(Box::new(Startup::new(assets))),
        }
    }

    /// The M0 test scene, which needs no game data: `-testscene`, headless runs and CI.
    #[must_use]
    pub fn test_scene() -> Game {
        Game {
            scene: Scene::Test(Box::new(TestScene::new())),
        }
    }

    pub fn input(&mut self, event: InputEvent) {
        match &mut self.scene {
            Scene::Test(scene) => scene.input(event),
            Scene::Startup(scene) => scene.input(event),
        }
    }

    /// Advances the simulation by exactly one 14 ms tick.
    pub fn tick(&mut self) {
        match &mut self.scene {
            Scene::Test(scene) => scene.tick(),
            Scene::Startup(scene) => scene.tick(),
        }
    }

    #[must_use]
    pub fn frame(&self) -> Frame<'_> {
        match &self.scene {
            Scene::Test(scene) => scene.frame(),
            Scene::Startup(scene) => scene.frame(),
        }
    }

    /// Appends the interleaved stereo samples produced since the last call
    /// (`AUDIO_FRAMES_PER_TICK * AUDIO_CHANNELS` per tick).
    pub fn take_audio(&mut self, out: &mut Vec<i16>) {
        match &mut self.scene {
            Scene::Test(scene) => scene.take_audio(out),
            Scene::Startup(scene) => scene.take_audio(out),
        }
    }
}
