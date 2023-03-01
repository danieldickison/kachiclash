#!/bin/sh

cargo build --bin=server --release || exit

sudo rsync -rv public /home/kachiclash/
sudo chown -R kachiclash:nogroup /home/kachiclash/public
sudo chmod 0555 /home/kachiclash/public
# sudo chmod 0555 /storage/kachiclash.com/public/{css,img,js}
sudo install -vb \
    -o kachiclash -g nogroup -m 0555 \
    target/release/server \
    /home/kachiclash

sudo systemctl restart kachiclash
sudo systemctl restart kachiclash-levelone
