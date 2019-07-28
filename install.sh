#!/bin/sh

cargo build --bin=server --release

sudo install \
    -d -o kachiclash -g kachiclash -m0555 \
    target/release/server public \
    /storage/kachiclash.com

sudo systemctl restart kachiclash
