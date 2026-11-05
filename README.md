# MortemQOL

A Balatro mod with a live calculator. While you choose cards, a small panel shows the highest-scoring play in your hand and whether reordering your jokers would beat it.

## What it shows

- **Best play:** the poker hand, its score, and what percentage of the blind it covers.
- **Range:** the lowest and highest possible scores when chance is involved, such as Lucky Cards or Misprint. Plays are ranked by expected score.
- **Your selection:** the score of the cards you have selected, for comparison.
- **Joker order:** when reordering jokers would score more, the gain appears on the panel and blue numbers on the jokers show the order.
- **Card order:** when position matters, such as with Photograph, purple numbers on your cards show the order to drag them into.

The **Select** button selects the recommended cards. The mod never plays a hand or moves a card or joker.

## Install

You need [r2modmac](https://github.com/Zard-Studios/r2modmac) (or r2modman) with Steamodded and Lovely in your profile.

1. Download `MortemQOL-<version>.zip` from [Releases](https://github.com/MrMortem/MortemQOL/releases).
2. In r2modmac, import the zip as a custom mod. Import the **zip itself**. On macOS, a folder you extracted by hand can be blocked by quarantine, and the panel will say "Engine unavailable".
3. Apply the profile and launch the game.

r2modmac may warn that the mod contains loadable files. That's expected: the calculator is a compiled library in `native/`, built from `engine/` in this repository by GitHub Actions.

Supported: macOS (Apple Silicon and Intel) and Windows x64. Linux players running the game through Proton use the Windows build. **The Windows build compiles and passes tests in CI but hasn't been confirmed in the real game yet**, so please open an issue if it doesn't load.

## Multiplayer

**Disable this mod in your r2modmac profile before playing ranked.** It does not turn itself off.

The calculator stays on in multiplayer lobbies, on the assumption that everyone in the lobby has agreed to it. The mod appears under its real name in the PvP mod's mod list, and nothing hides it.

Multiplayer rulesets can change scoring, and PvP adds its own jokers, which the calculator doesn't model. All results in a lobby are labeled "Approximate".

## What "Approximate" means

Hover over the label to see why. It appears when the result can't be exact:

- a joker, enhancement, edition or seal from another mod (treated as having no effect)
- a face-down card or joker (the calculator only uses what you can see, and never looks at deck order)
- The Hook, which discards random cards before scoring
- any multiplayer lobby

## Limits

- It covers the current hand only. It doesn't suggest discards or plan across a round.
- With more than 6 jokers, or very large hands, joker order is the best found rather than guaranteed best, and the panel says so.
- Scores use standard floating-point numbers, so Talisman-scale scores aren't supported.

## Reporting a wrong score

The mod compares every play with the score the game actually awards. If they disagree, the panel shows a mismatch count and the details are saved to `MortemQOL/verify.jsonl` in your Balatro save folder (`~/Library/Application Support/Balatro` on macOS, `%AppData%\Balatro` on Windows). Please attach that file to a [new issue](https://github.com/MrMortem/MortemQOL/issues).

## Development

- `engine/`: the Rust scoring engine and search. Run `cargo test` there.
- `src/`: the Lua mod. Tests are in `tests/lua/`; run them with `luajit` from the repository root.
- `tests/ingame/autoplay.lua`: a development script that plays randomized hands in the real game so the live check can compare results. It isn't included in releases.
- `docs/jokers.md`: coverage for all 150 vanilla jokers.

Pushing a `v*` tag builds and publishes a release.

## License

MIT
