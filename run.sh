#!/bin/zsh

if [[ -f data.json ]]; then
    rm data.json
fi
echo '{"mode": "loading"}' > data.json

set -e
rust -R
set +e

INFOBEAMER_INFO_INTERVAL=86400 info-beamer . &

target/release/info-beamer-quantum-werewolf || {
    exit_code=$?
    echo '{"mode": "error"}' > data.json
    bun &> /dev/null; sudo killall info-beamer
    exit $exit_code
}
