# M8a — race records and a growing Hall of Fame: verification

The [design](../design/2026-10-10-records-and-fame-design.md) goes beyond the original, so these checks are DeadRally's own: tests that failed when the behaviour was taken out, and screens checked by eye in the game (`deadrally-headless render --deadrally`), then pinned with manifests. The runs checked against the original play as the Windows version and keep their manifests unchanged.

## Results

| Check | How | Result |
|---|---|---|
| `dr.cfg` keeps the new data | unit tests in `crates/gamedata/src/dr_cfg.rs` | **Pass.** Race records by circuit, laps and car, and the Hall of Fame's entries past the tenth, are written and read back. The first ten entries stay in the original's slots. A file of 1.0.0 (the original's bytes and the Arena's lap records) reads and writes back byte for byte. A block is written empty only before a filled one. |
| A whole race's time becomes its race record as the race ends | `a_race_s_records_are_written_to_dr_cfg_as_it_ends`: M6's leader turn as DeadRally, every record slow | **Pass.** The `dr.cfg` handed over as the race ends holds the easy race's 02:05.58 as Holocaust's 4-lap record of the player's car, and no other race record. |
| An abandoned race sets none | `an_abandoned_race_sets_no_race_record`: M4b's abort run as DeadRally | **Pass**, and it failed with the whole-race check taken out: the seconds before Escape and Y then became a race record. |
| The Arena's race | `a_lap_in_the_arena_is_the_arena_s_record_and_written_as_it_ends`: M6's won Arena as DeadRally | **Pass.** The race is the Arena's 9-lap record of the player's car, by the same driver as its lap record. |
| A race is whole only when finished on its last lap | `a_whole_race_is_one_finished_on_its_last_lap` | **Pass**, and it failed when any finished car counted. |
| The last lap after the winner is timed | `the_player_s_last_lap_after_the_winner_is_timed_in_deadrally` | **Pass.** As DeadRally the lap becomes the best lap and calls the record. As the Windows version neither happens, as in the original. |
| Up and Down on the records | `up_and_down_show_deadrally_s_race_records_of_each_count_of_laps`, then `the_race_records_and_a_grown_hall_of_fame_show_as_deadrally` | **Pass**, and the first failed without the kinds. Down steps from the lap's records to 4, 5 and 6 laps and round again, Up the other way. Left and Right keep the kind where the circuit has it: the Arena has the lap's and 9 laps. The third heading reads the laps ("4 LAPS:") over the bar's "LAP TIME:"; the title stays the circuit's name, because "Hell Mountain - 6 laps" did not fit its box. |
| The Hall of Fame grows and scrolls | `a_won_game_grows_the_hall_of_fame_and_nobody_drops_out`, `up_and_down_scroll_a_hall_of_fame_grown_past_ten`, then `the_race_records_and_a_grown_hall_of_fame_show_as_deadrally` | **Pass**, and the scrolling test failed without the scrolling. With thirteen entries, Down scrolls to ranks 4 to 13 and no further; Up goes back. With ten, Up and Down still leave as any key. |
| A winner after the tenth enters | `a_winner_after_the_tenth_still_enters_the_hall_of_fame`: the won Arena as DeadRally, every entry at one race | **Pass.** The winner is the eleventh. The entry screen shows ranks 2 to 11 with the border round the winner's row (checked by eye). |
| The runs against the original | every manifest of the Windows version's runs | **Pass.** Unchanged. |
