#!/bin/bash
# Find the line with "use axum::" and add imports before it
sed -i '/^use axum::/i use axum::response::IntoResponse;\nuse serde_json::json;' api/src/handlers.rs
echo "✓ Imports added correctly"
head -15 api/src/handlers.rs
