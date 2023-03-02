#!/bin/sh

cargo build --bin=server --release || exit

sudo rsync -rv public/ /home/kachiclash/public-beta
sudo chown -R kachiclash:nogroup /home/kachiclash/public-beta
sudo chmod 0555 /home/kachiclash/public-beta
# sudo chmod 0555 /home/kachiclash/public-beta/{css,img,img2,js}
sudo install -vb \
    -o kachiclash -g kachiclash -m 0555 \
    target/release/server \
    /home/kachiclash/server-beta

sudo systemctl restart kachiclash-beta
