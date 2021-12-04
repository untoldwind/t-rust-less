#!/bin/sh

cargo install --path daemon --force --locked
cargo install --path cli --force --locked
cargo install --path native --force --locked

cp conf/*.target $HOME/.config/systemd/user
cp conf/*.service $HOME/.config/systemd/user

systemctl --user daemon-reload

systemctl --user enable t-rust-less.service

