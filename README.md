# Cross-Compile

## Prepare

On archlinux one needs AUR `mingw-w64-gcc` (or `mingw-w64-gcc-bin`).

Add `~/.cargo/config`:
```
[target.x86_64-pc-windows-gnu]
linker = "/usr/bin/x86_64-w64-mingw32-gcc"
ar = "/usr/x86_64-w64-mingw32/bin/ar"
```

## Building

```
cargo build --target=x86_64-pc-windows-gnu
```

## Test

`cargo test` works but is pretty slow as there is no optimization.

For development:

```
cargo test --features fast_tests 
```

Regression:

```
cargo test --release
```
