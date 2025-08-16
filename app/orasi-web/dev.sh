#!/bin/bash

# Development script for Orasi UI with Trunk
set -e

echo "Starting Orasi UI development server..."

# Check if trunk is installed
if ! command -v trunk &> /dev/null; then
    echo "Installing trunk..."
    cargo install trunk
fi

# Start the development server
echo "Starting development server with hot reloading..."
trunk serve
