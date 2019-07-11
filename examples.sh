#!/bin/bash

echo "Build examples for native platform"
cargo build -p rendy-examples --features gl,mesh,texture-image,wsi-winit --bins
echo "Build examples for web"
cargo build -p rendy-examples --features gl,mesh,texture-image --bins --target=wasm32-unknown-unknown
echo "Build successful"

echo "Generate bindings"
wasm-bindgen --web --out-dir examples/www/generated ../target/wasm32-unknown-unknown/debug/triangle.wasm
wasm-bindgen --web --out-dir examples/www/generated ../target/wasm32-unknown-unknown/debug/sprite.wasm
wasm-bindgen --web --out-dir examples/www/generated ../target/wasm32-unknown-unknown/debug/meshes.wasm
wasm-bindgen --web --out-dir examples/www/generated ../target/wasm32-unknown-unknown/debug/quads.wasm

echo "Optimize wasm"
wasm-opt examples/www/generated/triangle_bg.wasm -O -o examples/www/generated/triangle.wasm
wasm-opt examples/www/generated/sprite_bg.wasm -O -o examples/www/generated/sprite.wasm
wasm-opt examples/www/generated/meshes_bg.wasm -O -o examples/www/generated/meshes.wasm
wasm-opt examples/www/generated/quads_bg.wasm -O -o examples/www/generated/quads.wasm

echo "Run examples"
cargo run --features gl,mesh,texture-image,wsi-winit --bin triangle
cargo run --features gl,mesh,texture-image,wsi-winit --bin sprite
cargo run --features gl,mesh,texture-image,wsi-winit --bin meshes
cargo run --features gl,mesh,texture-image,wsi-winit --bin quads

echo "Open in default browser"
python3 -m webbrowser http://localhost:8000
python3 -m http.server 8000 --directory examples/www
