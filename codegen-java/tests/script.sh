#!/bin/bash
set -e

INPUT_PATH="$1"
OUTPUT_PATH="$2"

# Build dependencies
mvn clean package -DskipTests -B -q
mvn dependency:copy-dependencies -DoutputDirectory=target/lib -DincludeScope=runtime -B -q

# Run
java -jar target/jsoncodegen.jar < "$INPUT_PATH" > "$OUTPUT_PATH"
