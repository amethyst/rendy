@echo off
cd examples

echo Build examples for native platform
cargo build -p rendy-examples --features gl,mesh,texture-image,wsi-winit --bins || goto :builderror

echo Build examples for web
cargo build -p rendy-examples --features gl,mesh,texture-image --bins --target=wasm32-unknown-unknown || goto :builderror

echo Build successful

echo Generate bindings
wasm-bindgen --web --out-dir www/generated ../target/wasm32-unknown-unknown/debug/triangle.wasm || goto :bindgenerror
wasm-bindgen --web --out-dir www/generated ../target/wasm32-unknown-unknown/debug/sprite.wasm || goto :bindgenerror
wasm-bindgen --web --out-dir www/generated ../target/wasm32-unknown-unknown/debug/meshes.wasm || goto :bindgenerror
wasm-bindgen --web --out-dir www/generated ../target/wasm32-unknown-unknown/debug/quads.wasm || goto :bindgenerror

echo Optimize wasm
wasm-opt www/generated/triangle_bg.wasm -O -o www/generated/triangle.wasm || goto :opterror
wasm-opt www/generated/sprite_bg.wasm -O -o www/generated/sprite.wasm || goto :opterror
wasm-opt www/generated/meshes_bg.wasm -O -o www/generated/meshes.wasm || goto :opterror
wasm-opt www/generated/quads_bg.wasm -O -o www/generated/quads.wasm || goto :opterror

echo Run examples
cargo run --features gl,mesh,texture-image,wsi-winit --bin triangle || goto :runerror
cargo run --features gl,mesh,texture-image,wsi-winit --bin sprite || goto :runerror
cargo run --features gl,mesh,texture-image,wsi-winit --bin meshes || goto :runerror
cargo run --features gl,mesh,texture-image,wsi-winit --bin quads || goto :runerror

echo Open in default browser
python -m webbrowser http://localhost:8000 || goto :browsererror
python -m http.server 8000 --directory www || goto :servererror

cd ..

:builderror
echo Build failed
cd ..
exit /b %errorlevel%

:runerror
echo Example execution failed
cd ..
exit /b %errorlevel%

:bindgenerror
echo Wasm binding generation failed
cd ..
exit /b %errorlevel%

:opterror
echo Wasm optimization failed
cd ..
exit /b %errorlevel%

:browsererror
echo Failed to open browser
cd ..
exit /b %errorlevel%

:servererror
echo Failed to start http server
cd ..
exit /b %errorlevel%
