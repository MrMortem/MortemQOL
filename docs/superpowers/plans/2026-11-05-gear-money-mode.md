# Gear Menu and Money Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a gear menu that turns assistance on and off and selects Score or Money mode, with money from the current hand as a new search objective, and engine groundwork for suggested discards and seeded mode.

**Architecture:** The engine returns money alongside score from one evaluation path, and search ranks by an objective read from the snapshot's `options`. Lua gains a tested settings module and a tested panel-text builder, so `ui.lua` and `gear.lua` hold only game UI code. The live check gains a second, delayed comparison for money.

**Tech Stack:** Rust stable, LuaJIT, Steamodded 26.1002.0, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-11-05-gear-money-mode-design.md`

## Global Constraints

- Work on branch `feat/gear-money-mode`; merge through a PR on the user's approval. Commits use the repo-local noreply identity and end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. The PR body ends with `🤖 Generated with [Claude Code](https://claude.com/claude-code)`.
- The v1 rule stands: never read deck order, predict RNG or call the game's RNG. Face-down cards stay masked.
- Advice only: the mod never plays a hand or moves a card or joker. Nothing hides it from PvP's mod hash.
- Score mode results stay identical to v1.0.0. All existing tests keep passing unchanged, except where a task names a test it replaces.
- Performance target stays: 8 cards and 5 jokers within 100 ms in a release build, in both modes.
- Dependencies stay `serde` and `serde_json`. ABI version stays 1.
- Money values are the change from the balance at play time, and can be negative.
- Before any in-game run: confirm Balatro isn't running, back up `~/Library/Application Support/Balatro/{1,2,3,settings.jkr}`, and move the user's `Mods/Local-MortemQOL-*` aside so it doesn't clash with the dev copy. Afterward, restore all of them and verify with `diff -r`.

## Review Focus

- Changing mode or turning assistance off during a search: a result from the old setting is never shown. Test in Task 4.
- Money mode where every play is blocked and earns nothing: no recommendation, rather than a zero-score zero-money play. Test in Task 3.
- Odd balances: The Ox with debt or at $0, and Bull with debt. The Ox brings debt up to $0, so its money is positive. Test in Task 2.
- Old or damaged config: a missing, invalid or legacy value falls back without an error. Test in Task 4.
- A play that never reaches its money comparison, such as quitting mid-hand: it doesn't get compared against a later play's balance. Test in Task 5.

---

## Shared Contracts

**Snapshot additions:**

```json
{"options": {"objective": "score", "discards": false, "seeded": false},
 "deck": {"contents": [{"rank": 14, "suit": "Spades", "enh": "m_glass", "ed": null, "seal": null, "count": 2}]}}
```

A missing `options` means Score mode. `snapshot::parse` returns an error when `discards` or `seeded` is true, or `objective` is unknown.

**`Scored` and `best` additions:** `money_floor`, `money_expected`, `money_ceiling`.

**Search result additions:** `gain_money`, `blocked` (the best play is blocked by the blind), `blind_warning` (see Task 3).

**Scope correction from the spec:** the spec says The Tooth and The Ox still apply on blocked hands. Only The Psychic, The Eye and The Mouth block hands, and only one blind is active, so those can't occur together. Matador is the only blocked-hand money source. Tasks test that, including Blueprint copies.

---

### Task 1: Chance interface and money results

**Files:**
- Modify: `engine/src/score.rs`, `engine/src/jokers.rs`
- Test: `engine/tests/score_money.rs` (create)

**Interfaces:**
- Produces in `score.rs`:
  - `pub trait Chance { fn weight(&mut self, p: f64) -> f64; fn pick(&mut self, min: f64, max: f64) -> f64; fn used(&self) -> bool; }`
  - `pub struct ModeChance { mode: Mode, used: bool }`, implementing `Chance` with today's floor, expected and ceiling behavior.
  - `ScoreCtx.chance: &'a mut dyn Chance`, replacing `mode` and `chance_used`. `j_misprint` uses `pick`.
  - `Scored` gains `money_floor`, `money_expected`, `money_ceiling`, also in `to_json`.
  - `pub fn score_expected(...) -> Expected`, with `pub struct Expected { pub score: f64, pub money: f64 }`, replacing the `f64` return.
  - Money from one evaluation is `ctx.dollars - s.round.dollars`. Space Joker's expected money uses the same weighted sum as score.

- [ ] **Step 1: Write failing tests in `score_money.rs`**

`gold_seal_earns_3`: AS with Gold Seal: floor, expected and ceiling money all 3, score still 16.
`no_money_is_zero`: AS: money 0 in all three.
`lucky_money_range`: AS Lucky: `money_floor` 0, `money_expected` 20/15, `money_ceiling` 20.
`money_serializes`: `to_json()["money_expected"]` equals 3.0 for the Gold Seal hand.
`custom_chance_drives_joker_effects`: a test `Chance` that always returns weight 1 gives the same score as `Mode::Ceiling` for AS Lucky with Bloodstone, called through `pub fn run_with(s, play, order, chance: &mut dyn Chance) -> Expected`.

- [ ] **Step 2: Run to verify they fail**

Run: `cd engine && cargo test --test score_money`
Expected: FAIL to compile.

- [ ] **Step 3: Implement the interface and money fields**

Keep every existing `ctx.prob`, `chance_mult`, `chance_x_mult` and `chance_dollars` call site; only their internals change. Update `search.rs` to read `.score` from `score_expected`.

- [ ] **Step 4: Verify the whole suite and the benchmark**

Run: `cd engine && cargo test && cargo run --release --example bench`
Expected: all existing tests pass unchanged; total under 100 ms.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "refactor(engine): put chance behind an interface and report money"
```

---

### Task 2: Every money source and blocked hands

**Files:**
- Modify: `engine/src/score.rs`, `engine/src/jokers.rs`
- Test: `engine/tests/score_money.rs`

**Interfaces:**
- Consumes: `Scored` money fields.
- Produces: `score_play` and `score_expected` calculate money for blocked hands; score stays 0 and `legal` stays false.

- [ ] **Step 1: Write failing and characterization tests**

Run each with `common::play`-style setup, default balance $4:

| Test | Setup | Money |
|---|---|---|
| `j_ticket_money` | scoring Gold Card, Golden Ticket extra 4 | 4 |
| `j_rough_gem_money` | AD, extra 1 | 1 |
| `j_business_money_range` | KS, extra 2 | floor 0, expected 1, ceiling 2 |
| `j_reserved_parking_money_range` | AS, held KH, odds 2, dollars 1 | floor 0, expected 0.5, ceiling 1 |
| `j_todo_list_money` | To Do hand matches, dollars 4 | 4; other hand 0 |
| `gold_seal_retrigger_money` | AS Gold + Red Seal | 6 |
| `blueprint_rough_gem_money` | Blueprint then Rough Gem, AD | 2 |
| `tooth_money` | 5 cards on `bl_tooth` | -5 |
| `ox_takes_balance` | $10, most played hand, `bl_ox` | -10 |
| `ox_with_gold_seal` | $10, AS Gold Seal | -7 |
| `ox_with_debt_brings_balance_to_zero` | -$5 | 5 |
| `ox_at_zero_costs_nothing` | $0 | 0 |
| `j_bull_with_debt_adds_nothing` | -$5, AS | score 16 |
| `matador_on_debuffed_card` | debuffed AS, extra 8 | 8 |
| `blocked_matador_pays` | `bl_psychic`, 1 card, extra 8 | money 8, score 0, `legal` false |
| `blocked_copied_matador_pays_twice` | Blueprint then Matador | 16 |
| `blocked_todo_list_pays_nothing` | `bl_psychic`, matching hand | 0 |
| `blocked_without_matador_is_zero` | `bl_psychic` | 0 |

- [ ] **Step 2: Run to verify which fail**

Run: `cd engine && cargo test --test score_money`
Expected: the `blocked_*` tests with Matador FAIL. Others may already pass because the engine tracks money for Bull; keep them as coverage, and fix any that fail.

- [ ] **Step 3: Handle blocked hands in `score.rs`**

Add `jokers::debuffed_hand_money(s, order) -> f64`, summing Matador's extra across `resolved_jokers`. No other phase runs on a blocked hand.

- [ ] **Step 4: Verify, then commit**

Run: `cd engine && cargo test`
Expected: PASS.

```bash
git add -A && git commit -m "feat(engine): count money for every source, including blocked hands"
```

---

### Task 3: Options, objective and money search

**Files:**
- Modify: `engine/src/snapshot.rs`, `engine/src/search.rs`, `engine/examples/bench.rs`
- Test: `engine/tests/search.rs`, `engine/tests/snapshot_parse.rs`

**Interfaces:**
- Produces:
  - `snapshot::Options { objective: Objective, discards: bool, seeded: bool }`, `pub enum Objective { Score, Money }`, `Snapshot.options` with a default of Score.
  - `snapshot::DeckGroup { rank: i32, suit: String, enh: Option<String>, ed: Option<String>, seal: Option<String>, count: f64 }` and `Deck.contents: Vec<DeckGroup>`, parsed leniently and not otherwise used.
  - `SearchResult` gains `gain_money: f64`, `blocked: bool`, `blind_warning: bool`; `Best` gains the three money fields.

- [ ] **Step 1: Write failing tests**

`snapshot_parse.rs`: `missing_options_mean_score`; `money_objective_parses`; `discards_true_is_error`; `seeded_true_is_error`; `unknown_objective_is_error`; `deck_contents_parse`; `missing_deck_contents_is_empty`.

`search.rs`, using a helper that sets `options.objective`:
- `money_mode_prefers_money_over_score`: hand "AS AH 2D" with 2D Gold Seal: picks a play containing 2D with `money_expected` 3, not the higher-scoring plain pair.
- `money_ties_go_to_score`: "AS AH 2D 3D" with 2D and 3D Gold Seals: among plays earning $6, picks the highest score.
- `no_money_matches_score_mode`: for every snapshot in `common::table()` with no money source, both modes give the same `expected` and `play`.
- `score_mode_ignores_money`: same hand as the first test picks the pair.
- `blocked_never_recommended_in_score_mode`: Matador on `bl_psychic` with 2 cards gives a null best.
- `blocked_recommended_in_money_mode`: same setup gives a non-null best, money 8, `blocked` true.
- `blocked_zero_money_is_not_recommended`: `bl_psychic` with no Matador in Money mode gives a null best.
- `money_joker_order_search`: "AD" with `[j_blueprint, j_joker, j_rough_gem]` reorders so Blueprint copies Rough Gem; `gain_money` 1.
- `money_card_order_search`: Hanging Chad with cards where the first scoring card's Gold Seal retrigger pays more; `card_order_matters` true.
- `blind_warning_on_losing_last_hand`: Money mode, `hands_left` 1, target 300, recommended score below it: true.
- `blind_warning_off_when_hands_remain`; `blind_warning_off_when_it_wins`; `blind_warning_off_in_score_mode`. Winning counts `round.chips` plus the play's expected score.
- `sort_handles_infinite_scores`: candidates mixing `naneinf` and finite scores complete without panicking.

- [ ] **Step 2: Run to verify they fail**

Run: `cd engine && cargo test --test search --test snapshot_parse`
Expected: FAIL to compile.

- [ ] **Step 3: Implement**

`Candidate` gains `money: f64` and `blocked: bool`. Replace `beats` with `beats(objective, a, b)`: Score compares expected score then floor; Money compares money, then expected score, then floor. Keep the existing tolerance for replacing a candidate. `top_first` sorts with `f64::total_cmp` on each field in the same order, which also resolves the deferred comparator issue; treat NaN as lowest. In Score mode, blocked plays are dropped as now. In Money mode, a blocked play is kept only if its expected money is above 0. Gain fields compare the same cards under the current and recommended joker orders, as `gain` does now. Add a Money-mode run to `bench.rs`.

- [ ] **Step 4: Verify the suite and benchmark**

Run: `cd engine && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && cargo run --release --example bench`
Expected: all pass; every benchmark under 100 ms.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(engine): add search options and a money objective"
```

---

### Task 4: Settings, panel text and live-loop changes

**Files:**
- Create: `src/settings.lua`, `src/panel_text.lua`, `tests/lua/test_settings.lua`, `tests/lua/test_panel_text.lua`
- Modify: `src/live.lua`, `src/snapshot.lua`, `src/format.lua`, `src/ui.lua`, `config.lua`, `MortemQOL.lua`
- Test: `tests/lua/test_live.lua`, `tests/lua/test_snapshot.lua`, `tests/lua/test_format.lua`

**Interfaces:**
- Produces:
  - `settings.init(config)`, `settings.assist() -> bool`, `settings.set_assist(bool)`, `settings.mode() -> "score"|"money"`, `settings.set_mode(string)`, `settings.options() -> table` for the snapshot. Setters save through the injectable `settings._save(config)`, which defaults to `SMODS.save_mod_config`.
  - `format.money(x) -> string`.
  - `panel_text.build(state, mode, mismatches) -> { hand, score, range, percent, selection, gain, status, money, warning }`, with strings, `""` when empty.
  - `snapshot.build()` includes `options` and `deck.contents`.
  - `MQOL.settings` is available to `live.lua`.

- [ ] **Step 1: Write failing tests**

`test_settings.lua`: `defaults_are_on_and_score`; `legacy_show_panel_false_means_off`; `assist_wins_over_legacy_show_panel`; `invalid_mode_falls_back_to_score`; `non_boolean_assist_falls_back_to_on`; `setters_update_and_save`; `set_mode_rejects_unknown`; `options_report_objective_and_false_flags`.

`test_format.lua` additions, for `format.money`: `3` → `"+$3"`; `4.5` → `"+$4.50"`; `-5` → `"-$5"`; `0` → `"$0"`; `"naneinf"` → `"naneinf"`.

`test_panel_text.lua`:
- `score_mode_has_no_money_line`, and matches v1's text for the same result.
- `money_mode_shows_money_first`: `money` is `"+$4.50"`, with `" ($0 to +$20)"` appended when floor and ceiling differ.
- `money_mode_selection_shows_both`: `"Selected: +$3, 64"`.
- `money_gain_preferred`: `gain_money` 2 gives `"Reorder jokers: +$2"`; `gain_money` 0 with `gain` 128 gives `"Reorder jokers: +128"`.
- `best_found_label_kept` when `order_exact` is false.
- `blocked_label`: `hand` reads `"Blocked: scores 0"`.
- `blind_warning_text`: `warning` is `"Won't beat the blind"`.
- `approximate_and_mismatch_shown_together`: both appear in `status`, joined by `" | "`. This resolves v1's deferred label issue.
- `naneinf_gain_is_safe`.
- `unavailable_status`: `"Engine unavailable"`, other lines empty.

`test_live.lua` additions:
- `assist_off_clears_and_stops`: result cleared, status `"idle"`, no further `search_start`, `score` or `search_poll` calls.
- `assist_on_restarts_search`.
- `mode_change_starts_new_search_and_clears_result`: while a search is running, change mode; the old result is gone and a poll of the old id isn't used.
- `search_json_contains_mode`.

`test_snapshot.lua`, replacing `snapshot_never_reads_deck_order`:
- `deck_contents_ignore_order`: the same deck in two orders gives identical `json.encode(s.deck)`.
- `deck_contents_group_and_count`: two identical cards give `count` 2.
- `deck_contents_have_no_ids_or_positions`: no group has `id`, `sort_id` or a position field.
- `deck_remaining_still_counts`.

Update `fake_game.lua` so deck cards are real cards rather than poisoned ones.

- [ ] **Step 2: Run to verify they fail**

Run: `for t in settings panel_text format live snapshot; do luajit tests/lua/test_$t.lua engine/target/release/; done`
Expected: each fails on missing modules or functions.

- [ ] **Step 3: Implement**

Move `ui.lua`'s `set_text` logic into `panel_text.build`, and have `ui.refresh` use it, adding lines for `money` and `warning`, and removing the `show_panel` toggle from the config tab. `config.lua` gains `assist = nil` (so a legacy `show_panel` can be detected) and `mode = "score"`. In `live.step`, when `settings.assist()` is false, clear state and return before any other work. Deck groups sort by rank, suit, enhancement, edition, then seal.

- [ ] **Step 4: Verify**

Run: `for t in tests/lua/test_*.lua; do case $t in *test_ffi*) luajit $t engine/target/release/libmortemqol_engine.dylib;; *) luajit $t engine/target/release/;; esac; done`
Expected: all pass, including `test_end_to_end`.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(mod): add settings, money panel text and deck counts"
```

---

### Task 5: Money in the live check

**Files:**
- Modify: `src/verify.lua`, `src/live.lua`, `engine/tests/replay.rs`
- Test: `tests/lua/test_verify.lua`

**Interfaces:**
- Consumes: prediction money fields; `settings.assist()`.
- Produces: `verify.compare_money(prediction, actual) -> "match"|"mismatch"|"approximate"`; `verify.tick()`, called every frame from `live.update` whatever the assistance setting, so a play already in flight still finishes its comparison. Replay lines with `actual_money` are checked for money; lines with `actual` for score.

- [ ] **Step 1: Write failing tests**

`money_exact_match`; `money_exact_mismatch`; `money_range_match`; `money_outside_range_mismatches`; `money_approximate_never_mismatches`.
`tick_waits_for_hand_played_to_end`: with `G.STATE == G.STATES.HAND_PLAYED`, `tick()` compares nothing; after the state changes, it compares `G.GAME.dollars` with the recorded balance once.
`money_mismatch_writes_line_with_actual_money` to `verify.jsonl` and increments `mismatch`.
`new_play_discards_uncompared_money`: `before_play` twice without a state change compares only the second.
`assist_off_records_nothing`: `before_play` does nothing when `settings.assist()` is false.
`check_catches_wrong_money` in `replay.rs`.

- [ ] **Step 2: Run to verify they fail**

Run: `luajit tests/lua/test_verify.lua; cd engine && cargo test --test replay`
Expected: FAIL.

- [ ] **Step 3: Implement**

Score comparison stays immediate in `after_score`. Keep the money comparison pending until `tick` sees a state other than `HAND_PLAYED`. Money tolerance is 1e-9 relative with a minimum slack of 1e-9. Add `HAND_PLAYED` to `fake_game.lua` states if missing.

- [ ] **Step 4: Verify, then commit**

Run: `luajit tests/lua/test_verify.lua && cd engine && cargo test --test replay`
Expected: PASS.

```bash
git add -A && git commit -m "feat(mod): compare predicted money with real money"
```

---

### Task 6: Gear icon and menu, verified in the real game

**Files:**
- Create: `src/gear.lua`, `assets/1x/gear.png` (34x34), `assets/2x/gear.png` (68x68)
- Modify: `MortemQOL.lua`, `src/ui.lua`, `.github/workflows/release.yml`, `tests/ingame/autoplay.lua`, `.superpowers/devinstall.sh`, `README.md`

**Interfaces:**
- Consumes: `settings`.
- Produces: `gear.init(mod)`, `gear.refresh()`, called from the update hook after `ui.refresh`, wrapped in `pcall`. `G.FUNCS.mqol_gear` toggles the menu. Autoplay reads files `MortemQOL/money` (Money mode) and `MortemQOL/scenarios` as now.

- [ ] **Step 1: Add the UI**

Register `SMODS.Atlas({ key = "gear", path = "gear.png", px = 34, py = 34 })`. The gear is a `UIBox` at `hard_set_T(0.25, 0.15, ...)`, shown when `G.STAGE == G.STAGES.RUN`, holding a `G.UIT.O` sprite, or a `"QOL"` text button if `G.ASSET_ATLAS["MortemQOL_gear"]` is nil. The menu sits beneath it with `create_toggle` for Assistance and `create_option_cycle` for Mode (`Score`, `Money`), wired to the setters. The config tab shows the same two controls. Add `assets` to the release workflow's `cp` line and the dev install.

- [ ] **Step 2: Extend autoplay**

Add scenarios: `money_tooth`, `money_ox`, `matador_blocked` (`bl_psychic`, Matador, 1 card), `gold_seal_retrigger`, `todo_list`. The script sets Money mode through `settings.set_mode` when `MortemQOL/money` exists, and adds `money` to its log lines through `verify.counts()`.

- [ ] **Step 3: Run forced scenarios in the real game**

Follow the Global Constraints' backup steps, then run `.superpowers/devinstall.sh autoplay` with `MortemQOL/scenarios` and `MortemQOL/money` present, and launch.
Expected: `autoplay.log` shows at least two passes through every scenario with mismatch 0. If a money comparison disagrees because of timing, fix where `tick` reads the balance, following the spec's Risks section, and rerun.

- [ ] **Step 4: Run random hands in both modes**

Remove `scenarios`. Run at least 100 hands with `money` present, then at least 100 without.
Expected: mismatch 0, `rng_changes` 0, and no `panel error` in the Lovely log. A mismatch becomes a line in `engine/tests/replays/`, a failing test, and a fix.

- [ ] **Step 5: Check the panel with screenshots**

Use the `hold` file and background screenshots.
Expected: the gear doesn't overlap the blind info, and money lines and the warning are readable in the panel.

- [ ] **Step 6: Update the README and commit**

Describe the gear menu, Money mode, what money counts, the blind warning, and that money mode can lose a run. Restore saves and mods as the Global Constraints require.

```bash
git add -A && git commit -m "feat(mod): add gear menu and verify money mode in-game"
```

---

### Task 7: Review, release candidate and release

**Files:**
- Modify: `manifest.json`, `MortemQOL.json`, plus fixes found

**Interfaces:**
- Consumes: everything above.

- [ ] **Step 1: Whole-branch review and fix pass**

Follow the execution skill's Final Review.
Expected: Critical and Important findings fixed test-first, and the suite green.

- [ ] **Step 2: Publish a release candidate**

Set `version_number` to `1.1.0` and `version` to `1.1.0-rc.1`, run all gates, and tag `v1.1.0-rc.1`.
Expected: the release workflow passes, and the zip contains `assets/1x/gear.png` and `assets/2x/gear.png` but not `autoplay`.

- [ ] **Step 3: User acceptance**

Ask the user to import the zip and check what the script can't: the gear opens and closes the menu; both controls work; Assistance off removes everything and on brings it back; the gear doesn't overlap the blind info at their window size; the chosen mode survives a restart; Money mode gives sensible recommendations in play.
Expected: the user confirms, with no new `verify.jsonl` lines.

- [ ] **Step 4: PR, merge and tag on approval**

Set `version` to `1.1.0`, open the PR, and on the user's approval merge it and tag `v1.1.0`.
Expected: a full release is published.
