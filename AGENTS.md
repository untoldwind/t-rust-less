# AGENTS.md

Rust password-manager workspace. Five crates, Cap'n Proto codegen, no JS tooling.

## Workspace layout

| Directory | Crate | Role |
|-----------|-------|------|
| `lib/` | `t-rust-less-lib` | Core library — the only crate published to crates.io |
| `cli/` | `t-rust-less` | Cursive TUI + clap CLI binary |
| `daemon/` | `t-rust-less-daemon` | Tokio async background service |
| `native/` | `t-rust-less-native` | Native messaging host for browser extensions |
| `experiments/` | `t-rust-less-experiments` | Scratch / experimental (edition 2018, others are 2021) |

## Dev commands

```sh
cargo fmt          # format (apply)
cargo fmt -- --check           # what CI runs — fails if unclean
cargo clippy -- -Dwarnings     # what CI runs — any warning = failure
cargo build
cargo test                     # skips slow tests (see below)
cargo test --release           # full suite — required before merging
cargo check                    # typecheck only, fastest feedback
```

Run a focused subset:
```sh
cargo test -p t-rust-less-lib cipher        # substring match within a crate
cargo test -p t-rust-less-lib <test_name>
```

## Critical: slow tests are silently skipped in debug mode

Several tests use `#[cfg_attr(debug_assertions, ignore)]` — **not** `#[ignore]`. They run only when debug assertions are off (i.e. `--release`). A plain `cargo test` silently omits them; there is no `--ignored` flag that helps. Always run `cargo test --release` for the full suite.

## Formatting quirk: 2-space indentation

`rustfmt.toml` sets `tab_spaces = 2` (Rust default is 4). Do not reformat with default rustfmt settings — CI will reject it. Other non-default settings: `max_width = 120`, `use_try_shorthand = true`.

## System dependency on Linux

Default features include `with_x11`, which requires `libx11-dev`:
```sh
sudo apt-get install libx11-dev   # Debian/Ubuntu
```
CI installs this before building. On a fresh machine, build will fail without it.

## Cap'n Proto codegen

`lib/src/secrets_store_capnp.rs` is generated from `lib/src/secrets_store.capnp` via `lib/build.rs`. The generated file **is committed** — do not hand-edit it. Regeneration is timestamp-gated; it only runs when the `.capnp` source is newer. If you edit `secrets_store.capnp`, you need `capnproto` installed (`capnpc` binary) and then `cargo build` will regenerate it.

## `lib/` feature flags

- `with_x11`, `with_wayland` — clipboard backends (x11 requires system lib)
- `rust_crypto` — pure-Rust RSA+AES-GCM (enabled by default, tested in CI)
- `openssl` — optional OpenSSL cipher (not default; its tests are feature-gated)
- `dropbox`, `with_sled`, `with_specta` — optional backends

For Windows cross-compile: `--features crossterm_backend --no-default-features` (CLI only).

## CI order (`.github/workflows/build.yml`)

1. `cargo fmt -- --check`
2. `cargo clippy -- -Dwarnings`
3. `cargo build --release`
4. `cargo test --release`

All four must pass. Fmt and clippy both block merge. Run them locally in this order before pushing.

## Local install / service

```sh
./install-local.sh   # installs binaries + sets up systemd user service
./update-local.sh    # hot-update running service
./build-release.sh   # cross-compile via Docker (see builder/)
```

## Testing notes

- No external services or databases required for tests — block store tests use in-memory or `tempfile` tmpdir backends.
- Assertions use the `spectral` crate (`assert_that(...).is_equal_to(...)`).
- Property-based tests use `quickcheck` in a few files.
- Cipher fixture tests embed large hex blobs directly in source (`lib/src/secrets_store/cipher/fixture_tests.rs`) — do not generate separate fixture files.
