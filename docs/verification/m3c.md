# M3c: verification against the original

The checks of the M3c spec (section 4), run against the original `dr.exe` under Wine as in [M3b](m3b.md), with `scripts/reference-run.sh --seed 1 --save 0:FILE` and M3b's test game. Screenshots stay under `captures/`.

## Results

| Check | Scenario | Result |
|---|---|---|
| The shop's purchases | `shop-purchases`, seed 1, the test game | **Pass.** All 46 shots equal one of our frames: an engine and a tire upgrade to their maximum, two armour upgrades, five repairs down to 0 %, the descriptions coming back 310 passes later, a car too dear (the money short by $25072), a cheaper one offered with its refund and declined, offered again, bought and painted, and the car box on the next car. |
| The Underground Market | `market`, seed 1, the test game | **Pass.** All 60 shots equal one of our frames: the shop fading out and the market in (15 shots on the way), the moves along the bottom row and up to the loan shark, his $1500 loan and its paying back, the mines, spikes, rocket fuel and sabotage bought and sold out, the messages giving way to the descriptions, Escape and the shop fading back in (7 shots), the market again and on to the sign-up, wiped in over it. |
| The sabotage | `sabotage`, seed 1, the test game, `--sabotage-clock 31375` | **Pass.** All 23 shots equal one of our frames with our clock fixed the same way: the sabotage bought, the sign-up, the places filling and the popup with the best-ranked rival in the player's race 42 % damaged. A first run with the clock at 31347 showed 25 %, as our draw gives for that clock. |
| The hitman's offer | `offer`, seed 106 (a seed where he comes), the test game | **Pass.** All 24 shots equal one of our frames: the sign-up, the hitman's picture and lines with the victim drawn from the player's race and the pay for the player's car, the question after 70 waits, "no" and "yes" selected. |
| Quick save and load | `quick-save`, seed 1, the test game, F2 and F3 held 300 ms | **Pass.** All 22 shots equal one of our frames: "game saved" over the shop, an engine bought, "game loaded" with the engine and the money back, the market and "game saved" there. The run's `DR.SG7` equals ours byte for byte. |

## What the original does that the decompilations do not say

- **A repair below 10 % damage** costs damage × (repair price / 10), halved with weapons, while the box shows damage × ((repair price / 10) halved): the two differ by a few dollars for odd prices.
- **Upgrades and repairs add their price to the car's worth**, which the dealer's refund is a quarter of.
- **The dealer's offer has two wordings:** when the refund is more than the new car's price, it says how much comes back instead of what the car costs.
- **The market's fades leave entries 96–127 alone** and fade the rest of the composed palette, and the fade in stops at 98 %: the market (and the sign-up after it) keeps the player's colour ramp at 98 % until something sets it again. The side panel (`drawCarRightSide` 0x41FC20) never sets that ramp; DeadRally did, which the earlier runs could not show.
- **Composing the palette reads the player's colour from their record** each time (0x4224E0), so a loaded game's colour shows in the fades without the licence having set it.
- **The sign-up shows only the copper ramp, entries 176–182, at full brightness** when it opens (0x435806), not 16 entries.
- **The sabotage is sold out while the player leads** every other driver on points (0x436783), and a loaded game sells only what the car is not full of (0x42F6A1): mines below 8, the others not fitted.
- **The loan shark** lends 12000, 9000, 6000, 3000 or 1500 by car (the best car the most; the Vagabond's driver gets nothing). What is owed grows by a third of half the loan each race; Windows computes it in doubles and truncates, dRally (`underground___2e350h.c`) in integers: the two first differ by a dollar from the fifteenth race after the loan.
- **The way on with a wreck** says the shop's message with another last line (0x4440FC).
- **The shop's fade back in after the market** turns the continue flag only when the player could race: no loan due, the money with the car's trade-in value at least 1000, the money for a repair, at most 95 % damage (0x438C13).
- **After a sign-up** the sabotage's popup or an offer replaces the screen's linger; the sabotage seeds `rand()` again from the clock, so the damage changes with the moment (25 % at 31347 ms, 42 % at 31375). The offer's question ignores Escape. A drug run pays twice the hit's pay for the same car.
- **F2 and F3 are read as held keys** each pass of the shop and the market; a tap shorter than a pass is missed, so the scenario holds them (`keydown`, `keyup`). After the confirmation the screen comes back as it was under it.
- **A screenshot can match two key ticks:** the pulse runs up and down, so the same colours come back on both sides of its turn. The market's run first matched the shop with the space a pass late; the flag's frame at the next key showed it, and the space was put back two passes earlier.

## Key ticks

Every key of the scenario has a shot after it; the keys were put on the 14 ms line from the menu's idle shot and then moved, one at a time, by the smallest shift (none further than one tick) that matched the shots up to the next key. In the market's run the space before the shop sits four ticks before its place on the line and every key after it two ticks before (the shop's first pass and the pulse's turn, above); the later runs' keys sit up to two ticks either side of the line likewise.

## The manifest

`crates/headless/tests/shop-purchases-run.sha256` holds our frames at the 46 shots' ticks, the run's sound and the last `dr.cfg` written; `market-run.sha256`, `sabotage-run.sha256`, `offer-run.sha256` and `quick-save-run.sha256` the same for the other runs (the quick save's also with the `DR.SG7` it writes).

## Review

A fresh review of the branch found nothing Critical or Important; graded by what a player gets, four of its findings were fixed with tests that failed first: F2 and F3 count as let go when a confirmation ends (0x42DC70), so a quick save held through "game saved" happens once; a saved game with an odd colour is refused like a damaged one (the paint would step it below 0); a hand-made loan count or car worth wraps as the original's ints do instead of stopping the game; and a comment no longer quotes the dealer's words. The test of a quick load without a quicksave now checks the confirmation is shown. Deferred (minor): the quick keys are read only on passes that stay in the shop or the market (the original also reads them on the pass that leaves, and right after a car's turn, offer or paint); the shop's fade back in from the market works out the trade-in value afresh where the original keeps the one from the shop's start; the sabotage test does not pin its victim; the 14 ms tick is written in three places.
