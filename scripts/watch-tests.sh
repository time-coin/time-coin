#!/bin/bash

echo "👀 Watching for changes and running tests..."
cargo watch -x "test --workspace"
