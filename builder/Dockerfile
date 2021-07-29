FROM rust:1.54.0

RUN apt-get update && apt-get install -y apt-transport-https && rm -rf /var/lib/apt/lists/*
RUN apt-get update && apt-get install -y --no-install-recommends bash make git mingw-w64 capnproto libxcb-shape0-dev libxcb-xfixes0-dev && rm -rf /var/lib/apt/lists/*

RUN rustup component add clippy
RUN rustup target add x86_64-pc-windows-gnu
