#!/bin/sh

systemctl --user stop t-rust-less.service 
cargo install --path daemon --force --locked
cargo install --path cli --force --locked
cargo install --path native --force --locked
systemctl --user start t-rust-less.service 
