RUST_BACKTRACE:=1
RENDY_BACKEND:=

ifeq ($(OS),Windows_NT)
	RENDY_BACKEND=dx12
else
	UNAME_S:=$(shell uname -s)
	ifeq ($(UNAME_S),Linux)
		RENDY_BACKEND=vulkan-x11
	endif
	ifeq ($(UNAME_S),Darwin)
		RENDY_BACKEND=metal
	endif
endif

fast:
	cd rendy && cargo build --all --examples --features "full $(RENDY_BACKEND) no-slow-safety-checks"

build:
	cd rendy && cargo build --all --examples --features "full $(RENDY_BACKEND)"

test:
	cd rendy && cargo test --all --features "full $(RENDY_BACKEND)"

doc:
	cd rendy && cargo doc --all --features "full $(RENDY_BACKEND)"

all: fast build test doc

quads:
	cd rendy && cargo run --features "full $(RENDY_BACKEND)" --example quads

triangle:
	cd rendy && cargo run --features "full $(RENDY_BACKEND)" --example triangle

sprite:
	cd rendy && cargo run --features "full $(RENDY_BACKEND)" --example sprite

meshes:
	cd rendy && cargo run --features "full $(RENDY_BACKEND)" --example meshes
