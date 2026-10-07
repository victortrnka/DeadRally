//! The end of a game won (spec M6): `showEndAnim_4312D0` after the race in the Arena. The end
//! animation, the title in and out, the best ten with the winner put in
//! (`showHallOfFameEndGame_430FA0`), then the game over and the main menu again.

use crate::animation::Player;
use crate::audio::FULL_VOLUME;
use crate::canvas::at;

use super::results::Film;
use super::{Menu, State};

/// `sub_427280` and `sub_427300`: the title in from 0 to 96 % and out from 100 % to 0, 4 % a
/// wait; the best ten in the same way.
const TITLE_IN_STEPS: u32 = 25;
const TITLE_OUT_STEPS: u32 = 26;
const STEP: i64 = 4;
/// The way out after the best ten (0x431440): 51 waits from 100 % down 2 % a wait, the music
/// from 0xFFDC down 0x51E a wait.
const OUT_STEPS: u32 = 51;
const VOLUME: u32 = 0xFFDC;
const VOLUME_STEP: u32 = 0x51E;
/// The best ten's title, and the border round the winner's row (`drawBorder_421980`).
const FAME_TITLE: (usize, usize) = (0, 84);
const ROW_BORDER: (usize, usize, usize) = (17, 138, 603);
const ROW_STEP: usize = 22;
const ROW_HEIGHT: usize = 24;

impl Menu {
    /// The race in the Arena won (0x43139D): `ENDANI.HAF` under `TR0-MUS.CMF` with
    /// `ENDANI-E.CMF`'s effects.
    pub(super) fn the_end(&mut self) -> State {
        self.sound.stop();
        self.film = Some(Player::new(&self.assets.letterbox));
        self.sound
            .play_music(&self.assets.intro_music, 0, FULL_VOLUME);
        self.sound.load_effects(&self.assets.end_effects);
        State::Film {
            film: Film::End,
            fade: false,
        }
    }

    /// The end animation over, its sound with it (`openAnimation`): the title shown under a
    /// black palette (`showStartScreen`, 0x427880), then faded in.
    pub(super) fn after_the_end(&mut self) -> State {
        self.sound.stop();
        self.screen.copy_all(&self.assets.title.image);
        self.shown = self.screen.clone();
        self.palette.show(&self.assets.title.palette, 0);
        State::EndTitleIn { step: 0 }
    }

    /// A wait of the title's way in.
    pub(super) fn end_title_in(&mut self, step: u32) -> State {
        let title = self.assets.title.palette.clone();
        self.palette.show(&title, STEP * i64::from(step));
        if step + 1 < TITLE_IN_STEPS {
            return State::EndTitleIn { step: step + 1 };
        }
        State::EndTitleOut { step: 0 }
    }

    /// A wait of the title's way out (`sub_427300`); then the best ten.
    pub(super) fn end_title_out(&mut self, step: u32) -> State {
        let title = self.assets.title.palette.clone();
        self.palette.show(&title, 100 - STEP * i64::from(step));
        if step + 1 < TITLE_OUT_STEPS {
            return State::EndTitleOut { step: step + 1 };
        }
        // 0x4313B8: the shop's pictures loaded again resets the pulse and the shop's loops.
        self.shop.reset();
        self.palette.reset_pulse();
        self.open_fame_entry()
    }

    /// `showHallOfFameEndGame_430FA0`: the menus' background with the bottom panel telling of
    /// the win, the best ten's title, the winner put into the best ten by their races (fewest
    /// first) with a border round their row and `dr.cfg` written, the rows drawn; shown under
    /// the black palette, then faded in.
    fn open_fame_entry(&mut self) -> State {
        let mut screen = std::mem::take(&mut self.screen);
        screen.copy_all(&self.graphics.background);
        self.graphics.panel_frame(&mut screen, 0, 371, 639, 109);
        let lines = self.assets.menu.texts.campaign.end_lines.clone();
        self.panel.tell(&lines);
        self.graphics.panel_text(&mut screen, &self.panel);
        screen.draw(
            &self.assets.menu.fame_title,
            at(FAME_TITLE.0, FAME_TITLE.1),
            true,
        );
        let player = *self.campaign.player();
        let difficulty = self.config.difficulty();
        if let Some(rank) = self
            .config
            .insert_hall_of_fame(player.name(), player.races, difficulty)
        {
            let (x, y, w) = ROW_BORDER;
            self.thin_border(&mut screen, x, y + ROW_STEP * rank, w, ROW_HEIGHT);
            self.save = true;
        }
        self.draw_best_ten(&mut screen);
        self.screen = screen;
        self.shown = self.screen.clone();
        self.compose_palette();
        State::EndFameIn { step: 0 }
    }

    /// A wait of the best ten's way in; then the keys pressed meanwhile let go of.
    pub(super) fn end_fame_in(&mut self, step: u32) -> State {
        self.palette.fade(STEP * i64::from(step));
        if step + 1 < TITLE_IN_STEPS {
            return State::EndFameIn { step: step + 1 };
        }
        self.keys.take();
        self.keys.take();
        State::EndFameWait
    }

    /// The best ten's wait for a key.
    pub(super) fn end_fame_wait(&mut self) -> State {
        self.palette.after_wait();
        if self.keys.take() == 0 {
            return State::EndFameWait;
        }
        self.keys.take();
        self.compose_palette();
        State::EndFameOut { step: 0 }
    }

    /// A wait of the way out with the music falling; then the menus' music, the game over, and
    /// the main menu fading in.
    pub(super) fn end_fame_out(&mut self, step: u32) -> State {
        self.sound.set_mask((VOLUME - VOLUME_STEP * step) >> 8);
        self.palette.fade(100 - 2 * i64::from(step));
        if step + 1 < OUT_STEPS {
            return State::EndFameOut { step: step + 1 };
        }
        self.menu_sound_back();
        self.reset_game();
        self.set_up();
        State::FadeIn { step: 0 }
    }
}
