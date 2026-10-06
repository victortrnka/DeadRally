# M3c — Buying, the Underground Market and the loan shark: design

- **Date:** 2026-10-06
- **Status:** written without the owner, who asked to go on milestone after milestone without stopping; section 2 lists the decisions taken on their behalf; [docs/verification/m3c.md](../../verification/m3c.md) records the checks against the original.
- **Scope:** the last part of milestone M3: the shop's purchases (engine, tire and armour upgrades, repairs, a new car with its paint), the Underground Market with its weapons and the loan shark, and the hitman's offer after a sign-up.
- **Builds on:** [M3a](2026-10-06-m3a-new-game-design.md), [M3b](2026-10-06-m3b-saves-shop-design.md).

## 1. Goal

Everything the player can buy between races works as in the original: the same prices, refunds, messages and animations, the same money afterwards, pixel for pixel, and a game saved after shopping is the original's file byte for byte.

## 2. Decisions taken on the owner's behalf

1. **M3c comes in parts on one branch:** the shop's purchases first, then the Underground Market and the loan shark, then the hitman's offer. *Cost if wrong:* none; the order of work only.
2. **The final race against the Adversary** (the player ahead of every other driver on points when going on from the shop) leads to the sign-up until M6 brings that race. *Cost if wrong:* none until then; points cannot change before races exist.

## 3. Facts about the original

- **Upgrades** (`enterShop` 0x4373B0, engine 0x43805F): the price checked against the money (`hasInsuficientMoneyToBuy` 0x421E50 writes how much is missing under the popup's title and sounds effect 23 on channel 2), then paid and added to the car's worth; the box shows the next level (or the "no more" picture) with its price and what was bought (`reloadEngineAnimation` 0x4212F0 and the two after it, texts 240 bytes a level from 0x450DB8, 0x451538, 0x451CB8); 310 shop passes later the item's description comes back (`framesToWaitAfterBuy` 0x456B70, car and upgrades only).
- **Repairs** (0x438383): ten points of damage, what is left under ten, at the price the repair box shows (computed in another order than the box's, so the two can differ by a few dollars under 10 % damage); paid and added to the car's worth; effect 31.
- **A new car** (0x4374A5): the refund is a quarter of the car's worth rounded up, less the damage's repair, never negative, rounded down to tens; the popup offers the car for the price less the refund (or returns the difference when the refund is more), "yes" selected; the car turns and the cursor beside the answer turns, two waits a pass. "Yes" trades the car in (upgrades and damage gone), redraws every box and asks for the paint: Left and Right move the colour by 2 (0 to 254), two waits a pass; Enter ends it, the car box moves to the next car and says what was bought (`showCarBought` 0x4210C0).
- **Continue** (0x4384BE): a wrecked car (100 % damage) cannot go on without weapons; with weapons the Underground Market (0x436700) comes first; without, effect 24 and the sign-up, or the Adversary when the player leads every other driver on points.

## 4. Verification

The test game of M3b, loaded into the original (`reference-run.sh --save 0:FILE --seed 1`), goes through every purchase: an engine and a tire upgrade to their maximum, two armour upgrades, the repair down to nothing, a car too dear, and a cheaper car declined, then bought and painted. Every shot must equal one of our frames.

## 5. Tests

Data-free: an upgrade is paid for and adds to the car's worth; short of money nothing changes. A data test runs the scenario's keys (manifest `shop-purchases-run.sha256`).
