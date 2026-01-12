#!/bin/bash
cargo build --target wasm32-unknown-unknown --release && python3 -m http.server 8080
