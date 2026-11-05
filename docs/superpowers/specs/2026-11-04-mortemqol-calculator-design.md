# MortemQOL Live Calculator: Design

Date: 2026-11-04
Status: awaiting review

## Goal

A Balatro mod that shows, live during a blind, the highest-scoring play available
from the current hand and whether reordering jokers would beat it. It installs
through r2modmac from a public GitHub repository.

**Success means:** the score the mod predicts for a play matches what the game
awards for that play, and the answer appears quickly enough to feel live.

## Decisions

| Topic | Decision |
|---|---|
| Name and repo | `MortemQOL`, at `MrMortem/MortemQOL` (public, MIT). The calculator is its first feature. |
| What it optimizes | Best cards to play now, the order of those cards, and joker order. |
| Engine | Own scoring model in Rust, loaded through LuaJIT FFI. It never runs the game's scoring code. |
| Assurance | Every real play is compared with its prediction, and mismatches are logged. |
| Autonomy | Advice only. It may select cards on request. It never plays a hand or moves a joker. |
| Multiplayer | Stays on in lobbies. Players are expected to agree to it; ranked is handled by disabling the mod in the r2modmac profile. |

### Why Rust

Balatro calls `jit.off()` in `conf.lua`, and nothing in the installed mod set turns
it back on, so Lua code in the game runs interpreted. A synthetic scoring-shaped
loop ran about 9x slower with JIT off (13 ms vs 1.5 ms). A native library also runs
on its own thread, which keeps the search out of the game's frame time.
`MultiplayerAPI` already ships native libraries loaded by FFI, so the approach has
precedent in this mod set.

## Out of scope for v1

- Discard advice and whole-round planning
- Modeling jokers added by other mods, including PvP's custom jokers
- Talisman-scale numbers; scores are `f64`, and overflow displays as `naneinf`
- A Thunderstore listing and one-click updates
- A native Linux build; Linux players run the Windows game through Proton, which
  uses the Windows library

## Architecture

```
Game state --> snapshot.lua --JSON--> Rust engine (worker thread)
                                         |
ui.lua <-- bridge.lua (polls each frame) <-- result JSON
verify.lua: prediction before a real play vs. the game's final score --> verify.jsonl
```

### Repository layout

```
manifest.json          Thunderstore-style manifest, read by r2modmac on import
MortemQOL.json         Steamodded metadata; depends on Steamodded
MortemQOL.lua          Entry point
src/snapshot.lua       Reads visible game state into a plain table; writes nothing
src/json.lua           Small JSON encoder/decoder
src/bridge.lua         Loads the native library, starts searches, polls results
src/ui.lua             Panel, joker and card badges, Select button, config tab
src/verify.lua         Compares predictions with real scores
engine/                Rust crate, built as a cdylib; no game knowledge beyond the snapshot
engine/tests/          Golden scoring tests and replayed snapshots
docs/jokers.md         Coverage table for all 150 vanilla jokers
.github/workflows/     CI on every push; release build on version tags
```

### Native interface

Plain C ABI with JSON strings, so the Lua side stays small and the snapshot format
can evolve without changing the ABI:

- `mqol_abi_version() -> u32`: checked on load; a mismatch counts as a load failure.
- `mqol_search_start(json, len) -> u64`: starts a search and returns its id. A newer
  search cancels any older one.
- `mqol_search_poll(id, out, cap) -> i32`: returns a status (running, finished or
  error) and writes the latest result, including provisional ones.
- `mqol_score(json, len, out, cap) -> i32`: scores one specified play synchronously.
  Used for the current selection and for predictions.

No panic may cross the boundary: every entry point and the worker use
`catch_unwind` and report an error status instead.

## Snapshot

`snapshot.lua` reads only what the player can see, and never mutates game state or
touches the RNG.

**Included:**

- Hand cards in order: rank, suit, enhancement, edition, seal, permanent bonuses,
  debuffed state, forced selection
- Jokers in order: key, edition, debuffed state, ability values, sell value
- Hand levels and play counts
- Blind key and whether it is disabled
- Hands left, discards left, money, and blind target
- Deck type, vouchers that affect scoring (such as Observatory), and consumables
  those vouchers act on
- Deck composition as counts, which the player can view in-game

**Excluded or masked:**

- **Deck order** is never read.
- **Face-down hand cards** are sent as unknown. The search does not choose them, and
  any result in a hand containing them is labeled approximate.
- **Face-down jokers** (Amber Acorn) are sent as unknown and labeled approximate.

A snapshot is rebuilt only when the state may have changed, in the
hand-selection state. If the resulting JSON matches the previous one, no search starts.

## Engine

### Scoring model

The engine follows the order in the game's source and Steamodded 26.1002.0:

1. Detect the poker hand and scoring cards, honoring Four Fingers, Shortcut,
   Smeared Joker, Pareidolia, Splash and Stone cards.
2. Apply blind effects that act before scoring (The Flint, The Arm, The Psychic,
   The Eye, The Mouth, and so on) and deck effects such as Plasma's balancing.
3. Apply `before` joker effects, including in-hand scaling (Green Joker, Ride the Bus,
   Square Joker, Runner, Spare Trousers, and others).
4. Score each scoring card left to right: base chips, enhancement, edition, seal,
   per-card joker effects and retriggers.
5. Apply held-in-hand effects (Steel, Baron, Mime, Shoot the Moon, Raised Fist).
6. Apply joker effects left to right, including editions, Blueprint and Brainstorm.
7. Apply end-of-scoring deck effects.
8. Track money gained during the hand for jokers that read it (Bull, Bootstraps).

An illegal play under the current blind scores zero, as it does in the game.

### Chance effects

Each play gets three deterministic evaluations: **floor** (no chance effect fires),
**ceiling** (all fire at their highest), and **expected** (each effect replaced by
its mean, such as Lucky +4 Mult or Misprint +11.5). Oops! All 6s doubles
probabilities, capped at 1.

Independent effects multiply out correctly under this method. Space Joker is the
exception: its level-up changes chips and mult together, so the engine evaluates
both outcomes and weights them.

Plays rank by **expected** score, with ties going to the higher floor. The engine
uses no RNG, so results are reproducible and the game's seed is unaffected.

### Search

The worker publishes a result after each stage:

1. **All plays, current joker order.** Every legal subset of 1 to 5 known cards,
   including any forced card. Provisional result.
2. **All plays, all joker orders.** With 6 or fewer jokers it tries every permutation,
   skipping those that cannot differ. With more it uses swap-based hill climbing and
   labels the result "best found".
3. **Card order for the top 32 plays.** Played cards score in the order they sit in
   hand, so this only runs when a position-sensitive effect is present (Photograph,
   Hanging Chad, or mixed add and multiply effects).

The result holds the best play, its card order, score range, hand name, percentage
of the blind, best joker order, gain over the current order, and approximate flags.

**Performance target:** a full search of 8 cards and 5 jokers finishes within 100 ms
on Apple Silicon in a release build, measured by a benchmark. If the target is
missed, stage 2 narrows to the top plays from stage 1.

## In-game UI

- **Panel:** small and movable. Shows the best hand, score (with min-max if chance is
  involved), percentage of the blind, and the score of the current selection.
- **Joker order:** when reordering helps, shows the gain and numbered badges on jokers.
- **Card order:** when position matters, shows numbered badges on the recommended cards.
- **Select button:** selects the recommended cards. It does not reorder cards or play.
- **Config tab:** panel on/off.
- **States:** "calculating", "approximate" (with the reason available on hover), and
  "engine unavailable".

## Live check

Before a real play, `verify.lua` takes a snapshot and scores the play with
`mqol_score`. After scoring, it compares the prediction with the game's final
chips times mult.

- With no chance effects, the scores must match within a relative tolerance of 1e-9.
- With chance effects, the actual score must fall between floor and ceiling.
- A mismatch appends the snapshot, prediction and actual score to `verify.jsonl` in
  the mod's save folder. Each entry can be replayed as a regression test.
- Results labeled approximate are logged separately and do not count as failures.

## Multiplayer

- The calculator stays active in lobbies.
- The mod appears under its real name in the PvP mod's hash. Nothing hides it, and
  nothing works around `PVP.BANNED_MODS` if it is added there. It is not on the list
  as of PvP 1.0.4.
- The README states that it must be disabled in the r2modmac profile for ranked play.
  That manual step is the only safeguard, by design.
- PvP changes scoring: it defines custom jokers, alters some vanilla ones (at least
  Ice Cream and Seltzer) and has rulesets that change the game. In any lobby, or
  whenever an unknown joker is present, results are labeled approximate.

## Error handling

| Failure | Behavior |
|---|---|
| Library missing, blocked, or wrong ABI version | Panel shows "engine unavailable"; the game is unaffected. There is no Lua fallback engine. |
| Unknown joker | Treated as no effect; result labeled approximate. |
| Engine error or panic | Caught at the FFI boundary; panel shows "unavailable"; next state change retries. |
| Error in mod Lua code | Wrapped in `pcall` and logged, so it cannot crash a run. |
| Malformed snapshot | The engine returns an error status. |

## Distribution

- CI runs tests and lints on every push.
- A `v*` tag builds a macOS universal library (arm64 and x86_64, ad-hoc signed) and
  a Windows x64 library, then publishes `MortemQOL-<version>.zip` as a GitHub release.
- The zip has `manifest.json` at its root, which r2modmac's custom mod import needs.
- **Install:** download the zip from Releases, then import it in r2modmac. r2modmac
  is expected to warn that the mod contains loadable files. Steamodded and Lovely
  must already be in the profile.

## Testing

**Automated:**

- Golden tests with one or more cases per scoring joker, plus enhancements, editions,
  seals, hand detection and boss blinds. Expected values are derived from the game's
  source, and `docs/jokers.md` tracks coverage.
- Property tests: floor <= expected <= ceiling; order-insensitive setups give equal
  scores across joker orders; search results are never below any play scored directly.
- Replayed `verify.jsonl` snapshots.
- A smoke test that loads the built library from `luajit` through FFI.
- A benchmark for the performance target.

**Manual, in the real game:**

- Imported through r2modmac from a real downloaded release zip, to confirm macOS
  loads the library. This is an open risk until tested; if macOS blocks it, the
  README gets the required step or the distribution changes.
- A complete vanilla run with zero mismatches in `verify.jsonl`. This is the bar
  for calling v1 finished.
- RNG unchanged: `G.GAME.pseudorandom` is identical before and after a search.
- No visible frame drops during a search.
- A private PvP lobby, confirming results are labeled approximate and nothing breaks.

## Risks

- **macOS blocking the library:** covered by the release test above.
- **Game or Steamodded updates changing scoring:** the live check surfaces this as
  mismatches.
- **PvP banning the mod:** accepted; it would need to be removed for those lobbies.
- **Windows build untested on real hardware:** CI proves it compiles and passes tests,
  but this Mac cannot confirm it loads in the game. The first Windows player is the
  real test, and the release notes say so.
