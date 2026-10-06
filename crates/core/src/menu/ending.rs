//! The end of the game (spec M6): a won race in the Arena (0x4347CB, the player's place 1)
//! calls `showEndAnim` (0x4312D0), the end animation `ENDANI.HAF` with its music, the Hall of
//! Fame entry and the menus faded back in. That comes with M6d; until then [`Menu::the_end`]
//! goes straight on to what the game's "ended" flags lead to: the Adversary's screen after the
//! won race (0x4354F9) renames the Start Racing menu's rows to a new game, resets the flags and
//! sets the drivers up afresh (`initDrivers`), and `[0x456B64]` takes the shop and the Start
//! Racing menu back to the main menu.

use super::{Menu, State};

impl Menu {
    /// The end of a won game, without its animation and Hall of Fame entry (M6d): the game
    /// ended and the main menu faded in with the menus' music.
    pub(super) fn the_end(&mut self) -> State {
        self.after_race();
        self.end_game();
        self.set_up();
        State::FadeIn { step: 0 }
    }
}
