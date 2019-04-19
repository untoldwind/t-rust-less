#!/bin/sh

mkdir .cargo_tmp

cat > .cargo_tmp/config <<END 
[target.x86_64-pc-windows-gnu]
linker = "/usr/bin/x86_64-w64-mingw32-gcc"
ar = "/usr/x86_64-w64-mingw32/bin/ar"
END

docker build -t t-rust-less-builder builder
docker run --rm -t -u $(id -u):$(id -g) -v $(pwd):/project -e CARGO_HOME=/project/.cargo_tmp --workdir /project t-rust-less-builder cargo test --release
docker run --rm -t -u $(id -u):$(id -g) -v $(pwd):/project -e CARGO_HOME=/project/.cargo_tmp --workdir /project t-rust-less-builder cargo build --release
docker run --rm -t -u $(id -u):$(id -g) -v $(pwd):/project -e CARGO_HOME=/project/.cargo_tmp --workdir /project/cli t-rust-less-builder cargo build --release --target x86_64-pc-windows-gnu --features crossterm_backend --no-default-features