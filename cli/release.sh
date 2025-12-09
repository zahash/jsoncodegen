#!/usr/bin/env bash
set -e

# Check if target argument is provided
if [ -z "$1" ]; then
    echo "Usage: $0 <target-triple>"
    echo ""
    echo "Example target triples:"
    echo "  Linux:   x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu, i686-unknown-linux-gnu"
    echo "  Windows: x86_64-pc-windows-msvc, i686-pc-windows-msvc, aarch64-pc-windows-msvc"
    echo "  macOS:   aarch64-apple-darwin, x86_64-apple-darwin"
    exit 1
fi

TARGET=$1

# Ensure cross is installed
if ! command -v cross &> /dev/null; then
    cargo install cross
fi

# Add the target
rustup target add "$TARGET"

# Build with cross
cross build --package jcg --profile cli --target "$TARGET"

echo "Build complete! Binary location: target/$TARGET/cli/jcg"
