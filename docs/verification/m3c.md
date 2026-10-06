# M3c: verification against the original

The checks of the M3c spec (section 4), run against the original `dr.exe` under Wine as in [M3b](m3b.md), with `scripts/reference-run.sh --seed 1 --save 0:FILE` and M3b's test game. Screenshots stay under `captures/`.

## Results

| Check | Scenario | Result |
|---|---|---|
| The shop's purchases | `shop-purchases`, seed 1, the test game | **Pass.** All 46 shots equal one of our frames: an engine and a tire upgrade to their maximum, two armour upgrades, five repairs down to 0 %, the descriptions coming back 310 passes later, a car too dear (the money short by $25072), a cheaper one offered with its refund and declined, offered again, bought and painted, and the car box on the next car. |
| The Underground Market | `market`, seed 1, the test game | **Pass.** All 60 shots equal one of our frames: the shop fading out and the market in (15 shots on the way), the moves along the bottom row and up to the loan shark, his $1500 loan and its paying back, the mines, spikes, rocket fuel and sabotage bought and sold out, the messages giving way to the descriptions, Escape and the shop fading back in (7 shots), the market again and on to the sign-up, wiped in over it. |

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
- **A screenshot can match two key ticks:** the pulse runs up and down, so the same colours come back on both sides of its turn. The market's run first matched the shop with the space a pass late; the flag's frame at the next key showed it, and the space was put back two passes earlier.

## Key ticks

Every key of the scenario has a shot after it; the keys were put on the 14 ms line from the menu's idle shot and then moved, one at a time, by the smallest shift (none further than one tick) that matched the shots up to the next key. In the market's run the space before the shop sits four ticks before its place on the line and every key after it two ticks before (the shop's first pass and the pulse's turn, above).

## The manifest

`crates/headless/tests/shop-purchases-run.sha256` holds our frames at the 46 shots' ticks, the run's sound and the last `dr.cfg` written; `market-run.sha256` the same for the market's 60 shots.
