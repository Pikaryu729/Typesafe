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
setup screen. Press `q` to quit.

## Requirements

TypeSafe requires Rust 1.85 or later and a terminal with Unicode support.

## License

This project is licensed under the [MIT License](LICENSE-MIT).
