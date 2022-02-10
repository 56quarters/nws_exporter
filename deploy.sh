#!/bin/sh

set -o xtrace
set -o errexit
set -o nounset

HOST="$1"

cargo build --release --target x86_64-unknown-linux-musl
scp target/x86_64-unknown-linux-musl/release/nws_exporter "$HOST":
scp ext/nws_exporter.service "$HOST":
