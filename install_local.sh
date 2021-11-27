#systemctl --user stop t-rust-less

#cp conf/t-rust-less.service $HOME/.config/systemd/user

#systemctl --user daemon-reload

cargo install --path cli --force
cargo install --path daemon --force
cargo install --path native --force

#systemctl --user start t-rust-less
