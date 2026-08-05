# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```sh
cargo run                       # launch the TUI
cargo run -- relay 0.0.0.0:9876 # run the race relay server instead of the TUI
cargo test                      # all tests
cargo test relay_pairs_two_clients_and_forwards_progress   # a single test by name
cargo test race::               # a module's tests (binary-only crate — --lib has no target)
cargo clippy --all-targets
cargo fmt
```

`TYPESAFE_RELAY_ADDR` selects the relay a client connects to (default `127.0.0.1:9876`).

Edition 2024, MSRV 1.85 — let-chains (`if let Some(x) = a && cond`) are used throughout and require that toolchain.

## Architecture

Four modules under a single binary crate; `main.rs` owns the terminal lifecycle and the event loop.

- **`main.rs`** — Splits into two programs: with a `relay` first argument it binds a `TcpListener` and calls `race::run_relay` (no TUI at all); otherwise it enters raw mode / alternate screen and runs the render loop. **All key handling lives here**, as a single `match key.code` with `if app.screen == …` guards — adding a screen or a binding means editing this match, not `app.rs`. The loop polls events with a 50 ms timeout, so `app.poll_race()` and `app.update_clock()` get called at least ~20×/s; that tick is what drives the countdown, the auto-finish at duration, and the race start transition.

- **`app.rs`** — All state and rules in one `App` struct plus a `Screen` enum (`Menu`, `RaceSetup`, `RaceLobby`, `Typing`, `Finished`, `History`). No async, no message bus: methods mutate `App` directly and `poll_race` drains the race channel into state changes. Accuracy is a positional character-by-character zip of `typed` against `prompt` (backspacing rewrites history, there is no per-keystroke error log). WPM is `chars/5 / minutes`.

  Prompt generation differs by mode: solo play calls `extend_prompt` on every keystroke to keep ~360 chars ahead of the cursor, while a race pre-generates a fixed 6 000-char prompt (`prepare_race_prompt`) *before* connecting, because both players must type identical text. Consequently `type_character` skips extension when `race_client` is set — a racer who outruns 6 000 chars simply runs out of prompt.

- **`race.rs`** — Both halves of the multiplayer protocol. `RaceClient` spawns a reader and a writer thread around one `TcpStream` and exposes them as an mpsc `Receiver<RaceEvent>` / `Sender<String>` pair, so the UI thread never blocks on IO (`try_event` is non-blocking). `run_relay` is a thread-per-connection server holding `Arc<Mutex<HashMap<String, Room>>>`; a room holds the duration, the shared prompt, and host/guest writers.

  The wire protocol is newline-delimited ASCII over plain TCP, space-separated, parsed with `splitn`: client sends `CREATE <code> <secs> <hex-prompt>`, `JOIN <code>`, `PROGRESS <chars> <acc>`, `FINISH <chars> <wpm> <acc>`; relay sends `HOSTED`, `START <epoch_ms> <secs> <hex-prompt>`, `PEER_PROGRESS`, `PEER_FINISHED`, `PEER_DISCONNECTED`, `ERROR <kebab-code>`. Prompts are hex-encoded (`encode`/`decode`) precisely because the framing is space-delimited. Duration `0` on the wire means the infinite option. Start is synchronized by absolute wall-clock: the relay sends `now + 3 000 ms` and each client waits for its own `SystemTime::now()` to reach it — so clock skew between players shifts the start, and `Instant`-based elapsed time only begins at the local transition.

  The invite code is generated **client-side** (`invite_code`, ambiguity-free alphabet) and the relay rejects collisions with `ERROR code-unavailable`. Host disconnect removes the room; guest disconnect only clears the guest slot.

- **`storage.rs`** — SQLite (`rusqlite` with `bundled`, so no system libsqlite needed) at `<data_local_dir>/typesafe/sessions.sqlite3`. One table, `typing_sessions`, created idempotently on open. Sessions are recorded both on natural finish and on Escape-out (`return_to_menu`), including races. Storage failures never abort the app — they land in `App::storage_error`, which is rendered only on the History screen (`ui.rs`, inside `history`, overdrawing the heatmap row), so a failed save is invisible from the results screen.

- **`ui.rs`** — Pure rendering; takes `&App` and never mutates. `render` splits header / body / footer, then dispatches on `app.screen`. The history screen derives its weekly-average trend and year-long heat map from the raw session list at draw time (`average_wpm_trend`, `heatmap`, `week_start`) — there are no aggregate tables.

- **`words.txt`** — Embedded at compile time via `include_str!` into the `WORDS` `LazyLock`; edits require a rebuild. Words must be lowercase and plain — capitals, digits, and symbols are layered on at generation time by the `include_*` flags, and `generates_words_from_the_word_bank` asserts every generated word came from this file.

## Tests

Tests live in `#[cfg(test)] mod tests` at the bottom of each module. Note that `App::new()` in tests opens the **real** user database (`SessionStore::open_default`), so app tests touch `~/.local/share/typesafe/sessions.sqlite3`; only `SessionStore::in_memory()` (test-only) is isolated. Race tests start a real relay on `127.0.0.1:0` in a background thread and poll with a deadline helper (`wait_for_event`) rather than sleeping a fixed interval.
