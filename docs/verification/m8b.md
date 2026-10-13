# M8b — personal best: verification

The [design](../design/2026-10-13-personal-best-design.md) goes beyond the original; the checks are DeadRally's own. Where a row says so, its test failed with the behaviour taken out. The screens were checked by eye in the game (`deadrally-headless render --deadrally`), then pinned. The runs checked against the original play as the Windows version and keep their manifests unchanged.

## Results

| Check | How | Result |
|---|---|---|
| The personal best's data | `a_personal_best_keeps_the_player_s_better_times_by_circuit_laps_and_car`, `a_saved_game_carries_its_personal_best_after_the_original_s_bytes` | **Pass.** A first time and each better one are kept, a worse one or no lap is not, and laps, races and cars stay apart. The block after the original's 2179 bytes leaves the original's game whole. |
| A race's times kept, then saved with the game | `a_race_s_times_go_into_the_game_s_personal_best`: M6's leader turn as DeadRally, F2 held in the shop afterwards | **Pass.** The quick save holds the Dervish's Holocaust lap, 00:31.07, and its 4-lap race, 02:05.58, and nothing else there. |
| Loading, saving, a new game | `a_saved_game_carries_its_personal_best_as_deadrally`, `a_new_game_starts_with_no_personal_best` | **Pass**, and each failed with the load or the reset taken out. A loaded personal best is saved again, a game the original saved has none, the Windows version writes the original's file alone, and a new game starts with none. |
| After the statistics | `the_statistics_lead_on_to_the_personal_best_and_back_to_the_menu`, `escape_on_the_statistics_and_the_windows_version_go_back_to_the_menu` | **Pass**, and the first failed with the way on taken out. Escape, and any key as the Windows version, still go back to the menu. |
| The records held | `the_statistics_count_the_records_the_player_holds_by_name` | **Pass.** Counted by the name as the records keep it; the Arena's and the races' count, Remedy's do not. |
| The screens | `the_personal_best_follows_the_statistics_as_deadrally` | **Pass.** Checked by eye: the statistics with "Lap records : 3" and "Race records : 1", the personal best's first page (Suburbia), Palm Side's with the Vagabond's 00:14.02, 00:58.21 and 01:13.90, and the menu back on the statistics' row. |
| The runs against the original | every manifest of the Windows version's runs | **Pass.** Unchanged. |
