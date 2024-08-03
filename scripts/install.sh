#!/bin/bash

set -e

KC_HOME=${KC_HOME:-/home/kachiclash}
PUBLIC=$KC_HOME/public
SERVER=$KC_HOME/server
SERVICE=kachiclash

case $1 in
--beta)
    PUBLIC=$KC_HOME/public-beta
    SERVER=$KC_HOME/server-beta
    SERVICE=kachiclash-beta
    ;;
'') ;;
*)
    echo "Unknown option: $1"
    exit 3
    ;;
esac

cargo build --bin=server --release --locked || exit

sudo rsync -rv public/ $PUBLIC
sudo chown -R kachiclash:nogroup $PUBLIC
sudo chmod 0555 $PUBLIC
# sudo chmod 0555 /storage/kachiclash.com/public/{css,img,js}
sudo install -vb \
    -o kachiclash -g nogroup -m 0555 \
    target/release/server \
    $SERVER

sudo systemctl restart $SERVICE
