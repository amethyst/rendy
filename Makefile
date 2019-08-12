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
	cd rendy && cargo build --all --features full

test:
	cd rendy && cargo test --all --features full

doc:
	cd rendy && cargo doc --all --features full

quads:
	cd examples && cargo run --bin quads

triangle:
	cd examples && cargo run --bin triangle

sprite:
	cd examples && cargo run --bin sprite

meshes:
	cd examples && cargo run --bin meshes

web-triangle-build $(WASM_BUILT)/triangle.wasm:
	cd examples && cargo build --bin triangle --target=wasm32-unknown-unknown

$(WASM_GEN)/triangle_bg.wasm: $(WASM_BUILT)/triangle.wasm
	wasm-bindgen --target=web $(WASM_BUILT)/triangle.wasm --out-dir $(WASM_GEN)

$(WASM_GEN)/triangle.wasm: $(WASM_GEN)/triangle_bg.wasm
	wasm-opt $(WASM_GEN)/triangle_bg.wasm -o $(WASM_GEN)/triangle.wasm

web-sprite-build $(WASM_BUILT)/sprite.wasm:
	cd examples && cargo build --bin sprite --target=wasm32-unknown-unknown

$(WASM_GEN)/sprite_bg.wasm: $(WASM_BUILT)/sprite.wasm
	wasm-bindgen --target=web $(WASM_BUILT)/sprite.wasm --out-dir $(WASM_GEN)

$(WASM_GEN)/sprite.wasm: $(WASM_GEN)/sprite_bg.wasm
	wasm-opt $(WASM_GEN)/sprite_bg.wasm -o $(WASM_GEN)/sprite.wasm

web-meshes-build $(WASM_BUILT)/meshes.wasm:
	cd examples && cargo build --bin meshes --target=wasm32-unknown-unknown

$(WASM_GEN)/meshes_bg.wasm: $(WASM_BUILT)/meshes.wasm
	wasm-bindgen --target=web $(WASM_BUILT)/meshes.wasm --out-dir $(WASM_GEN)

$(WASM_GEN)/meshes.wasm: $(WASM_GEN)/meshes_bg.wasm
	wasm-opt $(WASM_GEN)/meshes_bg.wasm -o $(WASM_GEN)/meshes.wasm

web-suit: web-triangle-build web-sprite-build web-meshes-build $(WASM_GEN)/triangle.wasm $(WASM_GEN)/sprite.wasm $(WASM_GEN)/meshes.wasm

all: build test doc web-suit
examples: triangle sprite meshes quads web-suit
