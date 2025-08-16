#!/bin/bash

# Build script for Orasi UI
set -e

echo "Building Orasi UI..."

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "Installing wasm-pack..."
    cargo install wasm-pack
fi

# Build the project
echo "Building with wasm-pack..."
wasm-pack build --target web

echo "Build complete! Files are in the pkg/ directory."
echo "To serve the files, run: python3 -m http.server 8080"
echo "Then open http://localhost:8080 in your browser."
