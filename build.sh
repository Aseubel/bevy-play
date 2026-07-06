#!/bin/bash
# Exit immediately if a command exits with a non-zero status
set -e

# delete old file
rm -f web/bevy-play_bg.wasm

echo "========================================="
echo "  1. Compiling Bevy project to WASM..."
echo "========================================="
cargo build --target wasm32-unknown-unknown --release

echo "========================================="
echo "  2. Generating Web Assembly Bindings..."
echo "========================================="
wasm-bindgen --target web --out-dir web --no-typescript target/wasm32-unknown-unknown/release/bevy-play.wasm

echo "========================================="
echo "  3. Running size optimizations..."
echo "========================================="
# Enables only the stable standard features used by the Rust compiler.
# This prevents wasm-opt from failing validation while avoiding experimental 
# features (like reference types or tail calls) that crash browsers.
wasm-opt -Oz --enable-bulk-memory --enable-sign-ext --enable-nontrapping-float-to-int web/bevy-play_bg.wasm -o web/bevy-play_bg.wasm

echo "========================================="
echo "  Build successful!"
echo "========================================="
echo "To play the games, run one of the following commands:"
echo "  - python3 -m http.server 8000 --directory web"
echo "  - npx serve web"
echo "Then visit http://localhost:8000 in your browser."
