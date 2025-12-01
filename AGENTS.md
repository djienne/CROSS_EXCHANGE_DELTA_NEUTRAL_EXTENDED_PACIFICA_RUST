# Repository Guidelines

## Project Structure & Module Organization
- `src/main.rs` is the CLI entry; `lib.rs` re-exports library entry points. Core logic: `bot.rs`, `opportunity.rs`, `trading.rs`, clients in `rest.rs`/`websocket.rs`/`pacifica/*`, and signing in `signature.rs`/`snip12/*`. Emergency tooling: `src/bin/emergency_exit.rs`.
- Tests live beside modules plus the hash-order regression in `examples/test_field_orderings.rs` (enabled via `[[test]]`). Examples in `examples/` cover the bot, scanners, and REST/WebSocket samples.
- Config: `.env` and `config.json`; runtime state in `bot_state.json` (gitignored). Python signing helpers reside in `scripts/` and `python_sdk-starknet/`.
- Docker workflows use `Dockerfile` and `docker-compose.yml`; additional references in `DOC/`, `DOCKER.md`, and `EMERGENCY_EXIT.txt`.

## Build, Test, and Development Commands
- Install Python signing deps: `pip install -r requirements.txt` and `(cd python_sdk-starknet && pip install -e .)`.
- Build/run: `cargo build --release`; `cargo run --release` or `cargo run --example funding_bot`.
- Utilities: `cargo run --example scan_opportunities` for dry scans; `cargo run --bin emergency_exit` to close positions.
- Tests/format: `cargo test`; `cargo test --test test_field_orderings -- --nocapture`; `cargo fmt`; `cargo clippy --all-targets -- -D warnings` if available.

## Coding Style & Naming Conventions
- Rust 2021 with standard `rustfmt` formatting (4-space indents). Favor async-first code with `tokio`, structured logging via `tracing`, and typed errors in `error.rs` (`thiserror`) with `anyhow` at outer layers.
- Naming: modules/files `snake_case`, types/traits `UpperCamelCase`, functions/vars `snake_case`, constants `SCREAMING_SNAKE_CASE`. Keep exchange-specific logic in their modules and load credentials from env/config only.
- Match existing CLI table/log style when using `colored` and `prettytable`.

## Testing Guidelines
- Place concise unit tests near the code; use `#[tokio::test]` for async paths and cover retry/backoff, signing, and parsing branches. Avoid live API calls; stub network boundaries or gate integration checks behind feature flags.
- For serialization/signing updates, extend `examples/test_field_orderings.rs` or add deterministic vectors. Use descriptive names (`fn parses_pacifica_positions()` style).

## Commit & Pull Request Guidelines
- Use `<type>: <imperative summary>` (`feat|fix|chore|docs|refactor|security`, e.g., `feat: add pacifica funding cache`). Keep scopes small and subjects under ~72 chars.
- PRs should state what/why, env/config deltas, and test evidence (`cargo test`, `cargo fmt -- --check`, relevant example runs). Add screenshots/log snippets for CLI/table changes, link issues, and flag operational risks (keys, network calls, migrations).

## Security & Configuration Tips
- Never commit secrets (`.env`, API keys, private keys); use `.env.example` as the template. Sanity-check `config.json` risk limits before merging.
- `bot_state.json` holds live position state—treat as sensitive runtime data, keep Docker mounts local, and rotate keys if any logs expose credential-like data.
