#!/bin/sh

cargo build --bin=server --release

sudo rsync -rv public /storage/kachiclash.com/
sudo chown -R kachiclash:kachiclash /storage/kachiclash.com/public
sudo chmod 0555 /storage/kachiclash.com/public{,css,img,js}
sudo install -v \
    -o kachiclash -g kachiclash -m 0555 \
    target/release/server \
    /storage/kachiclash.com

sudo systemctl restart kachiclash
