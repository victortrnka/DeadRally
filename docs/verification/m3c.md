# M3c: verification against the original

The checks of the M3c spec (section 4), run against the original `dr.exe` under Wine as in [M3b](m3b.md), with `scripts/reference-run.sh --seed 1 --save 0:FILE` and M3b's test game. Screenshots stay under `captures/`.

## Results

| Check | Scenario | Result |
|---|---|---|
| The shop's purchases | `shop-purchases`, seed 1, the test game | **Pass.** All 46 shots equal one of our frames: an engine and a tire upgrade to their maximum, two armour upgrades, five repairs down to 0 %, the descriptions coming back 310 passes later, a car too dear (the money short by $25072), a cheaper one offered with its refund and declined, offered again, bought and painted, and the car box on the next car. |

## What the original does that the decompilations do not say

- **A repair below 10 % damage** costs damage × (repair price / 10), halved with weapons, while the box shows damage × ((repair price / 10) halved): the two differ by a few dollars for odd prices.
- **Upgrades and repairs add their price to the car's worth**, which the dealer's refund is a quarter of.
- **The dealer's offer has two wordings:** when the refund is more than the new car's price, it says how much comes back instead of what the car costs.

## Key ticks

Every key of the scenario has a shot after it; the keys were put on the 14 ms line from the menu's idle shot and then moved, one at a time, by the smallest shift (none further than one tick) that matched the shots up to the next key.

## The manifest

`crates/headless/tests/shop-purchases-run.sha256` holds our frames at the 46 shots' ticks, the run's sound and the last `dr.cfg` written.
