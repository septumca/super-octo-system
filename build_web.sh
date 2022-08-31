#!/bin/bash

echo "Building and copying wasm"
cargo build --target wasm32-unknown-unknown --release
git checkout web
cp target/wasm32-unknown-unknown/release/solsys.wasm .
echo "Deploying"
git commit --amend -m "web"
git push -u origin web --force
git checkout master