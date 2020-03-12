#!/bin/bash

echo "Building $1"

rm -rf target/generated-wasm || { echo 'Unknown failure' ; exit 1; }

mkdir -p target/generated-wasm/$1/sprite || { echo 'Unknown failure' ; exit 1; }
mkdir -p target/generated-wasm/$1/sprite || { echo 'Unknown failure' ; exit 1; }

if [ "$1" == "release" ]; then
  RELEASE="--release"
else
  RELEASE=""
fi

# Sprite
cargo build $RELEASE --manifest-path examples/Cargo.toml --bin sprite --target wasm32-unknown-unknown --features "base, init-winit, texture-image, gl" || { echo 'cargo build targeting wasm32 failed' ; exit 1; }
wasm-bindgen target/wasm32-unknown-unknown/$1/sprite.wasm --out-dir target/generated-wasm/$1/sprite --web || { echo 'wasm-bindgen failed' ; exit 1; }
cp examples/src/sprite/index.html target/generated-wasm/$1/sprite/index.html || { echo 'Unknown failure' ; exit 1; }

# Meshes
cargo build $RELEASE --manifest-path examples/Cargo.toml --bin meshes --target wasm32-unknown-unknown --features "base, init-winit, texture-image, gl" || { echo 'cargo build targeting wasm32 failed' ; exit 1; }
wasm-bindgen target/wasm32-unknown-unknown/$1/meshes.wasm --out-dir target/generated-wasm/$1/meshes --web || { echo 'wasm-bindgen failed' ; exit 1; }
cp examples/src/meshes/index.html target/generated-wasm/$1/meshes/index.html || { echo 'Unknown failure' ; exit 1; }

exit 0