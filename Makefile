RUST_BACKTRACE:=1
RENDY_BACKEND:=

ifeq ($(OS),Windows_NT)
	RENDY_BACKEND=dx12
else
	UNAME_S:=$(shell uname -s)
	ifeq ($(UNAME_S),Linux)
		RENDY_BACKEND=vulkan
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

publish-chain:
	cd chain && cargo publish

publish-command: publish-util
	cd command && cargo publish

publish-descriptor:
	cd descriptor && cargo publish

publish-factory: publish-command publish-descriptor publish-memory publish-resource publish-util publish-wsi
	cd factory && cargo publish

publish-frame: publish-command publish-factory publish-memory publish-resource publish-util
	cd frame && cargo publish

publish-graph: publish-chain publish-command publish-descriptor publish-factory publish-frame publish-memory publish-resource publish-shader publish-util publish-wsi
	cd graph && cargo publish

publish-memory:
	cd memory && cargo publish

publish-mesh: publish-command publish-memory publish-resource publish-factory publish-util
	cd mesh && cargo publish

publish-resource: publish-descriptor publish-memory publish-util
	cd resource && cargo publish

publish-shader: publish-factory publish-util
	cd shader && cargo publish

publish-texture: publish-memory publish-resource publish-factory publish-util
	cd texture && cargo publish

publish-util:
	cd util && cargo publish

publish-wsi: publish-memory publish-resource publish-util
	cd wsi && cargo publish

publish-rendy: publish-chain publish-command publish-descriptor publish-factory publish-frame publish-graph publish-memory publish-mesh publish-resource publish-shader publish-texture publish-util publish-wsi
	