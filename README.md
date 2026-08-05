# TypeSafe

A focused typing-practice application for your terminal. TypeSafe presents a
randomized word stream, tracks your speed and accuracy, and keeps the session
controls out of your way.

## Install

```sh
cargo install typesafe
```

## Run

```sh
typesafe
```

Use the arrow keys or `j`/`k` to move through the setup menu. Press Space to
toggle character options, Left/Right to change the duration, and Enter to
start. During a session, Backspace corrects a typo and Escape returns to the
setup screen. Select **View typing history** from setup to see average WPM by
week and a GitHub-style, year-long session activity heat map. Press `q` to
quit.

## Invite-code races

Invite-code races use a small TypeSafe relay. Run the relay on a host both
players can reach:

```sh
typesafe relay 0.0.0.0:9876
```

Then each player points TypeSafe at it and chooses **Race with an invite code**
from setup:

```sh
TYPESAFE_RELAY_ADDR=races.example.com:9876 typesafe
```

The host receives a six-character invite code to share. Once the other player
joins, both clients receive the exact same prompt and begin after a shared
three-second countdown. The relay only coordinates the room and forwards live
progress; typing history remains local.

The built-in relay speaks plain TCP. Run it on a trusted network, or place it
behind a TLS tunnel before exposing it to the public internet.

## Session history

Completed sessions, including sessions ended early with Escape after typing,
are stored locally in SQLite. The database is never sent anywhere. It lives in
your operating system's local application-data directory under
`typesafe/sessions.sqlite3` (for example,
`~/Library/Application Support/typesafe/sessions.sqlite3` on macOS).

## Requirements

TypeSafe requires Rust 1.88 or later and a terminal with Unicode support.

## License

This project is licensed under the [MIT License](LICENSE-MIT).
