use deadrally_gamedata::assets::Assets;
use deadrally_gamedata::dr_cfg::DrCfg;

use crate::menu::Menu;
use crate::startup::Startup;
use crate::test_scene::TestScene;
use crate::{Frame, InputEvent};

/// The whole game state: the original's startup sequence and then its main menu, or the M0
/// test scene.
#[derive(Debug)]
pub struct Game {
    scene: Scene,
}

#[derive(Debug)]
enum Scene {
    Test(Box<TestScene>),
    Startup(Box<Startup>),
    Menu(Box<Menu>),
    /// Only while one scene hands over to the next.
    Handover,
}

impl Game {
    /// Starts the original's startup sequence (intro, Apogee, Remedy, title), then the main
    /// menu.
    ///
    /// # Panics
    ///
    /// If the intro letterbox is not 320x200 ([`Assets::load`] guarantees it is).
    #[must_use]
    /// `config` is the player's `dr.cfg` (`assets.menu.default_config` when there is none).
    pub fn new(assets: Assets, config: DrCfg) -> Game {
        Game::with_seed(assets, config, 0)
    }

    /// Like [`Game::new`], with `seed` for the original's random numbers, which it seeds with
    /// the milliseconds since its start as the main menu starts: same seed and same keys, same
    /// drivers and races (spec M3a §2).
    #[must_use]
    pub fn with_seed(assets: Assets, config: DrCfg, seed: u32) -> Game {
        Game {
            scene: Scene::Startup(Box::new(Startup::new(assets, config, seed))),
        }
    }

    /// The saved games `DR.SG0`..`DR.SG7` as the host found them, `None` for an empty slot;
    /// for a game that has not reached its menu yet.
    pub fn set_saved_games(&mut self, slot_files: Vec<Option<Vec<u8>>>) {
        if let Scene::Startup(startup) = &mut self.scene {
            let mut slot_files = slot_files;
            slot_files.resize(deadrally_gamedata::save_game::SLOTS, None);
            startup.slot_files = slot_files;
        }
    }

    /// A game the player saved since the last call: its slot and the file to write, which
    /// the host keeps out of the game folder.
    pub fn take_saved_game(&mut self) -> Option<(usize, Vec<u8>)> {
        match &mut self.scene {
            Scene::Menu(menu) => menu.take_saved_game(),
            _ => None,
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
            Scene::Menu(scene) => scene.input(event),
            Scene::Handover => unreachable!("no scene hands over between calls"),
        }
    }

    /// Advances the simulation by exactly one 14 ms tick.
    pub fn tick(&mut self) {
        match &mut self.scene {
            Scene::Test(scene) => scene.tick(),
            Scene::Startup(scene) => scene.tick(),
            Scene::Menu(scene) => scene.tick(),
            Scene::Handover => unreachable!("no scene hands over between calls"),
        }
        if matches!(&self.scene, Scene::Startup(startup) if startup.finished())
            && let Scene::Startup(startup) = std::mem::replace(&mut self.scene, Scene::Handover)
        {
            self.scene = Scene::Menu(Box::new(startup.into_menu()));
        }
    }

    /// The player chose to exit the game and its end screen is over: the frontend should close.
    #[must_use]
    pub fn quit_requested(&self) -> bool {
        matches!(&self.scene, Scene::Menu(menu) if menu.quit_requested())
    }

    /// The bytes of `dr.cfg` when the original would write the file (at start-up, on leaving
    /// Configure, after the end screen); `None` otherwise. The frontend writes them to
    /// DeadRally's own copy.
    pub fn take_config(&mut self) -> Option<Vec<u8>> {
        match &mut self.scene {
            Scene::Startup(scene) => scene.take_config(),
            Scene::Menu(scene) => scene.take_config(),
            Scene::Test(_) => None,
            Scene::Handover => unreachable!("no scene hands over between calls"),
        }
    }

    #[must_use]
    pub fn frame(&self) -> Frame<'_> {
        match &self.scene {
            Scene::Test(scene) => scene.frame(),
            Scene::Startup(scene) => scene.frame(),
            Scene::Menu(scene) => scene.frame(),
            Scene::Handover => unreachable!("no scene hands over between calls"),
        }
    }

    /// Appends the interleaved stereo samples produced since the last call
    /// (`AUDIO_FRAMES_PER_TICK * AUDIO_CHANNELS` per tick).
    pub fn take_audio(&mut self, out: &mut Vec<i16>) {
        match &mut self.scene {
            Scene::Test(scene) => scene.take_audio(out),
            Scene::Startup(scene) => scene.take_audio(out),
            Scene::Menu(scene) => scene.take_audio(out),
            Scene::Handover => unreachable!("no scene hands over between calls"),
        }
    }
}
