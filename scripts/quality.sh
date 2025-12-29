#!/bin/bash
set -euox pipefail

cargo fmt --all -- --check
cargo clippy --all-features -- -D warnings