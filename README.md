# t-rust-less

Spritual ancestor of https://github.com/untoldwind/trustless taking over the concepts that worked, improving those
that did not turned out so well and avoiding all the quirky stuff (potentially by replacing it with new quirky stuff).

Some major changes:
* `t-rust-less` is a ground up rewrite in rust (as the name suggests)
* ... which allows a much better control over memory and protecting/cleaning up sensitive data.
* For the most part `trustless` tried to be compatible with `gpg`, `t-rust-less` drops this idea entirely
  in favor of adding more modern ciphers and key-derivations to the mix.
* ... which essentially means that a `trustless` store will not be compatible with a `t-rust-less` whatsoever.
  Sorry, but an `export` -> `import` will be required.

## Cross-Compile

### To windows (library only atm)

#### Prepare

On archlinux one needs AUR `mingw-w64-gcc` (or `mingw-w64-gcc-bin`).

Add `~/.cargo/config`:
```
[target.x86_64-pc-windows-gnu]
linker = "/usr/bin/x86_64-w64-mingw32-gcc"
ar = "/usr/x86_64-w64-mingw32/bin/ar"
```

#### Building

```
cargo build --release --target=x86_64-pc-windows-gnu
```

### To wasm (library only)


#### Prepare

Requires emscripten.

#### Building

```
cargo build --release --target=wasm32-unknown-emscripten
```

## Tests

Some tests are pretty slow and will be ignored during a regular development cycle with
`cargo test`. To run the full suit for regression:

```
cargo test --release
```
