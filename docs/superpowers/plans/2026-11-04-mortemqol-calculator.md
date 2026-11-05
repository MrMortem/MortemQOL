# MortemQOL Live Calculator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Balatro mod, installed through r2modmac from a public GitHub release, that live-computes the highest-scoring play and best joker order.

**Architecture:** A thin Lua layer inside Steamodded snapshots visible game state to JSON and hands it over FFI to a Rust library. The library searches on a worker thread with its own scoring model, and Lua polls for results and draws a panel. Every real play is compared with its prediction.

**Tech Stack:** Rust stable (`cdylib`, `serde`, `serde_json`), LuaJIT FFI, Steamodded 26.1002.0, Lovely, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-11-04-mortemqol-calculator-design.md`

## Global Constraints

- Repo `MrMortem/MortemQOL`, public, MIT. Work on branch `feat/live-calculator`; merge through a PR at the end.
- Commits use the repo-local noreply identity already configured. Never change it or use another email. Each commit message ends with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`; the PR body ends with `🤖 Generated with [Claude Code](https://claude.com/claude-code)`.
- Advice only: mod code may highlight cards on the Select button. It never plays a hand, moves a joker, or writes other game state.
- Never read deck order. Face-down hand cards and jokers are masked as unknown. Never call the game's RNG or scoring code from the calculator.
- Calculator stays active in multiplayer lobbies. Nothing hides the mod from PvP's mod hash or works around `PVP.BANNED_MODS`.
- Scores are `f64`. Non-finite scores cross the JSON boundary as the string `"naneinf"`.
- Performance target: full search of 8 cards and 5 jokers within 100 ms on Apple Silicon, release build.
- No panic crosses the FFI boundary. `panic = "unwind"` stays on in the release profile.
- Dependencies are limited to `serde` and `serde_json`. Commit `Cargo.lock`.
- Scoring truth is the game's source, patched with Steamodded: `~/Library/Application Support/Balatro/Mods/lovely/dump/` (`card.lua` `Card:calculate_joker`, `functions/state_events.lua` `G.FUNCS.evaluate_play`, `functions/common_events.lua`) and Steamodded's `src/utils.lua`. Read these rather than relying on memory. Never copy game source into the repo.

## Review Focus

- A hand with fewer than 5 cards, an empty hand, or no legal play: the panel shows no recommendation and nothing crashes. Test in Task 7.
- Very large hands (15+ cards) or many jokers (8+): the game keeps running, and a newer search cancels the old one promptly. Test in Task 7.
- The state changes between a result and pressing Select (a card was played or discarded): Select does nothing, and a stale result is never shown. Tests in Tasks 7 and 8.
- Content from other mods: unknown enhancement, edition, seal or joker keys, missing ability fields, or non-numeric ability values give an approximate result, not an error. Tests in Tasks 3 and 8.
- Huge scores: infinity or NaN round-trips through JSON as `"naneinf"` and displays that way. Tests in Tasks 4 and 9.

---

## Shared Contracts

Tasks use these exact formats. Fields are optional only where marked.

**C ABI** (`engine/src/ffi.rs`):

```c
uint32_t mqol_abi_version(void);                     // returns 1
uint64_t mqol_search_start(const uint8_t *json, size_t len);  // returns id >= 1, or 0 on failure
// Poll and score statuses: 0 running, 1 finished, -1 error (out holds message),
// -2 buffer too small (out_len holds the needed size), -3 unknown or superseded id.
int32_t  mqol_search_poll(uint64_t id, uint8_t *out, size_t cap, size_t *out_len);
int32_t  mqol_score(const uint8_t *json, size_t len, uint8_t *out, size_t cap, size_t *out_len);
```

While running, poll sets `*out_len` to 0 if there is no result newer than the last one returned for that id.

**Snapshot JSON:**

```json
{"schema":1,
 "hand":[{"id":12,"rank":14,"suit":"Hearts","enh":"m_glass","ed":"e_holo","seal":"Red",
          "perma":{"bonus":0,"mult":0,"x_mult":1},"debuff":false,"forced":false,"hidden":false}],
 "jokers":[{"id":3,"key":"j_green_joker","ed":null,"debuff":false,"hidden":false,"sell":2,"ab":{"mult":4}}],
 "levels":{"Pair":{"level":1,"chips":10,"mult":2,"l_chips":15,"l_mult":1,"played":3,"played_round":0,"visible":true}},
 "blind":{"key":"bl_flint","disabled":false,"target":600},
 "round":{"hands_left":4,"discards_left":3,"dollars":7,"chips":0},
 "deck":{"back":"b_plasma","remaining":44,"total":52},
 "vouchers":["v_observatory"],"consumables":[{"key":"c_pluto"}],
 "lobby":false}
```

`id` is the card's `sort_id`. `rank` is 2 to 14. `ab` is the joker's `ability` table with functions and cycles removed; tallies the game maintains there (such as `steel_tally`) are how the engine reads deck-wide counts. Unknown fields are ignored; unknown keys are kept as strings.

**Score request:** `{"snapshot":<snapshot>,"play":[<card ids, left to right>]}`

**Score result:** `{"hand":"Pair","scored":[ids],"floor":64,"expected":64,"ceiling":64,"legal":true,"has_chance":false,"approximate":[]}`

**Search result:**

```json
{"stage":3,"done":true,
 "best":{"play":[ids],"hand":"Flush","scored":[ids],"floor":1,"expected":1,"ceiling":1,"blind_pct":12.5},
 "joker_order":[ids],"current_order_expected":1,"gain":0,"order_exact":true,"card_order_matters":false,
 "approximate":["unknown_joker:j_x","hidden_card","hidden_joker","lobby"]}
```

`best` is `null` when there is no legal play. `approximate` is empty when the result is exact.

---

### Task 1: Repo, crate skeleton and FFI proof

**Files:**
- Create: `LICENSE`, `.gitignore`, `engine/Cargo.toml`, `engine/Cargo.lock`, `engine/src/lib.rs`, `engine/src/ffi.rs`, `tests/lua/test_ffi.lua`, `.github/workflows/ci.yml`
- Test: `engine/src/ffi.rs` unit tests, `tests/lua/test_ffi.lua`

**Interfaces:**
- Produces: the C ABI above, with stubs. `mqol_score` and `mqol_search_*` return error status -1 with message `not implemented`, until Tasks 4 and 7. Build output: `engine/target/release/libmortemqol_engine.dylib`.

- [ ] **Step 1: Install Rust and create the branch**

Run: `brew install rustup && rustup-init -y --no-modify-path && ~/.cargo/bin/rustup target add x86_64-apple-darwin`, then `git switch -c feat/live-calculator` if not already on it.
Expected: `~/.cargo/bin/cargo --version` prints a version. If Homebrew's `rustup` already provides `cargo` differently, use that path consistently.

- [ ] **Step 2: Write failing tests**

Rust: `abi_version_is_1`; `score_with_null_pointer_returns_error_not_crash`; `poll_unknown_id_returns_minus_3`.
`tests/lua/test_ffi.lua`: `ffi.load` the built library by the path in `arg[1]`, assert `mqol_abi_version() == 1`.

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd engine && cargo test`
Expected: FAIL to compile, functions not defined.

- [ ] **Step 4: Implement the crate**

`crate-type = ["cdylib", "rlib"]`, name `mortemqol_engine`. Wrap each entry point in a shared `guarded` helper using `catch_unwind`.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd engine && cargo test && cargo build --release && luajit ../tests/lua/test_ffi.lua target/release/libmortemqol_engine.dylib`
Expected: all pass.

- [ ] **Step 6: Add CI**

`ci.yml` runs on push on `macos-latest` and `windows-latest`: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`. On macOS only, install LuaJIT and run `test_ffi.lua`.

- [ ] **Step 7: Commit, create the public repo, and push**

```bash
git add -A && git commit -m "feat: add engine crate skeleton and FFI"
gh repo create MrMortem/MortemQOL --public --source . --push
git push -u origin main feat/live-calculator
```
Expected: `gh run list` shows CI green on the branch.

---

### Task 2: Release pipeline and in-game load proof

This task settles the spec's largest open risk before the engine is built. Stop and report to the user if macOS blocks the library.

**Files:**
- Create: `manifest.json`, `MortemQOL.json`, `MortemQOL.lua`, `src/bridge.lua`, `icon.png` (256x256), `.github/workflows/release.yml`, `tests/lua/test_bridge.lua`

**Interfaces:**
- Consumes: C ABI.
- Produces: `bridge.load(mod_path) -> bool, err`; `bridge.available() -> bool`; `bridge.search_start(json) -> id`; `bridge.search_poll(id) -> status, json_or_nil`; `bridge.score(json) -> status, json_or_nil`. Poll and score retry once with a larger buffer on status -2. Library paths: `native/macos/libmortemqol_engine.dylib`, `native/windows/mortemqol_engine.dll`.

- [ ] **Step 1: Write failing test `tests/lua/test_bridge.lua`**

Assertions: `bridge.load` with a missing path returns `false` plus a message and does not raise; `bridge.available()` is then `false`; loading the built library returns `true`; a stubbed wrong ABI version returns `false`.

- [ ] **Step 2: Run to verify it fails, implement `src/bridge.lua`, run to verify it passes**

Run: `luajit tests/lua/test_bridge.lua engine/target/release/`
Expected: FAIL first with module not found, then PASS.

- [ ] **Step 3: Write the mod metadata and entry point**

`manifest.json`: name `MortemQOL`, `version_number` `0.0.1`, `website_url` `https://github.com/MrMortem/MortemQOL`, description under 250 characters, `dependencies` `[]`. `MortemQOL.json`: id and prefix `MortemQOL`, `main_file` `MortemQOL.lua`, `dependencies: ["Steamodded (>=1.0.0~BETA-1814a)"]`. The entry point loads modules using `SMODS.load_file`, calls `bridge.load(SMODS.current_mod.path)`, and logs success or failure with `sendInfoMessage`.

- [ ] **Step 4: Write `release.yml`**

On tag `v*`: build `aarch64-apple-darwin` and `x86_64-apple-darwin`, `lipo` them into one universal library, `codesign -s -` it. Build Windows on `windows-latest`. Check that the tag matches `manifest.json`'s version. Package with `manifest.json` at the zip root and publish `MortemQOL-<version>.zip` using `gh release create`, marked prerelease if the tag contains `-` or starts with `v0.0`.

- [ ] **Step 5: Tag and download the real artifact**

Run: `git tag v0.0.1 && git push origin feat/live-calculator v0.0.1`. Download the zip **with a browser**, not `gh`, so macOS applies quarantine.
Expected: `xattr` shows `com.apple.quarantine` on the zip, and the zip has `manifest.json` at its root.

- [ ] **Step 6: Import into r2modmac and launch**

Import the zip as a custom mod in a test profile with Steamodded and Lovely, apply, launch.
Expected: the Lovely log (`~/Library/Application Support/Balatro/Mods/lovely/log/`) shows the bridge success line, and the game runs. If it fails, record the error, test `xattr -d com.apple.quarantine`, and bring the result to the user before continuing.

- [ ] **Step 7: Commit**

```bash
git add -A && git commit -m "feat: load the native engine in-game and add release pipeline"
```

---

### Task 3: Snapshot model and hand detection

**Files:**
- Create: `engine/src/snapshot.rs`, `engine/src/hand.rs`, `engine/tests/common/mod.rs`, `engine/tests/hand_detection.rs`, `engine/tests/snapshot_parse.rs`

**Interfaces:**
- Produces: `snapshot::parse(&[u8]) -> Result<Snapshot, String>`, with `Snapshot`, `Card`, `Joker`, `Level`, `Blind`, `Round`, `Deck` mirroring the Shared Contracts; `hand::Rules { four_fingers, shortcut, smeared, pareidolia, splash }`; `hand::detect(played: &[&Card], rules: &Rules) -> Detected { name: &'static str, scoring: Vec<usize> }`, using names exactly as in the game (`"Pair"`, `"Flush Five"`, and so on). Test helpers `common::card("AS")`, `common::snap(...)`.

- [ ] **Step 1: Write failing tests**

`every_hand_type_detected` (all 12 types, including Royal Flush reported as the game reports it); `kicker_excluded_from_scoring`: AS AH KD → `Pair`, scoring `[0,1]`; `four_fingers_flush`; `shortcut_straight` (2-4-6-8-10); `smeared_flush_mixes_red` ; `wild_card_counts_for_any_suit`; `stone_card_has_no_rank_and_always_scores`; `splash_scores_all_played`; `pareidolia_makes_all_cards_face`.
Snapshot: `unknown_keys_preserved_as_strings`; `missing_optional_fields_default` (no `ab`, no `perma`); `non_numeric_ability_value_does_not_fail_parse`; `malformed_json_returns_err`.

- [ ] **Step 2: Run to verify they fail**

Run: `cd engine && cargo test --test hand_detection --test snapshot_parse`
Expected: FAIL to compile.

- [ ] **Step 3: Implement `snapshot.rs` and `hand.rs`**

Port logic from `get_flush`, `get_straight`, `get_X_same` and `evaluate_poker_hand` in the dumped `functions/misc_functions.lua`. `ab` stays `serde_json::Value`, accessed with defaulting helpers.

- [ ] **Step 4: Run to verify they pass, then commit**

Run: `cd engine && cargo test`
Expected: PASS.

```bash
git add -A && git commit -m "feat(engine): parse snapshots and detect poker hands"
```

---

### Task 4: Scoring core without jokers

**Files:**
- Create: `engine/src/score.rs`, `engine/src/blinds.rs`, `engine/tests/score_base.rs`
- Modify: `engine/src/ffi.rs` (implement `mqol_score`), `engine/src/lib.rs`

**Interfaces:**
- Consumes: `snapshot`, `hand`.
- Produces: `score::score_play(s: &Snapshot, play: &[usize], joker_order: &[usize]) -> Scored { hand, scored, floor, expected, ceiling, legal, has_chance, approximate }`; `score::to_json_number(f64) -> serde_json::Value`, which returns `"naneinf"` for non-finite values. Jokers are hooked through `jokers::Phase` calls with an empty dispatcher, filled in by Tasks 5 and 6. Three evaluations share one code path through `enum Mode { Floor, Expected, Ceiling }`.

- [ ] **Step 1: Write failing tests with these exact values**

| Test | Setup | Expected |
|---|---|---|
| `pair_of_aces_level_1` | AS AH | 64 |
| `high_card_lucky_range` | A lucky | floor 16, expected 80, ceiling 336 |
| `glass_card_x2` | A glass | 32 |
| `red_seal_retriggers` | A red seal | 27 |
| `holo_adds_10_mult` | A holo | 176 |
| `flint_halves_base` | AS AH on `bl_flint` | 27 |
| `flint_disabled_is_normal` | as above, disabled | 64 |
| `plasma_balances` | AS AH on `b_plasma` | 289 |
| `psychic_illegal_under_5` | A on `bl_psychic` | `legal:false`, all scores 0 |
| `eye_rejects_repeat_hand` | Pair with `played_round:1` on `bl_eye` | `legal:false` |
| `mouth_rejects_other_hand` | Mouth with another type played this round | `legal:false` |
| `arm_lowers_level_first` | Pair level 2 on `bl_arm` | scores as level 1: 64 |
| `debuffed_card_adds_nothing` | A debuffed, high card | chips 5: 5 |
| `steel_held_x1_5` | A played, steel K held | 16 * 1.5 = 24 |
| `blue_seal_gold_no_score_effect` | A blue seal | 16 |
| `infinite_score_serializes_naneinf` | mult forced to `f64::MAX` | JSON field equals `"naneinf"` |
| `hidden_held_card_flags_approximate` | hidden card in hand | `approximate` contains `hidden_card` |
| `lobby_flag_approximate` | `lobby:true` | `approximate` contains `lobby` |
| `unknown_enhancement_flags_approximate` | `enh:"m_modded"` | `approximate` contains `unknown_enhancement:m_modded`; scores as no enhancement |

- [ ] **Step 2: Run to verify they fail**

Run: `cd engine && cargo test --test score_base`
Expected: FAIL to compile.

- [ ] **Step 3: Implement in the spec's scoring order**

Follow the "Scoring model" section. Include observatory: if `v_observatory` is present, each matching planet in `consumables` applies X1.5 mult. Read `blinds.rs` behaviors from dumped `blind.lua`.

- [ ] **Step 4: Run to verify they pass, then commit**

Run: `cd engine && cargo test`
Expected: PASS.

```bash
git add -A && git commit -m "feat(engine): score plays with enhancements, seals, editions and blinds"
```

---

### Task 5: Jokers with direct effects

**Files:**
- Create: `engine/src/jokers.rs`, `engine/tests/jokers_direct.rs`, `docs/jokers.md`, `engine/tests/coverage.rs`
- Modify: `engine/src/score.rs`

**Interfaces:**
- Consumes: `score::Mode` and the phase hooks.
- Produces: `jokers::apply(phase: Phase, joker: &Joker, ctx: &mut ScoreCtx)`, with `Phase { Before, PerCard, Held, Main, Edition, After }`; `hand::Rules::from_jokers`. `docs/jokers.md` is one row per vanilla joker: `key | status (modeled, no-score-effect, pending) | test`.

- [ ] **Step 1: Generate `docs/jokers.md`**

Extract all `j_*` keys from `P_CENTERS` in the game's `game.lua` with a throwaway script.
Expected: exactly 150 rows, each initially `pending`.

- [ ] **Step 2: Write failing coverage and joker tests**

`coverage.rs`: `table_has_150_rows`; `every_modeled_joker_has_a_named_test` (each `modeled` key appears in `engine/tests/jokers_*.rs`).

`jokers_direct.rs`: at least one test per joker whose effect depends only on the snapshot, named `j_<key>_<behavior>`. Pinned cases:
- `j_joker_then_cavendish_order`: high card A, `[j_joker, j_cavendish]` = 240; reversed = 112.
- `j_misprint_range`: high card A: floor 16, expected 200, ceiling 384.
- `j_bloodstone_range`: A of hearts: floor 16, expected 20 (X1.25), ceiling 24.
- `j_oops_doubles_lucky`: with Oops, Lucky's expected adds +8: 16 * 9 = 144.
- `debuffed_joker_has_no_effect`; `polychrome_joker_x1_5_after_its_effect`.
- `unknown_joker_no_effect_flagged`: `approximate` contains `unknown_joker:j_modded`, score unchanged.
- `hidden_joker_flagged`: `approximate` contains `hidden_joker`.

- [ ] **Step 3: Run to verify they fail**

Run: `cd engine && cargo test --test jokers_direct --test coverage`
Expected: FAIL.

- [ ] **Step 4: Implement direct jokers**

Port each from dumped `Card:calculate_joker`, reading values from `ab` rather than hardcoding them. Mark `modeled` or `no-score-effect` in the table. Scaling, retrigger, copy and money jokers stay `pending` for Task 6.

- [ ] **Step 5: Run to verify they pass, then commit**

Run: `cd engine && cargo test`
Expected: PASS.

```bash
git add -A && git commit -m "feat(engine): model jokers with direct scoring effects"
```

---

### Task 6: Scaling, retrigger, copy and money jokers

**Files:**
- Create: `engine/tests/jokers_stateful.rs`, `engine/tests/score_properties.rs`
- Modify: `engine/src/jokers.rs`, `engine/src/score.rs`, `docs/jokers.md`, `engine/tests/coverage.rs`

**Interfaces:**
- Consumes: `jokers::apply`.
- Produces: `coverage.rs` adds `no_joker_is_pending`. Task 7 relies on `score_play` being a pure function of its arguments.

- [ ] **Step 1: Write failing tests, one or more per remaining joker**

Pinned cases:
- `j_blueprint_copies_right`: high card A, `[j_blueprint, j_joker]` = 144; `[j_joker, j_blueprint]` = 80.
- `j_brainstorm_copies_leftmost`; `blueprint_chain_ending_in_itself_has_no_effect`.
- `j_space_weighted_levelup`: AS AH: expected 83.25, floor 64, ceiling 141.
- `j_hanging_chad_first_card`: high card A: 38.
- `j_photograph_card_order`: mult K then plain K with Photograph: 360; plain then mult: 240.
- `j_green_joker_counts_this_hand` and the rest of the pre-scoring scalers (Ride the Bus, Square, Runner, Spare Trousers, Obelisk) read their `before` change.
- `j_bull_reads_money_gained_this_hand`; `j_bootstraps_same`.
- `j_mime_retriggers_held_steel`; `j_dusk_on_last_hand`; `j_sock_and_buskin_faces`; `j_hack_low_ranks`; `j_seltzer`.

`score_properties.rs`, over a fixed table of at least 50 generated snapshots: `floor_le_expected_le_ceiling`; `deterministic_setups_have_equal_range`; `score_play_is_repeatable`; `additive_only_joker_orders_score_equally`.

- [ ] **Step 2: Run to verify they fail**

Run: `cd engine && cargo test --test jokers_stateful --test score_properties`
Expected: FAIL.

- [ ] **Step 3: Implement the remaining jokers**

Space Joker evaluates both level outcomes and weights them by its probability, including Oops. Cards changed during scoring by Midas Mask or Vampire use the source's order of effects. `ScoreCtx` is the only mutable state; `Snapshot` stays immutable.

- [ ] **Step 4: Run to verify they pass, then commit**

Run: `cd engine && cargo test`
Expected: PASS, including `no_joker_is_pending`.

```bash
git add -A && git commit -m "feat(engine): model scaling, retrigger, copy and money jokers"
```

---

### Task 7: Search, worker and benchmark

**Files:**
- Create: `engine/src/search.rs`, `engine/src/worker.rs`, `engine/tests/search.rs`, `engine/examples/bench.rs`
- Modify: `engine/src/ffi.rs`

**Interfaces:**
- Consumes: `score_play`.
- Produces: `search::run(s: &Snapshot, cancel: &dyn Fn() -> bool, publish: &mut dyn FnMut(SearchResult))`, publishing after each spec stage; `worker::start(json) -> u64`; `worker::poll(id) -> PollState`. One long-lived thread; a new start supersedes the current id.

- [ ] **Step 1: Write failing tests**

- `picks_pair_over_high_card`.
- `finds_joker_reorder_gain`: `[j_joker, j_cavendish]` reversed input gives `gain` 128 on a high card A-only hand.
- `finds_card_order`: two kings with Photograph gives the 360 order, `card_order_matters:true`.
- `never_below_any_direct_score`: for the Task 6 snapshot table, the best is at least every `score_play` with any subset.
- `forced_card_always_in_play` (Cerulean Bell).
- `hidden_cards_never_chosen`.
- `empty_hand_gives_null_best`; `one_card_hand_works`; `all_plays_illegal_gives_null_best`.
- `over_6_jokers_sets_order_exact_false`.
- `equal_expected_prefers_higher_floor`.
- `new_search_supersedes_old`: after a second start, polling the first returns -3.
- `large_search_cancels_quickly`: 15 cards and 8 jokers, a new start publishes within 200 ms.
- `results_report_stage_progression`: stages arrive in increasing order, with `done:true` last.

- [ ] **Step 2: Run to verify they fail**

Run: `cd engine && cargo test --test search`
Expected: FAIL.

- [ ] **Step 3: Implement search per the spec's three stages**

Check `cancel` at least once per candidate play. Skip joker permutations that differ only among jokers whose effect is purely additive to the same quantity and order-insensitive. Stage 3 runs only if a position-sensitive effect is present.

- [ ] **Step 4: Run to verify they pass**

Run: `cd engine && cargo test --test search`
Expected: PASS.

- [ ] **Step 5: Measure the performance target**

Run: `cd engine && cargo run --release --example bench`
Expected: prints time for 8 cards and 5 order-sensitive jokers; under 100 ms. If it misses after profiling obvious waste, narrow stage 2 to the top 32 plays from stage 1, as the spec allows, and add a test that `never_below_any_direct_score` still holds for stage 1 results.

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat(engine): search best play and joker order on a worker thread"
```

---

### Task 8: Lua snapshot and live loop

**Files:**
- Create: `src/json.lua`, `src/snapshot.lua`, `src/live.lua`, `tests/lua/test_snapshot.lua`, `tests/lua/fake_game.lua`
- Modify: `MortemQOL.lua`, `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: `bridge`.
- Produces: `json.encode(table) -> string`, `json.decode(string) -> table`; `snapshot.build() -> table`; `snapshot.selected_ids() -> {ids}`; `live.update(dt)`, called once per frame; `live.state() -> { status = "idle"|"calculating"|"ready"|"unavailable", result = table|nil, selection = table|nil }`; `live.select_best() -> bool`.

- [ ] **Step 1: Write failing tests using `fake_game.lua`, a minimal fake `G`**

- `snapshot_never_reads_deck_order`: fake `G.deck.cards` is a table whose metatable errors on any index; `build()` succeeds, using `#`-free counts from `G.GAME.starting_deck_size` and a pre-supplied remaining count source.
- `face_down_card_masked`: a card with `facing='back'` yields `hidden:true` with no rank, suit, enhancement, edition or seal.
- `face_down_joker_masked`: yields `hidden:true`, no `key`.
- `build_does_not_mutate_state`: a deep copy of fake `G` compares equal after `build()`.
- `functions_and_cycles_removed_from_ab`.
- `unknown_mod_fields_pass_through_as_strings`.
- `json_round_trip_with_special_characters`.
- `json_encodes_non_finite_as_naneinf`.
- `select_best_noop_when_card_gone`: with a result whose ids are no longer in hand, `select_best()` returns `false` and highlights nothing.
- `stale_poll_ignored`: a result for a superseded id does not change `live.state().result`.
- `no_new_search_when_json_unchanged`.
- `error_in_build_sets_unavailable_not_raise`.

To make the deck-order test exact: remaining count comes from `G.deck.config.card_count`, not the card list. Confirm that field in the dumped source, and if it doesn't exist, find another count that doesn't iterate cards.

- [ ] **Step 2: Run to verify they fail**

Run: `luajit tests/lua/test_snapshot.lua engine/target/release/`
Expected: FAIL.

- [ ] **Step 3: Implement the modules**

Throttle rebuilds to every 0.1 s, only while `G.STATE == G.STATES.SELECTING_HAND`. Wrap each `live.update` body in `pcall`. Set `lobby` to true when `PVP` exists and `PVP.LOBBY.code` is non-nil, read-only. `select_best` uses the game's normal `G.hand:add_to_highlighted` after clearing with `G.hand:unhighlight_all()`, and respects the game's selection limit.

- [ ] **Step 4: Run to verify they pass, wire into the entry point, and add to CI**

Hook by wrapping `love.update` from `MortemQOL.lua`, chaining the original. Add the Lua tests to CI's macOS job.
Expected: tests PASS.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(mod): snapshot visible state and run the live search loop"
```

---

### Task 9: In-game panel

**Files:**
- Create: `src/format.lua`, `src/ui.lua`, `tests/lua/test_format.lua`
- Modify: `MortemQOL.lua`

**Interfaces:**
- Consumes: `live.state()`, `live.select_best()`.
- Produces: `format.score(x) -> string`; `format.range(floor, ceiling) -> string|nil`; `format.reason(tag) -> string`; `ui.init()`; `ui.refresh()`.

- [ ] **Step 1: Write failing tests**

`format_large_numbers_match_game_style` (uses the game's `number_format` rules: `1,234`, `1.23e+15`); `format_naneinf_string`; `range_nil_when_floor_equals_ceiling`; `reason_for_each_approximate_tag` (`unknown_joker:*`, `hidden_card`, `hidden_joker`, `lobby`, `unknown_enhancement:*`, and an unknown tag falling back to generic text).

- [ ] **Step 2: Run to verify they fail, implement `format.lua`, run to verify they pass**

Run: `luajit tests/lua/test_format.lua`
Expected: FAIL, then PASS.

- [ ] **Step 3: Implement `ui.lua`**

Build the panel with `UIBox` nodes, movable, per the spec's "In-game UI" section: best hand, score and range, percent of blind, current selection's score, joker-order gain, numbered joker and card badges, Select button, and calculating, approximate and unavailable states with the reason on hover. Add the on/off toggle in the `SMODS.current_mod.config_tab`, stored via `config.lua`. Follow JokerDisplay's `src/ui.lua` and `src/config_tab.lua` as reference patterns only.

- [ ] **Step 4: Check it in the real game**

Copy the mod and a local release build into a test r2modmac profile. Launch and start a run.
Expected: panel appears only during hand selection; values change when jokers are dragged; Select highlights the right cards; no hand is played; the toggle hides it; no error in the Lovely log; panel doesn't overlap the hand at 1280x720 and larger. Take a screenshot.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(mod): add calculator panel, badges and select button"
```

---

### Task 10: Live check and replay

**Files:**
- Create: `src/verify.lua`, `tests/lua/test_verify.lua`, `engine/tests/replay.rs`, `engine/tests/replays/.gitkeep`
- Modify: `MortemQOL.lua`, `src/ui.lua`

**Interfaces:**
- Consumes: `snapshot.build`, `bridge.score`, `json`.
- Produces: `verify.before_play()`; `verify.after_score(actual: number)`; `verify.compare(prediction, actual) -> "match"|"mismatch"|"approximate"`; log `MortemQOL/verify.jsonl` through `love.filesystem`; session counters `verify.counts() -> {match, mismatch, approximate}`. Replay format, one JSON object per line: `{"snapshot":...,"play":[...],"prediction":...,"actual":n}`.

- [ ] **Step 1: Write failing tests**

Lua: `exact_match_within_1e_9`; `exact_mismatch_outside_1e_9`; `chance_inside_range_matches`; `chance_outside_range_mismatches`; `approximate_never_counts_as_mismatch`; `illegal_prediction_and_zero_actual_match`; `naneinf_prediction_with_infinite_actual_matches`; `mismatch_writes_replayable_line`; `verify_error_does_not_raise`.
Rust `replay.rs`: `all_replays_match` reads every `tests/replays/*.jsonl` line and asserts the same comparison rule, passing with an empty directory.

- [ ] **Step 2: Run to verify they fail**

Run: `luajit tests/lua/test_verify.lua && cd engine && cargo test --test replay`
Expected: FAIL.

- [ ] **Step 3: Implement and hook**

Wrap `G.FUNCS.play_cards_from_highlighted`, calling `verify.before_play()` first and the original unchanged. Wrap `G.FUNCS.evaluate_play`; after the original returns, call `verify.after_score(SMODS.last_hand_score)` (`functions/state_events.lua` in the dump sets it). Confirm in the dump that this value is set synchronously, and if it isn't, read it from an event queued afterward. All wrappers `pcall` mod code and always call the original exactly once. Show the session mismatch count on the panel only if it is above zero.

- [ ] **Step 4: Run to verify they pass, then check in-game**

Expected: tests PASS. Playing 3 hands in-game writes no mismatch for simple hands, and `verify.counts().match` is 3 (log it).

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat(mod): compare predictions with real scores and log mismatches"
```

---

### Task 11: Docs, release and acceptance

**Files:**
- Create: `README.md`, `tests/replays/*` as needed
- Modify: `manifest.json`, `MortemQOL.json` (version `1.0.0`), `engine/src/*` for fixes found

**Interfaces:**
- Consumes: everything above.

- [ ] **Step 1: Write the README**

Cover what it does; install via release zip and r2modmac custom import, including the expected loadable-file warning and any macOS step found in Task 2; **disable the mod in the r2modmac profile for ranked play**; the multiplayer label and PvP limits; what "approximate" means; v1 scope limits; how to report a mismatch by sharing `verify.jsonl`; a note that Windows hasn't been tested in the real game.

- [ ] **Step 2: Run the automated gates**

Run: `cd engine && cargo fmt --check && cargo clippy -- -D warnings && cargo test && cargo run --release --example bench`, then all `tests/lua/*.lua`.
Expected: all pass; bench under 100 ms.

- [ ] **Step 3: RNG and frame checks in-game**

With a temporary debug log, compare `G.GAME.pseudorandom` serialized before and after a full search.
Expected: identical. Dragging jokers rapidly causes no visible frame drop. Remove the debug log.

- [ ] **Step 4: Publish a release candidate and install it the real way**

Tag `v1.0.0-rc.1` (update manifest version to match the release check), download in a browser, import into r2modmac.
Expected: same as Task 2, with the panel working.

- [ ] **Step 5: Vanilla acceptance run with the user**

Ask the user to play one complete vanilla run on the rc build, with a variety of jokers, then read `verify.jsonl`.
Expected: zero mismatches. For each mismatch, copy the line into `engine/tests/replays/`, watch `replay.rs` fail, fix, and repeat with a new rc. Do not call v1 done until a complete run has zero mismatches.

- [ ] **Step 6: PvP lobby smoke test with the user**

Open a private PvP lobby on the rc build and play a few hands.
Expected: results labeled approximate; nothing breaks; any differences are logged as approximate.

- [ ] **Step 7: Merge and release**

Open a PR from `feat/live-calculator` to `main` and, on the user's approval, merge. Tag `v1.0.0` on `main`.
Expected: release workflow publishes `MortemQOL-1.0.0.zip`; CI is green.
