#!/bin/bash

set -e

KC_HOME=${KC_HOME:-/home/kachiclash}
PUBLIC=$KC_HOME/public
SERVER=$KC_HOME/server
SERVICE=kachiclash

while [[ $# -gt 0 ]]; do
    case $1 in
        --beta)
            shift
            PUBLIC=$KC_HOME/public-beta
            SERVER=$KC_HOME/server-beta
            SERVICE=kachiclash-beta
            ;;
        -r|--run)
            GH_RUN_ID="$2"
            shift
            shift
            ;;
        *)
            echo "Unknown option: $1"
            exit 3
            ;;
    esac
done

if [ -n "$GH_RUN_ID" ]; then
    echo "Using artifact from GH Action run ID: $GH_RUN_ID"
    mkdir -p var
    gh run download $GH_RUN_ID --name build-output --dir var/build-output
    cd var/build-output
else
    echo "Building locally"
    cargo build --bin=server --release --locked
fi

sudo rsync -rv --checksum public/ $PUBLIC
sudo chown -R kachiclash:nogroup $PUBLIC
sudo chmod 0555 $PUBLIC
# sudo chmod 0555 /storage/kachiclash.com/public/{css,img,js}
sudo install -vb \
    -o kachiclash -g nogroup -m 0555 \
    target/release/server \
    $SERVER

sudo systemctl restart $SERVICE

if [ -n "$GH_RUN_ID" ]; then
    cd ..
    rm -rf build-output
fi
