# TuxCommand

TuxCommand is a command and control (C2) framework meant to be easy to operate and set up, and intended to avoid detection. Note: It's not finished, don't use it.

Dependencies debian:
```
sudo apt install cargo rustc gcc musl-tools musl-dev libsqlite3-dev pkg-config make
```
There is also a nix flake for nixOS
```
cd server/
cargo build
sudo target/debug/server
```

