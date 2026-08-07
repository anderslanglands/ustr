#!/usr/bin/env sh

set -ex

export CARGO_NET_RETRY=5
export CARGO_NET_TIMEOUT=10

MIRI_NIGHTLY=nightly-$(curl -s https://rust-lang.github.io/rustup-components-history/x86_64-unknown-linux-gnu/miri)
echo "Installing latest nightly with Miri: $MIRI_NIGHTLY"
rustup toolchain install "$MIRI_NIGHTLY" --component miri

rustup run "$MIRI_NIGHTLY" cargo miri setup

export RUST_TEST_THREADS=1
rustup run "$MIRI_NIGHTLY" cargo miri test --features=serde
