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

if $GH_RUN_ID; then
    echo "Using artifact from GH Action run ID: $GH_RUN_ID"
    gh run download $GH_RUN_ID -n build-output -d gh-artifact || exit
    cd gh-artifact
else
    echo "Building locally"
    cargo build --bin=server --release --locked || exit
fi

sudo rsync -rv public/ $PUBLIC
sudo chown -R kachiclash:nogroup $PUBLIC
sudo chmod 0555 $PUBLIC
# sudo chmod 0555 /storage/kachiclash.com/public/{css,img,js}
sudo install -vb \
    -o kachiclash -g nogroup -m 0555 \
    target/release/server \
    $SERVER

sudo systemctl restart $SERVICE

if $GH_RUN_ID; then
    cd ..
    rm -rf gh-artifact
fi
