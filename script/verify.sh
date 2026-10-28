#!/bin/bash
# Local pre-push verifier for the Warp fork
# Runs the essential checks that CI enforces.
# Expected wall time: ~30-60 seconds on a warm cache.

set -e

echo "Running cargo fmt --check..."
cargo fmt --check

echo "Running cargo check --workspace..."
cargo check --workspace

echo "Running cargo clippy -p warp..."
cargo clippy -p warp -- -D warnings

echo "Running cargo test -p warp..."
cargo test -p warp

echo "Running script/check_orphan_modules.sh..."
bash script/check_orphan_modules.sh

echo "Running script/check_empty_files.sh..."
bash script/check_empty_files.sh

echo "All verification checks passed!"
