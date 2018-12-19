RUST_BACKTRACE:=1
RENDY_FEATURES:=

ifeq ($(OS),Windows_NT)
	RENDY_FEATURES=dx12
else
	UNAME_S:=$(shell uname -s)
	ifeq ($(UNAME_S),Linux)
		RENDY_FEATURES=vulkan
	endif
	ifeq ($(UNAME_S),Darwin)
		RENDY_FEATURES=metal
	endif
endif

build:
	cd rendy && cargo build --all --examples --features $(RENDY_FEATURES)

test:
	cd rendy && cargo test --all --features $(RENDY_FEATURES)

doc:
	cd rendy && cargo doc --all --features $(RENDY_FEATURES)

all: build test doc

quads:
	cd rendy && cargo run --features $(RENDY_FEATURES) --example quads

triangle:
	cd rendy && cargo run --features $(RENDY_FEATURES) --example triangle

