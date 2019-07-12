RUST_BACKTRACE:=1

ifndef RENDY_BACKEND
	RENDY_BACKEND=""
endif

ifeq ($(RENDY_BACKEND),"")
	ifeq ($(OS),Windows_NT)
		RENDY_BACKEND=vulkan
	else
		UNAME_S:=$(shell uname -s)
		ifeq ($(UNAME_S),Linux)
			RENDY_BACKEND=vulkan
		endif
		ifeq ($(UNAME_S),Darwin)
			RENDY_BACKEND=metal
		endif
	endif
endif

WASM_BUILT:=target/wasm32-unknown-unknown/debug
WASM_GEN:=examples/www/generated

build:
	cd rendy && cargo build --all --features "full $(RENDY_BACKEND)"

test:
	cd rendy && cargo test --all --features "full $(RENDY_BACKEND)"

doc:
	cd rendy && cargo doc --all --features "full $(RENDY_BACKEND)"

quads:
	cd examples && cargo run --bin quads --features "$(RENDY_BACKEND) mesh texture-image wsi-winit"

triangle:
	cd examples && cargo run --bin triangle --features "$(RENDY_BACKEND) mesh texture-image wsi-winit"

sprite:
	cd examples && cargo run --bin sprite --features "$(RENDY_BACKEND) mesh texture-image wsi-winit"

meshes:
	cd examples && cargo run --bin meshes --features "$(RENDY_BACKEND) mesh texture-image wsi-winit"

$(WASM_BUILT)/triangle.wasm:
	cd examples && cargo build --bin triangle --features "gl mesh texture-image" --target=wasm32-unknown-unknown

$(WASM_GEN)/triangle_bg.wasm: $(WASM_BUILT)/triangle.wasm
	wasm-bindgen --web $(WASM_BUILT)/triangle.wasm --out-dir $(WASM_GEN)

$(WASM_GEN)/triangle.wasm: $(WASM_GEN)/triangle_bg.wasm
	wasm-opt $(WASM_GEN)/triangle_bg.wasm -o $(WASM_GEN)/triangle.wasm

$(WASM_BUILT)/sprite.wasm:
	cd examples && cargo build --bin sprite --features "gl mesh texture-image" --target=wasm32-unknown-unknown

$(WASM_GEN)/sprite_bg.wasm: $(WASM_BUILT)/sprite.wasm
	wasm-bindgen --web $(WASM_BUILT)/sprite.wasm --out-dir $(WASM_GEN)

$(WASM_GEN)/sprite.wasm: $(WASM_GEN)/sprite_bg.wasm
	wasm-opt $(WASM_GEN)/sprite_bg.wasm -o $(WASM_GEN)/sprite.wasm

$(WASM_BUILT)/meshes.wasm:
	cd examples && cargo build --bin meshes --features "gl mesh texture-image" --target=wasm32-unknown-unknown

$(WASM_GEN)/meshes_bg.wasm: $(WASM_BUILT)/meshes.wasm
	wasm-bindgen --web $(WASM_BUILT)/meshes.wasm --out-dir $(WASM_GEN)

$(WASM_GEN)/meshes.wasm: $(WASM_GEN)/meshes_bg.wasm
	wasm-opt $(WASM_GEN)/meshes_bg.wasm -o $(WASM_GEN)/meshes.wasm

web-suit: $(WASM_GEN)/triangle.wasm $(WASM_GEN)/sprite.wasm $(WASM_GEN)/meshes.wasm

all: build test doc web-suit
examples: triangle sprite meshes quads web-suit
