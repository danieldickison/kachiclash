#!/bin/sh

cargo build --bin=server --release || exit

sudo rsync -rv public/ /storage/kachiclash.com/public-beta
sudo chown -R kachiclash:kachiclash /storage/kachiclash.com/public-beta
sudo chmod 0555 /storage/kachiclash.com/public-beta
sudo chmod 0555 /storage/kachiclash.com/public-beta/{css,img,img2,js}
sudo install -vb \
    -o kachiclash -g kachiclash -m 0555 \
    target/release/server \
    /storage/kachiclash.com/server-beta

sudo systemctl restart kachiclash-beta
