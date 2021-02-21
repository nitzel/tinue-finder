## Tinue-Finder
This program searches a database of [Tak](playtak.com) games for [Roads to Tinuë](https://en.wikipedia.org/wiki/Tak_(game)#Terminology) and stores them back in the database.

### Why
This program is used to generate longer Tinuë puzzles for [puzzle.exegames.de](https://puzzle.exegames.de).

### How to use
Run `tinue-finder --help` for an up to date list of available paramters.

Example: `tinue-finder --db ./playtak.db --board-size 5` goes through all games with a board size of `5` that ended in a road win.
It then remvoes a few plies from the end (number configurable through `--undo x`) and searches for tinues with a maximum number configurable through `--max-depth y`.
Tinues with a length of `1` are currently omitted as they only require a single move.

### Remarks
- Setting `--max-depth` to values greater than `5` will take a lot of time per game.

### How to build
1. Install [Rust](https://www.rust-lang.org/tools/install)
    - Currently (as of `2021-02-21`) the `nightly` version of Rust is required to build this, but that will change once certain features have made it to the stable versions.
2. Build: `cargo build` / `cargo build --release`
    - Or build and run: `cargo run` or `cargo run --release`. Arguments can be supplied after a `--`, e.g. `cargo run -- -n 5 --db playtak.db`
    - Release builds run faster but take longer to build.

## Thanks to
- [Morten](https://github.com/MortenLohne) for getting me started and providing  help with Rust