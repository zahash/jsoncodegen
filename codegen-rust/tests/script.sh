#!/bin/bash
set -e

INPUT_PATH="$1"
OUTPUT_PATH="$2"

/usr/local/cargo/bin/cargo run < "$INPUT_PATH" > "$OUTPUT_PATH"
