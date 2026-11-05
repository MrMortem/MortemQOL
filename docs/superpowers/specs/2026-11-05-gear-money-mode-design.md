# MortemQOL Gear Menu and Money Mode: Design

Date: 2026-11-05
Status: awaiting review
Builds on: `2026-11-04-mortemqol-calculator-design.md`, as shipped in v1.0.0

## Goal

Let the player turn the assistance on and off from a gear menu, and add a money mode that recommends the play earning the most money from the current hand, with score as the tiebreak. Lay the engine groundwork for two later features, suggested discards and seeded mode, without shipping either.

**Success means:** predicted money matches the money the game actually awards for a hand, as score predictions already do, and Score mode's results are unchanged.

## Decisions

| Topic | Decision |
|---|---|
| Where the controls live | A gear icon at the top left of the screen during a run. |
| Assistance off | No panel, badges, searches or live check. Replaces the "Show calculator panel" toggle. |
| Modes | Score (default) and Money. The choice is remembered between sessions. |
| Money scope | Money gained or lost by playing this hand only. |
| Money ranking | Highest expected money, then highest expected score. |
| Unbuilt features | Hidden from the menu until they work. |
| Deck order and RNG | The v1 rule stands: never read deck order or predict RNG. |

## Out of scope

- **Suggested discards** and **seeded mode** as features. Only the groundwork below ships.
- End-of-round money: held Gold Cards, interest, leftover-hand money, Golden Joker and similar.
- Money from discards, and planning across hands.
- Modeling PvP's jokers. Lobby results stay labeled "Approximate".

## Gear menu

- **Gear icon:** a small sprite in the empty space above the blind info at the top left, shown only during a run. It's clickable in every run state, but its menu only affects the panel during hand selection.
- **Menu:** clicking the gear opens a small panel beneath it, without pausing the game. Clicking the gear again closes it.
- **Controls:**
  - **Assistance:** on or off.
  - **Mode:** Score or Money.
- **Config tab:** the mod's Config tab shows the same two settings, so both places change the same values.
- **Saving:** settings save with Steamodded's mod config when changed. An existing `show_panel = false` setting carries over as Assistance off.
- **Sprite:** `assets/1x/gear.png` and `assets/2x/gear.png`, loaded as a Steamodded atlas and included in the release zip. If the sprite fails to load, the gear falls back to a small text button reading "QOL", so the controls stay reachable.

### Turning assistance off

- Any running search is abandoned, and the panel and badges are removed.
- The mod stops building snapshots and stops running the live check.
- Turning it back on during hand selection starts a fresh search right away.

## Money mode

### What counts

| Source | Effect |
|---|---|
| Gold Seal | +$3 per scoring trigger |
| Lucky Card | Chance of +$20 per scoring trigger |
| Golden Ticket | +$ per scoring Gold Card trigger |
| Rough Gem | +$ per scoring Diamond trigger |
| Business Card | Chance of +$2 per scoring face card trigger |
| Reserved Parking | Chance of +$ per held face card trigger |
| To Do List | +$ when the listed hand is played |
| Matador | +$ when the boss blind's ability triggers, including on a blocked hand |
| The Tooth | -$1 per played card |
| The Ox | Lose all money when the most-played hand is played |

Retriggers apply as they do in scoring. Chance effects use the existing floor, expected and ceiling evaluations, so money gets a range the same way score does.

**The Ox:** money lost is the player's whole balance at play time, so a play that triggers it ranks below plays that don't unless other effects outweigh that loss.

### Blocked hands

A hand blocked by a boss blind scores zero, but it can still change money: Matador pays, and The Tooth and The Ox still apply. The engine currently returns zero as soon as it finds a blocked hand, and it must calculate these effects instead. A blocked hand stays out of Score mode's recommendations, as it is now. In Money mode it's a valid candidate, and the panel marks it "Blocked: scores 0".

### Search

- Plays and joker orders use the same three search stages as Score mode.
- Candidates compare by expected money, then expected score, then floor score.
- With no money available, Money mode's recommendation matches Score mode's.
- Money gained during a hand also changes Bull and Bootstraps. The engine already models this and it needs no change.

### Panel

- **Money mode:** the top line shows the expected money, such as "+$4.50", with a range when chance is involved. The hand name and score follow. The current selection's line shows both money and score.
- **Reorder line:** in Money mode this reads "Reorder jokers: +$X" when the order adds money, and otherwise shows the score gain.
- **Score mode:** looks the same as v1, with no money line.

### Blind warning

On the last hand, if the recommended play's expected score plus the round's score so far doesn't reach the blind, the panel shows "Won't beat the blind". The recommendation doesn't change. This only appears in Money mode.

## Groundwork for later features

Each of these ships without a visible feature.

### Search options

Search requests gain an `options` object:

```json
{"objective": "score", "discards": false, "seeded": false}
```

`objective` is `"score"` or `"money"`. A missing `options` object means Score mode with the other two off. The engine reports an error if `discards` or `seeded` is true, since neither is implemented. The ABI version stays at 1 because the C functions don't change.

### Chance as a swappable part

Today, floor, expected and ceiling are hardcoded through `Mode` in `ScoreCtx`. Chance effects move behind one small interface, with those three as its implementations. A later seeded implementation can then supply real outcomes without changing joker code. Score results must stay identical, and the existing tests prove it.

### Results carry money and score

`Scored` gains `money_floor`, `money_expected` and `money_ceiling`, which `mqol_score` returns. Search results include the same fields on `best`. Money in this design is the change from the player's balance, so it can be negative.

### Deck contents as counts

Snapshots gain `deck.contents`: how many of each remaining card there are, grouped by rank, suit, enhancement, edition and seal. This is the information the game's deck view already shows, and it's what suggested discards will need to estimate draws.

- This is the first time the mod reads individual cards in the deck, so it has a strict rule: it counts them into groups, and sorts the groups, so that **no order can survive**.
- A test shuffles the same deck and requires identical output.
- The engine parses this field and doesn't otherwise use it in this release.

## Seeded mode: decisions deferred

Seeded mode would read exact future draws and chance outcomes, which reverses the v1 rule. This release neither reads that information nor adds any code path to it. When it's built, its spec must decide:

- How it behaves in multiplayer lobbies, where it's a much larger advantage than a score calculator and players may share a seed. The recommendation on record is to keep it off in lobbies.
- How it's labeled, so a run using it is clearly identified.
- Whether it's off by default.

## Live check

The live check gains a money comparison.

- **Before the play:** record the balance alongside the score prediction.
- **After the play:** when the game leaves its hand-played state, and before end-of-round payouts, compare the balance change with the prediction.
- **Rules:** with no chance effects, the amounts must match. With chance effects, the actual amount must be within the range. Approximate results are logged separately, as they are for score.
- **Log:** a money disagreement writes to `verify.jsonl` with `actual_money` added, and counts as a mismatch. A replay test covers these lines too.

## Error handling

| Failure | Behavior |
|---|---|
| Missing or invalid settings | Fall back to Assistance on, Score mode. |
| Engine given `discards` or `seeded` as true | Error status; the panel shows "Engine unavailable". The UI never sends this in this release. |
| Gear sprite fails to load | The text button appears instead. |
| Error in menu code | Caught and logged, like other mod code. |

## Testing

**Automated:**

- A money test for each source in the table, including retriggers, Blueprint copies and each chance range.
- Blocked hands: Matador pays, Tooth and Ox still apply, and the score stays zero.
- Search: money before score; ties go to score; no money available matches Score mode; blocked hands appear only in Money mode.
- Reorder gain and the blind warning in Money mode.
- Regression: all existing score tests pass unchanged.
- Snapshot: shuffling the deck doesn't change `deck.contents`, and face-down cards in hand stay masked.
- Options: missing options mean Score mode, and `discards` or `seeded` as true returns an error.
- Settings: missing and invalid values, and the `show_panel` carryover.
- Live check: money match, mismatch, range and log line.

**In the real game, through the autoplay script:**

- Random hands in Money mode with zero mismatches for both score and money.
- Forced scenarios for Tooth, Ox, Matador on blocked hands, Gold Seal retriggers and To Do List.

**By the user, since the script can't use a real mouse:**

- The gear opens and closes the menu, and both controls work.
- Assistance off removes everything, and on brings it back.
- The gear doesn't overlap the blind info at the sizes they play at.
- The chosen mode survives a restart.

## Risks

- **Timing of the money measurement:** money arrives through delayed game events. The check has to read the balance after those events and before end-of-round payouts, which the in-game scenarios must confirm.
- **Other mods changing money:** PvP and other mods can pay out in ways the engine doesn't model. Lobby results stay approximate, and an unknown joker already labels results that way.
- **Reading deck cards:** a mistake in `deck.contents` could leak order. The shuffle test guards against that.
- **Money mode can lose a run:** this follows from ranking money first. The last-hand warning makes it visible.
