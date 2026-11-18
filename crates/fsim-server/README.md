# `fsim-server`

Game and MCTS server hosting user authentication (`data/users.sqlite`), game actors, WebSocket streaming, checkpoint/rewind management, and disk persistence (`data/games/<game-id>/`).

- **State & Caching**: Stateful game actors (`AppState`) + SQLite account database (`users.sqlite`)
- **Dependencies**: Workspace: `fsim-core`, `fsim-protocols`, `fsim-net`, `fsim-gateway`, `fsim-plant-host`, and all 12 team crates | External: `axum`, `tokio`, `ring`, `zstd`, `uuid`, `serde`, `serde_json`
- **Data Flow**: Consumes HTTP/WebSocket commands and auth requests -> Produces `ClientGameSnapshot` diffs, `.log.zst` segments, and `.snap.zst` snapshots

## Design Intent & Assumptions
- **Encapsulation Boundary**: Owns the imperative HTTP/WebSocket shell, invite-only account authentication, and atomic file persistence around the pure simulation core.
- **Core Invariants**:
  - **Nothing Reachable Without Sign-In**: Every catalog and game endpoint requires a valid session token.
  - **Invite-Only Onboarding**: Accounts can only be created via single-use admin-generated invite links; password resets use single-use admin-generated reset links.
  - **Atomic Disk Writes**: Snapshots and metadata are written to `<file>.tmp`, flushed with `sync_all()`, and atomically renamed.

## Algorithms Implemented (Do Not Duplicate)
- **PBKDF2-HMAC-SHA256 Password Hashing & Atomic File Persistence** (`reference::reference_hash_password`, `reference::atomic_write_file`): Hashes credentials with 100,000 iterations of `ring::pbkdf2` and performs crash-safe atomic file renames.

## Public API & Reference Oracle
- **[`src/lib.rs`](src/lib.rs)**: `AppState`, `GameInstance`, `ClientGameSnapshot`, `StaticCatalogBundle` (includes `preview_models: catalog_id -> CellModel` for planning ghosts). `ClientGameSnapshot` omits gateway log entries (only `gateway_log_len`) so frequent play-mode steps stay small; `/step` accepts horizons down to 1 ms, `build_router()`. Serves the built web bundle from `web/dist` (`FSIM_WEB_DIST`).
- **[`src/sqlite.rs`](src/sqlite.rs)**: `SqliteConn` FFI wrapper for `libsqlite3`.
- **[`src/reference.rs`](src/reference.rs)**: Password hashing and atomic write oracle (`reference_hash_password`, `reference_verify_password`, `atomic_write_file`).
- **[`tests/headless_bot.rs`](tests/headless_bot.rs)**: Headless CI bot exercising invite-link registration, password reset, tutorial play, backward-chaining orders, checkpoint rewind, and problem reporting.
